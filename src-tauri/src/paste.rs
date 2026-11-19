use std::mem::size_of;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, IsWindow, SetForegroundWindow,
};

static LAST_FOREGROUND_HWND: AtomicIsize = AtomicIsize::new(0);

pub fn record_foreground_window(my_window_hwnd: Option<HWND>) {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return;
        }

        // Don't record if the foreground window is our own window
        if let Some(my_hwnd) = my_window_hwnd {
            if fg == my_hwnd {
                return;
            }
        }

        LAST_FOREGROUND_HWND.store(fg.0 as isize, Ordering::SeqCst);
    }
}

pub fn restore_focus_and_paste(content: &str) -> Result<(), String> {
    // 1. Write content to clipboard using arboard
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard.set_text(content).map_err(|e| format!("Failed to set text: {}", e))?;

    // 2. Allow time for Tauri window to hide
    sleep(Duration::from_millis(50));

    // 3. Restore focus to the saved HWND
    let raw_hwnd = LAST_FOREGROUND_HWND.load(Ordering::SeqCst);
    if raw_hwnd != 0 {
        let hwnd = HWND(raw_hwnd as *mut core::ffi::c_void);
        unsafe {
            if IsWindow(Some(hwnd)).as_bool() {
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    // 4. Brief pause before simulating keystroke
    sleep(Duration::from_millis(60));

    // 5. Send Ctrl+V input
    simulate_ctrl_v()?;

    Ok(())
}

fn simulate_ctrl_v() -> Result<(), String> {
    unsafe {
        let inputs = [
            // Ctrl down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // V down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // V up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // Ctrl up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err("Failed to send full Ctrl+V sequence".to_string());
        }
    }

    Ok(())
}
