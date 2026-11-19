use crate::classifier::classify_content;
use crate::db::Database;
use crate::models::ContentType;
use image::{ImageBuffer, Rgba};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::Cursor;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LRESULT, MAX_PATH, WPARAM};
use windows::Win32::System::DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
    GetMessageW, GetWindowTextW, GetWindowThreadProcessId, RegisterClassW, TranslateMessage,
    HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE, WNDCLASSW,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

struct ListenerState {
    last_hash: Option<String>,
    last_time: Option<Instant>,
}

static LISTENER_STATE: Mutex<ListenerState> = Mutex::new(ListenerState {
    last_hash: None,
    last_time: None,
});
static CLIPBOARD_PROCESSING_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn start_clipboard_listener(app_handle: AppHandle, db: Arc<Database>) {
    RUNNING.store(true, Ordering::SeqCst);

    thread::spawn(move || {
        let class_name = w!("ClipCtxClipboardListenerClass");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: class_name,
            ..Default::default()
        };

        unsafe {
            let _ = RegisterClassW(&wc);

            let hwnd = match CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                PCWSTR::null(),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Failed to create message-only window: {:?}", e);
                    return;
                }
            };

            if let Err(e) = AddClipboardFormatListener(hwnd) {
                eprintln!("Failed to add clipboard format listener: {:?}", e);
                let _ = DestroyWindow(hwnd);
                return;
            }

            println!("ClipCtx clipboard listener registered successfully.");

            let mut msg = MSG::default();
            while RUNNING.load(Ordering::SeqCst) && GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_CLIPBOARDUPDATE {
                    handle_clipboard_update(&app_handle, &db);
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = RemoveClipboardFormatListener(hwnd);
            let _ = DestroyWindow(hwnd);
        }
    });
}

enum CapturedData {
    Text {
        trimmed: String,
        content_hash: String,
    },
    Image {
        png_bytes: Vec<u8>,
        content_hash: String,
    },
}

fn handle_clipboard_update(app_handle: &AppHandle, db: &Arc<Database>) {
    // 1. Serialize clipboard update processing to avoid in-flight race conditions
    let _lock = CLIPBOARD_PROCESSING_LOCK.lock().unwrap();

    // 2. Read clipboard content with retry (text first, then image)
    let captured = match read_clipboard_with_retry(3, Duration::from_millis(25)) {
        Some(c) => c,
        None => return,
    };

    let content_hash = match &captured {
        CapturedData::Text { content_hash, .. } => content_hash.clone(),
        CapturedData::Image { content_hash, .. } => content_hash.clone(),
    };

    // 3. In-Flight Guard & Cooldown:
    // If the exact same content_hash arrives within 400ms, skip processing immediately.
    {
        let mut state = LISTENER_STATE.lock().unwrap();
        let now = Instant::now();
        if let (Some(last_h), Some(last_t)) = (&state.last_hash, state.last_time) {
            if last_h == &content_hash && now.duration_since(last_t) < Duration::from_millis(400) {
                // Ignore rapid burst duplicate event (e.g. Chrome writing multiple formats)
                return;
            }
        }
        state.last_hash = Some(content_hash.clone());
        state.last_time = Some(now);
    }

    // 4. Context extraction: foreground window metadata
    let (source_app, window_title) = get_foreground_context();

    // 5. Privacy guards
    if is_private_context(&source_app, &window_title) {
        println!("Ignoring clipboard content from private context: {} / {}", source_app, window_title);
        return;
    }

    // 6. Save and classify based on data type
    let (content, content_type) = match captured {
        CapturedData::Text { trimmed, .. } => {
            let ct = classify_content(&trimmed);
            (trimmed, ct)
        }
        CapturedData::Image { png_bytes, .. } => {
            let images_dir = app_handle
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("images");
            let _ = std::fs::create_dir_all(&images_dir);
            let image_path = images_dir.join(format!("{}.png", content_hash));
            if !image_path.exists() {
                let _ = std::fs::write(&image_path, &png_bytes);
            }
            (image_path.to_string_lossy().to_string(), ContentType::Image)
        }
    };

    // 7. Insert / Update in Database
    match db.insert_or_update(&content, &content_hash, &content_type, &source_app, &window_title) {
        Ok(clip_item) => {
            // 8. Emit real-time event to Tauri frontend
            let _ = app_handle.emit("clipboard-updated", &clip_item);
        }
        Err(e) => {
            eprintln!("Failed to insert clip: {:?}", e);
        }
    }
}

