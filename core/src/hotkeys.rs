//! Global keyboard shortcuts via Win32 `RegisterHotKey`.
//!
//! Unlike the typing hook (`input.rs`, a low-level keyboard hook that observes
//! every key while a typing-reactive effect is active), `RegisterHotKey` asks
//! the OS to notify us *only* when one exact registered chord is pressed. That
//! means no key logging, negligible cost, and the shortcuts work no matter
//! which app is focused — even while the Keyscape window is closed, since the
//! core runs in the background. The registration thread only exists while at
//! least one shortcut is actually bound.

use crate::engine::Cmd;
use crate::settings::HotkeyBinding;
use std::sync::mpsc::Sender;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, MSG, WM_HOTKEY, WM_QUIT,
};

pub struct HotkeyManager {
    tx: Sender<Cmd>,
    thread: Option<(std::thread::JoinHandle<()>, u32)>,
}

impl HotkeyManager {
    pub fn new(tx: Sender<Cmd>) -> HotkeyManager {
        HotkeyManager { tx, thread: None }
    }

    /// Re-register the given bindings from scratch. Cheap and rare (only on a
    /// settings change), so we just tear the old thread down and respawn.
    pub fn set_bindings(&mut self, bindings: Vec<(String, HotkeyBinding)>) {
        if let Some((handle, tid)) = self.thread.take() {
            unsafe {
                PostThreadMessageW(tid, WM_QUIT, 0, 0);
            }
            let _ = handle.join();
        }
        let active: Vec<(String, HotkeyBinding)> =
            bindings.into_iter().filter(|(_, b)| b.vk != 0).collect();
        if active.is_empty() {
            return;
        }

        let tx = self.tx.clone();
        let (id_tx, id_rx) = std::sync::mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("hotkeys".into())
            .spawn(move || unsafe {
                let _ = id_tx.send(GetCurrentThreadId());
                // Register each chord under an id equal to its index, so a
                // WM_HOTKEY's wParam indexes straight back into `active`.
                let mut registered: Vec<i32> = Vec::new();
                for (i, (_, b)) in active.iter().enumerate() {
                    let mut mods = MOD_NOREPEAT;
                    if b.ctrl {
                        mods |= MOD_CONTROL;
                    }
                    if b.alt {
                        mods |= MOD_ALT;
                    }
                    if b.shift {
                        mods |= MOD_SHIFT;
                    }
                    if b.win {
                        mods |= MOD_WIN;
                    }
                    // Best effort: a chord already claimed by another app just
                    // won't register (and won't fire), which is fine.
                    if RegisterHotKey(std::ptr::null_mut(), i as i32, mods, b.vk) != 0 {
                        registered.push(i as i32);
                    }
                }
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    if msg.message == WM_HOTKEY {
                        if let Some((action, _)) = active.get(msg.wParam as usize) {
                            if let Some(cmd) = cmd_for(action) {
                                let _ = tx.send(cmd);
                            }
                        }
                    }
                }
                for id in registered {
                    UnregisterHotKey(std::ptr::null_mut(), id);
                }
            })
            .expect("spawn hotkeys");
        if let Ok(tid) = id_rx.recv_timeout(std::time::Duration::from_secs(2)) {
            self.thread = Some((handle, tid));
        }
    }
}

/// Map an action id to the engine command it triggers. Keep in sync with the
/// UI's shortcut action list.
fn cmd_for(action: &str) -> Option<Cmd> {
    Some(match action {
        "toggle_lights" => Cmd::ToggleLights,
        "next_effect" => Cmd::NextEffect,
        "toggle_playlist" => Cmd::TogglePlaylist,
        "brightness_up" => Cmd::BrightnessStep(1),
        "brightness_down" => Cmd::BrightnessStep(-1),
        _ => return None,
    })
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        if let Some((handle, tid)) = self.thread.take() {
            unsafe {
                PostThreadMessageW(tid, WM_QUIT, 0, 0);
            }
            let _ = handle.join();
        }
    }
}
