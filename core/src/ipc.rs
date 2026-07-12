//! Control API: JSON over WebSocket, bound to 127.0.0.1 only.
//!
//! Request: {"op": "...", ...fields, "req": <any>} — "req" is echoed back so
//! clients can match replies. Preview frames arrive as binary messages
//! (LED_COUNT*3 wire bytes) after "subscribe_preview".

use crate::effects;
use crate::engine::Cmd;
use crate::layout::Layout;
use serde_json::{json, Value};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::{accept, Error as WsError, Message};

pub const PORT: u16 = 53971;

pub fn serve(tx: Sender<Cmd>, layout: Arc<Layout>) {
    let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("IPC bind failed on port {PORT}: {e} (another keyscape-core running?)");
            return;
        }
    };
    println!("IPC listening on ws://127.0.0.1:{PORT}");
    for stream in listener.incoming().flatten() {
        let tx = tx.clone();
        let layout = layout.clone();
        std::thread::Builder::new()
            .name("ipc-conn".into())
            .spawn(move || {
                let _ = handle_conn(stream, tx, layout);
            })
            .ok();
    }
}

fn handle_conn(stream: TcpStream, tx: Sender<Cmd>, layout: Arc<Layout>) -> Result<(), WsError> {
    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(_) => return Ok(()),
    };
    // Short read timeout turns the blocking socket into a poll loop so one
    // thread can both serve requests and push preview frames.
    ws.get_ref().set_read_timeout(Some(Duration::from_millis(25))).ok();
    let mut preview: Option<Receiver<Vec<u8>>> = None;

    loop {
        match ws.read() {
            Ok(Message::Text(txt)) => {
                let req: Value = serde_json::from_str(&txt).unwrap_or(Value::Null);
                let mut resp = handle_op(&req, &tx, &layout, &mut preview);
                if let Some(r) = req.get("req") {
                    resp["req"] = r.clone();
                }
                ws.send(Message::Text(resp.to_string()))?;
            }
            Ok(Message::Ping(p)) => ws.send(Message::Pong(p))?,
            Ok(Message::Close(_)) => return Ok(()),
            Ok(_) => {}
            Err(WsError::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return Ok(()),
        }
        if let Some(rx) = &preview {
            // collapse to the newest frame; skipping intermediates is fine
            let mut last = None;
            while let Ok(f) = rx.try_recv() {
                last = Some(f);
            }
            if let Some(f) = last {
                ws.send(Message::Binary(f))?;
            }
        }
    }
}

