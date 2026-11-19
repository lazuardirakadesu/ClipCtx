use crate::classifier::classify_content;
use crate::db::Database;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
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

fn handle_clipboard_update(app_handle: &AppHandle, db: &Arc<Database>) {
    // 1. Context extraction: foreground window metadata
    let (source_app, window_title) = get_foreground_context();

    // 2. Privacy guards
    if is_private_context(&source_app, &window_title) {
        println!("Ignoring clipboard content from private context: {} / {}", source_app, window_title);
        return;
    }

    // 3. Read clipboard text with short retry loop (in case copied app still locks it)
    let text = match read_clipboard_text_with_retry(3, Duration::from_millis(25)) {
        Some(t) => t,
        None => return,
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    // 4. Compute SHA-256 Hash
    let mut hasher = Sha256::new();
    hasher.update(trimmed.as_bytes());
    let hash_result = hasher.finalize();
    let content_hash: String = hash_result.iter().map(|b| format!("{:02x}", b)).collect();

    // 5. Smart Classification
    let content_type = classify_content(trimmed);

    // 6. Insert / Update in Database
    match db.insert_or_update(trimmed, &content_hash, &content_type, &source_app, &window_title) {
        Ok(clip_item) => {
            // 7. Emit real-time event to Tauri frontend
            let _ = app_handle.emit("clipboard-updated", &clip_item);
        }
        Err(e) => {
            eprintln!("Failed to insert clip: {:?}", e);
        }
    }
}

fn read_clipboard_text_with_retry(attempts: usize, delay: Duration) -> Option<String> {
    for _ in 0..attempts {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                return Some(text);
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