fn read_clipboard_with_retry(attempts: usize, delay: Duration) -> Option<CapturedData> {
    for _ in 0..attempts {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            // Check text first
            if let Ok(text) = clipboard.get_text() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let mut hasher = Sha256::new();
                    hasher.update(trimmed.as_bytes());
                    let hash_result = hasher.finalize();
                    let content_hash: String = hash_result.iter().map(|b| format!("{:02x}", b)).collect();
                    return Some(CapturedData::Text {
                        trimmed: trimmed.to_string(),
                        content_hash,
                    });
                }
            }

            // If no text, check image
            if let Ok(img) = clipboard.get_image() {
                if img.width > 0 && img.height > 0 && !img.bytes.is_empty() {
                    if let Some(rgba) = ImageBuffer::<Rgba<u8>, _>::from_raw(
                        img.width as u32,
                        img.height as u32,
                        img.bytes.into_owned(),
                    ) {
                        let mut png_bytes = Vec::new();
                        if rgba.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png).is_ok() {
                            let mut hasher = Sha256::new();
                            hasher.update(&png_bytes);
                            let hash_result = hasher.finalize();
                            let content_hash: String = hash_result.iter().map(|b| format!("{:02x}", b)).collect();
                            return Some(CapturedData::Image {
                                png_bytes,
                                content_hash,
                            });
                        }
                    }
                }
            }
        }
        sleep(delay);
    }
    None
}

fn get_foreground_context() -> (String, String) {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.0.is_null() {
            return ("Unknown".to_string(), "Unknown".to_string());
        }

        // Window Title
        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(fg_hwnd, &mut title_buf);
        let window_title = if len > 0 {
            String::from_utf16_lossy(&title_buf[..len as usize])
        } else {
            "Untitled Window".to_string()
        };

        // Process Executable Name
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(fg_hwnd, Some(&mut process_id));

        let source_app = if process_id > 0 {
            if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) {
                let name = get_process_name(handle);
                let _ = CloseHandle(handle);
                name
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        (source_app, window_title)
    }
}

fn get_process_name(handle: HANDLE) -> String {
    unsafe {
        let mut path_buf = [0u16; MAX_PATH as usize * 2];
        let mut size = path_buf.len() as u32;

        if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(path_buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok()
            && size > 0
        {
            let os_str = OsString::from_wide(&path_buf[..size as usize]);
            let path = Path::new(&os_str);
            if let Some(file_name) = path.file_name() {
                return file_name.to_string_lossy().to_string();
            }
        }
        "Unknown".to_string()
    }
}

fn is_private_context(source_app: &str, window_title: &str) -> bool {
    let app_lower = source_app.to_lowercase();
    let title_lower = window_title.to_lowercase();

    // Password Managers
    const PASSWORD_MANAGERS: &[&str] = &[
        "bitwarden.exe",
        "1password.exe",
        "keepass.exe",
        "keepassxc.exe",
        "lastpass.exe",
        "dashlane.exe",
        "enpass.exe",
        "roboform.exe",
        "authenticator.exe",
    ];

    for pm in PASSWORD_MANAGERS {
        if app_lower == *pm {
            return true;
        }
    }

    // Incognito / InPrivate / Private Browsing
    if title_lower.contains("incognito")
        || title_lower.contains("inprivate")
        || title_lower.contains("private browsing")
    {
        return true;
    }

    false
}
