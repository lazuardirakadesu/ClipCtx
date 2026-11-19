mod classifier;
mod clipboard_listener;
mod db;
mod models;
mod paste;
mod tray;

use db::Database;
use models::{AppFilterItem, ClipItem};
use std::sync::Arc;
use tauri::{Manager, State, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use windows::Win32::Foundation::HWND;

struct AppState {
    db: Arc<Database>,
}

#[tauri::command]
fn get_clips(
    state: State<AppState>,
    query: Option<String>,
    filter_app: Option<String>,
    filter_type: Option<String>,
    pinned_only: Option<bool>,
    limit: Option<u32>,
) -> Result<Vec<ClipItem>, String> {
    state
        .db
        .get_clips(
            query.as_deref(),
            filter_app.as_deref(),
            filter_type.as_deref(),
            pinned_only,
            limit,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_pin_clip(state: State<AppState>, id: i64) -> Result<bool, String> {
    state.db.toggle_pin(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_clip(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_clip(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_unpinned_clips(state: State<AppState>) -> Result<usize, String> {
    state.db.clear_unpinned().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_apps_summary(state: State<AppState>) -> Result<Vec<AppFilterItem>, String> {
    state.db.get_apps_summary().map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_clip_only(state: State<AppState>, id: i64) -> Result<(), String> {
    let clip = state
        .db
        .get_clip_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Clip not found".to_string())?;

    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(&clip.content).map_err(|e| e.to_string())?;
    let _ = state.db.touch_last_used(id);
    Ok(())
}

#[tauri::command]
fn paste_clip(window: WebviewWindow, state: State<AppState>, id: i64) -> Result<(), String> {
    let clip = state
        .db
        .get_clip_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Clip not found".to_string())?;

    // 1. Hide the Tauri window immediately
    let _ = window.hide();

    // 2. Touch last used in DB
    let _ = state.db.touch_last_used(id);

    // 3. Restore focus and send Ctrl+V
    paste::restore_focus_and_paste(&clip.content)?;

    Ok(())
}

#[tauri::command]
fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn show_window(window: WebviewWindow) -> Result<(), String> {
    let my_hwnd = get_window_hwnd(&window);
    paste::record_foreground_window(my_hwnd);
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn get_window_hwnd(window: &WebviewWindow) -> Option<HWND> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = window.hwnd() {
            return Some(HWND(hwnd.0 as *mut core::ffi::c_void));
        }
    }
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            let my_hwnd = get_window_hwnd(&window);
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                paste::record_foreground_window(my_hwnd);
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 1. Initialize SQLite Database in AppLocalDataDir
            let local_data_dir = app.path().app_local_data_dir().expect("Failed to get local data dir");
            let db_path = local_data_dir.join("clipctx.db");
            println!("ClipCtx database path: {:?}", db_path);

            let db = Arc::new(Database::new(&db_path).expect("Failed to initialize database"));
            app.manage(AppState { db: Arc::clone(&db) });

            // 2. Setup System Tray
            if let Err(e) = tray::setup_system_tray(&app_handle, Arc::clone(&db)) {
                eprintln!("Warning: Failed to setup system tray: {:?}", e);
            }

            // 3. Register Global Shortcut Alt+V
            let alt_v: Shortcut = "Alt+V".parse().expect("Failed to parse Alt+V shortcut");
            if let Err(e) = app.global_shortcut().register(alt_v) {
                eprintln!("Warning: Failed to register Alt+V global shortcut: {:?}", e);
            }

            // 4. Start Win32 Zero-Polling Clipboard Listener
            clipboard_listener::start_clipboard_listener(app_handle.clone(), Arc::clone(&db));

            // 5. Apply Native Windows 11 Vibrancy (Mica / Acrylic)
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    use window_vibrancy::{apply_acrylic, apply_mica};
                    if let Err(e) = apply_mica(&window, None) {
                        println!("apply_mica failed ({:?}), falling back to acrylic...", e);
                        let _ = apply_acrylic(&window, Some((18, 18, 18, 200)));
                    }
                }

                // Auto-hide when unfocused (blur)
                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clips,
            toggle_pin_clip,
            delete_clip,
            clear_unpinned_clips,
            get_apps_summary,
            copy_clip_only,
            paste_clip,
            hide_window,
            show_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClipCtx application");
}
