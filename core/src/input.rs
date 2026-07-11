//! Low-level keyboard hook feeding typing-reactive effects.
//!
//! Privacy: only *key positions* (scan codes mapped to LED indices) are
//! observed, only while a typing-reactive effect is active, and nothing ever
//! leaves the engine channel. The hook thread is started/stopped by the
//! engine's `set_input_capture` lifecycle hook so it never lingers.

use crate::effects::Tap;
use crate::engine::Cmd;
use crate::frame::LedMask;
use crate::layout::Layout;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

struct HookShared {
    tx: Sender<Cmd>,
    layout: Arc<Layout>,
    held: LedMask,
}

static SHARED: OnceLock<Mutex<Option<HookShared>>> = OnceLock::new();

fn shared() -> &'static Mutex<Option<HookShared>> {
    SHARED.get_or_init(|| Mutex::new(None))
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        // PS/2 set-1 scan code; extended keys map to 0x80 | code, matching
        // the codes in ASUS's layout CSV (arrows 0xC8.., RCtrl 0x9D, ...).
        let mut scan = (kb.scanCode & 0x7F) as u16;
        if kb.flags & LLKHF_EXTENDED != 0 {
            scan |= 0x80;
        }
        let down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
        let up = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;

        if let Ok(mut guard) = shared().lock() {
            if let Some(sh) = guard.as_mut() {
                if let Some(&ki) = sh.layout.key_of_scan.get(&scan) {
                    let k = &sh.layout.keys[ki];
                    if down && !sh.held.get(k.led) {
                        // key-repeat suppressed via the held set
                        sh.held.set(k.led);
                        let tap = Tap { key: ki, led: k.led, cx: k.cx, cy: k.cy, t: 0.0 };
                        let _ = sh.tx.send(Cmd::Input { taps: vec![tap], held: sh.held });
                    } else if up && sh.held.get(k.led) {
                        sh.held.clear(k.led);
                        let _ = sh.tx.send(Cmd::Input { taps: vec![], held: sh.held });
                    }
                }
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

pub struct InputCapture {
    tx: Sender<Cmd>,
    layout: Arc<Layout>,
    thread: Option<(std::thread::JoinHandle<()>, u32)>,
}

impl InputCapture {
    pub fn new(tx: Sender<Cmd>, layout: Arc<Layout>) -> InputCapture {
        InputCapture { tx, layout, thread: None }
    }

    pub fn set_capture(&mut self, on: bool) {
        if on && self.thread.is_none() {
            *shared().lock().unwrap() =
                Some(HookShared { tx: self.tx.clone(), layout: self.layout.clone(), held: LedMask::none() });
            let (id_tx, id_rx) = std::sync::mpsc::channel();
            let handle = std::thread::Builder::new()
                .name("kbd-hook".into())
                .spawn(move || unsafe {
                    let _ = id_tx.send(GetCurrentThreadId());
                    let hook =
                        SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), std::ptr::null_mut(), 0);
                    if hook.is_null() {
                        return;
                    }
                    let mut msg: MSG = std::mem::zeroed();
                    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    UnhookWindowsHookEx(hook);
                })
                .expect("spawn kbd-hook");
            if let Ok(tid) = id_rx.recv_timeout(std::time::Duration::from_secs(2)) {
                self.thread = Some((handle, tid));
            }
        } else if !on {
            if let Some((handle, tid)) = self.thread.take() {
                unsafe {
                    PostThreadMessageW(tid, WM_QUIT, 0, 0);
                }
                let _ = handle.join();
            }
            *shared().lock().unwrap() = None;
            // release any stuck "held" state in the engine
            let _ = self.tx.send(Cmd::Input { taps: vec![], held: LedMask::none() });
        }
    }
}

impl Drop for InputCapture {
    fn drop(&mut self) {
        self.set_capture(false);
    }
}
