//! Tray icon for the always-on core: Open Keyscape / Pause lighting / Quit.
//! Pure win32 (hidden message window + Shell_NotifyIconW) â€” the daemon has no
//! other visible surface, so this is how you find and stop it without the UI.

use crate::engine::Cmd;
use serde_json::json;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, HICON, ICONINFO,
    MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG, TPM_RIGHTBUTTON, WM_APP, WM_COMMAND,
    WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
};

const WM_TRAY: u32 = WM_APP + 1;
const CMD_OPEN: usize = 1;
const CMD_PAUSE: usize = 2;
const CMD_QUIT: usize = 3;

struct TrayCtx {
    tx: Sender<Cmd>,
    on_quit: Box<dyn Fn() + Send + Sync>,
}

static CTX: OnceLock<TrayCtx> = OnceLock::new();

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 32x32 mini version of the app icon (3x2 keycap grid), built in memory so
/// the daemon binary needs no resource section.
unsafe fn make_icon() -> HICON {
    let mut px = [0u32; 32 * 32];
    let key_cols = [
        [0xFF22D3A5u32, 0xFF40AADC, 0xFF7C5CFF],
        [0xFF2DBEC8u32, 0xFF5E82F0, 0xFF9650FF],
    ];
    for y in 0..32usize {
        for x in 0..32usize {
            let col = if (4..11).contains(&x) {
                0
            } else if (13..20).contains(&x) {
                1
            } else if (22..29).contains(&x) {
                2
            } else {
                9
            };
            let row = if (7..15).contains(&y) {
                0
            } else if (17..25).contains(&y) {
                1
            } else {
                9
            };
            px[y * 32 + x] = if col < 3 && row < 2 { key_cols[row][col] } else { 0xFF0B0E18 };
        }
    }
    let color = CreateBitmap(32, 32, 1, 32, px.as_ptr() as *const _);
    let mask = CreateBitmap(32, 32, 1, 1, std::ptr::null());
    let info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
    let icon = CreateIconIndirect(&info);
    DeleteObject(color);
    DeleteObject(mask);
    icon
}

fn open_ui() {
    use std::os::windows::process::CommandExt;
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("Keyscape.exe"), dir.join("../release/Keyscape.exe")] {
                if cand.exists() {
                    let _ = std::process::Command::new(cand).creation_flags(0x0000_0200).spawn();
                    return;
                }
            }
        }
    }
}

fn engine_paused() -> bool {
    let Some(ctx) = CTX.get() else { return false };
    let (stx, srx) = channel();
    let _ = ctx.tx.send(Cmd::QueryStatus(stx));
    srx.recv_timeout(Duration::from_millis(300))
        .ok()
        .and_then(|v| v.pointer("/settings/paused").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    let paused = engine_paused();
    AppendMenuW(menu, MF_STRING, CMD_OPEN, wide("Open Keyscape").as_ptr());
    AppendMenuW(
        menu,
        MF_STRING | if paused { MF_CHECKED } else { 0 },
        CMD_PAUSE,
        wide("Pause lighting").as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, CMD_QUIT, wide("Quit lighting core").as_ptr());
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, std::ptr::null());
    DestroyMenu(menu);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            let ev = (l as u32) & 0xFFFF;
            if ev == WM_LBUTTONUP {
                open_ui();
            } else if ev == WM_RBUTTONUP || ev == WM_CONTEXTMENU {
                show_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            match w & 0xFFFF {
                CMD_OPEN => open_ui(),
                CMD_PAUSE => {
                    if let Some(ctx) = CTX.get() {
                        let paused = engine_paused();
                        let _ = ctx.tx.send(Cmd::PatchSettings(json!({ "paused": !paused })));
                    }
                }
                CMD_QUIT => {
                    if let Some(ctx) = CTX.get() {
                        (ctx.on_quit)();
                    }
                    PostQuitMessage(0);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

/// Blocks running the tray message loop; spawn on its own thread.
pub fn run(tx: Sender<Cmd>, on_quit: Box<dyn Fn() + Send + Sync>) {
    let _ = CTX.set(TrayCtx { tx, on_quit });
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class_name = wide("KeyscapeTray");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("Keyscape").as_ptr(),
            WS_OVERLAPPED,
            0, 0, 0, 0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );

        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = make_icon();
        let tip = wide(concat!("Keyscape lighting core v", env!("CARGO_PKG_VERSION")));
        nid.szTip[..tip.len().min(128)].copy_from_slice(&tip[..tip.len().min(128)]);
        Shell_NotifyIconW(NIM_ADD, &nid);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}