fn handle_op(
    req: &Value,
    tx: &Sender<Cmd>,
    layout: &Arc<Layout>,
    preview: &mut Option<Receiver<Vec<u8>>>,
) -> Value {
    let op = req.get("op").and_then(|v| v.as_str()).unwrap_or("");
    match op {
        "ping" => json!({"ok": true, "pong": true}),
        "status" => {
            let (stx, srx) = channel();
            let _ = tx.send(Cmd::QueryStatus(stx));
            match srx.recv_timeout(Duration::from_millis(750)) {
                Ok(v) => json!({"ok": true, "status": v}),
                Err(_) => json!({"ok": false, "error": "engine timeout"}),
            }
        }
        "effects" => {
            let list: Vec<Value> = effects::registry()
                .iter()
                .map(|e| {
                    json!({
                        "id": e.id,
                        "name": e.name,
                        "category": e.category,
                        "blurb": e.blurb,
                        "needs_input": e.needs_input,
                        "default_palette": e.default_palette,
                        "specs": e.specs(),
                    })
                })
                .collect();
            json!({"ok": true, "effects": list})
        }
        "palettes" => {
            let list: Vec<Value> = crate::palette::builtins()
                .into_iter()
                .map(|(id, name, p)| {
                    let stops: Vec<Value> = p
                        .stops()
                        .iter()
                        .map(|(t, c)| {
                            json!({
                                "t": t,
                                "c": format!(
                                    "#{:02X}{:02X}{:02X}",
                                    (c.r * 255.0) as u8,
                                    (c.g * 255.0) as u8,
                                    (c.b * 255.0) as u8
                                )
                            })
                        })
                        .collect();
                    json!({"id": id, "name": name, "stops": stops})
                })
                .collect();
            json!({"ok": true, "palettes": list})
        }
        "layout" => {
            let keys: Vec<Value> = layout
                .keys
                .iter()
                .map(|k| {
                    json!({
                        "led": k.led, "name": k.name, "row": k.row, "col": k.col,
                        "x": k.x, "y": k.y, "w": k.w, "h": k.h,
                    })
                })
                .collect();
            let aux: Vec<Value> =
                layout.aux.iter().map(|(led, name)| json!({"led": led, "name": name})).collect();
            json!({"ok": true, "aspect": layout.aspect, "keys": keys, "aux": aux})
        }
        "masks" => {
            let list: Vec<Value> = Layout::mask_names()
                .iter()
                .map(|name| {
                    let m = layout.mask(name);
                    let leds: Vec<usize> =
                        layout.keys.iter().map(|k| k.led).filter(|&l| m.get(l)).collect();
                    json!({"name": name, "leds": leds})
                })
                .collect();
            json!({"ok": true, "masks": list})
        }
        "set_effect" => {
            if let Some(id) = req.get("id").and_then(|v| v.as_str()) {
                let _ = tx.send(Cmd::SetEffect(id.to_string()));
                json!({"ok": true})
            } else {
                json!({"ok": false, "error": "missing id"})
            }
        }
        "set_params" => {
            let id = req.get("id").and_then(|v| v.as_str());
            let params = req.get("params").and_then(|v| v.as_object());
            match (id, params) {
                (Some(id), Some(p)) => {
                    let _ = tx.send(Cmd::SetParams { id: id.to_string(), params: p.clone() });
                    json!({"ok": true})
                }
                _ => json!({"ok": false, "error": "missing id/params"}),
            }
        }
        "patch_settings" => match req.get("patch") {
            Some(p) if p.is_object() => {
                let _ = tx.send(Cmd::PatchSettings(p.clone()));
                json!({"ok": true})
            }
            _ => json!({"ok": false, "error": "missing patch"}),
        },
        "next" => {
            let _ = tx.send(Cmd::NextEffect);
            json!({"ok": true})
        }
        "subscribe_preview" => {
            let (ptx, prx) = channel();
            let _ = tx.send(Cmd::SubscribePreview(ptx));
            *preview = Some(prx);
            json!({"ok": true})
        }
        "scripts" => {
            json!({
                "ok": true,
                "dir": crate::effects::script::effects_dir().to_string_lossy(),
                "scripts": crate::effects::script::statuses(),
            })
        }
        "save_script" => {
            let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let content = req.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let safe: String = name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                .collect();
            if !safe.ends_with(".js") || safe.len() < 4 || content.is_empty() {
                json!({"ok": false, "error": "need a .js file with content"})
            } else {
                match crate::effects::script::validate(content) {
                    Err(e) => json!({"ok": false, "error": e}),
                    Ok(_) => {
                        let path = crate::effects::script::effects_dir().join(&safe);
                        match std::fs::write(&path, content) {
                            Err(e) => json!({"ok": false, "error": e.to_string()}),
                            Ok(()) => {
                                crate::effects::register_scripts(crate::effects::script::scan());
                                json!({"ok": true, "file": safe})
                            }
                        }
                    }
                }
            }
        }
        "delete_script" => {
            let file = req.get("file").and_then(|v| v.as_str()).unwrap_or("");
            let safe: String = file
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                .collect();
            if !safe.ends_with(".js") {
                json!({"ok": false, "error": "bad filename"})
            } else {
                let path = crate::effects::script::effects_dir().join(&safe);
                match std::fs::remove_file(path) {
                    Err(e) => json!({"ok": false, "error": e.to_string()}),
                    Ok(()) => {
                        crate::effects::register_scripts(crate::effects::script::scan());
                        json!({"ok": true})
                    }
                }
            }
        }
        "rescan_scripts" => {
            crate::effects::register_scripts(crate::effects::script::scan());
            json!({"ok": true, "scripts": crate::effects::script::statuses()})
        }
        "open_effects_dir" => {
            let dir = crate::effects::script::effects_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("explorer.exe").arg(&dir).spawn();
            json!({"ok": true})
        }
        "quit" => {
            // clean daemon shutdown (used by --zone-test and scripts)
            let _ = tx.send(Cmd::Shutdown);
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(700));
                std::process::exit(0);
            });
            json!({"ok": true})
        }
        "guard_running" => json!({"ok": true, "running": crate::guard::is_running()}),
        "guard_fix" => {
            crate::guard::elevate_disable();
            json!({"ok": true})
        }
        "guard_restore" => {
            crate::guard::elevate_enable();
            json!({"ok": true})
        }
        _ => json!({"ok": false, "error": format!("unknown op '{op}'")}),
    }
}
