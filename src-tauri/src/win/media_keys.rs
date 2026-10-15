//! Global media keys via `RegisterHotKey` (P3.4 / architecture §2.3).
//!
//! A dedicated thread owns a `hWnd = NULL` hotkey registration; Windows posts
//! `WM_HOTKEY` to that thread's queue even while the app is unfocused. Each
//! key is forwarded to the webview as the `media-key` Tauri event.

use tauri::Emitter;

const HOTKEY_PLAY_PAUSE: i32 = 1;
const HOTKEY_NEXT: i32 = 2;
const HOTKEY_PREV: i32 = 3;
const HOTKEY_STOP: i32 = 4;

/// Spawn the hotkey thread (Windows-only; no-op elsewhere).
pub fn spawn(app: tauri::AppHandle) {
    #[cfg(windows)]
    std::thread::spawn(move || run(app));
    #[cfg(not(windows))]
    let _ = app;
}

#[cfg(windows)]
fn run(app: tauri::AppHandle) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_NOREPEAT, VK_MEDIA_NEXT_TRACK,
        VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_HOTKEY,
    };

    let hotkeys: [(i32, u32); 4] = [
        (HOTKEY_PLAY_PAUSE, u32::from(VK_MEDIA_PLAY_PAUSE)),
        (HOTKEY_NEXT, u32::from(VK_MEDIA_NEXT_TRACK)),
        (HOTKEY_PREV, u32::from(VK_MEDIA_PREV_TRACK)),
        (HOTKEY_STOP, u32::from(VK_MEDIA_STOP)),
    ];

    // SAFETY: NULL hotkeys are thread-queue scoped; MSG is zero-initialized
    // before GetMessageW fills it; all registrations are released on exit.
    unsafe {
        let mut registered = Vec::new();
        for (id, vk) in hotkeys {
            if RegisterHotKey(std::ptr::null_mut(), id, MOD_NOREPEAT, vk) != 0 {
                registered.push(id);
            } else {
                eprintln!("[media-keys] RegisterHotKey({id}) failed — held by another app?");
            }
        }
        if registered.is_empty() {
            eprintln!("[media-keys] no hotkeys registered; global media keys disabled");
            return;
        }
        println!("[media-keys] {} global hotkeys registered", registered.len());

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_HOTKEY {
                let action = match msg.wParam as i32 {
                    HOTKEY_PLAY_PAUSE => "play-pause",
                    HOTKEY_NEXT => "next",
                    HOTKEY_PREV => "prev",
                    HOTKEY_STOP => "stop",
                    _ => continue,
                };
                let _ = app.emit("media-key", action);
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        for id in registered {
            UnregisterHotKey(std::ptr::null_mut(), id);
        }
        println!("[media-keys] hotkeys unregistered, thread exiting");
    }
}
