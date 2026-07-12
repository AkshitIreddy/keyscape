//! User-defined JavaScript effects, running on an embedded QuickJS engine —
//! no runtime to install, same language family as the UI.
//!
//! Drop a `.js` file into `%APPDATA%\Keyscape\effects\` defining:
//!
//! ```js
//! const EFFECT = { id: "my_fx", name: "My FX", params: [...] };
//! function render(req) {           // req: {t, dt, params, palette, taps, audio}
//!   return keys.map(k => [255, 0, 0]);   // one [r,g,b] 0-255 per key
//! }
//! ```
//!
//! Globals available to scripts: `keys` (array of {i, led, cx, cy, row, col,
//! name}), `state` (a plain object that persists between frames), `seed`.
//! An optional `setup()` runs once. See docs/js-effects.md and examples/.
//!
//! Each active scripted effect owns one interpreter on its own thread; the
//! engine mails it frame requests and reads the latest reply, so a slow
//! script can never stall the render loop. A QuickJS interrupt handler
//! aborts any eval that exceeds its time budget — an accidental
//! `while(true)` costs one aborted frame, not a hung engine.

use super::{Effect, EffectInfo, RenderCtx};
use crate::color::Col;
use crate::frame::Frame;
use crate::layout::Layout;
use crate::params::ParamSpec;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Per-eval time budget. Generous for 88 keys of arithmetic; small enough
/// that a runaway loop can't freeze the effect thread.
const EVAL_BUDGET_MS: u64 = 60;
/// Consecutive faulted frames before the effect is declared dead.
const MAX_FAULTS: u32 = 10;

struct ScriptMeta {
    source: String,
    extras: Vec<ParamSpec>,
}

use std::sync::RwLock;

static META: OnceLock<RwLock<HashMap<String, ScriptMeta>>> = OnceLock::new();

fn meta() -> &'static RwLock<HashMap<String, ScriptMeta>> {
    META.get_or_init(|| RwLock::new(HashMap::new()))
}

static STATUS: OnceLock<RwLock<Vec<Value>>> = OnceLock::new();

fn status_store() -> &'static RwLock<Vec<Value>> {
    STATUS.get_or_init(|| RwLock::new(Vec::new()))
}

/// Per-file scan results for the UI: {file, id?, name?, error?}.
pub fn statuses() -> Vec<Value> {
    status_store().read().unwrap().clone()
}

/// Validate a candidate script without saving it. Returns the manifest id.
pub fn validate(content: &str) -> Result<String, String> {
    let man = eval_oneshot(content, "JSON.stringify(EFFECT)", Duration::from_millis(400))?;
    let man: Value = serde_json::from_str(&man)
        .map_err(|_| "EFFECT is not a plain data object".to_string())?;
    if !man.get("params").map(|p| p.is_null() || p.is_array()).unwrap_or(true) {
        return Err("EFFECT.params must be an array".into());
    }
    man.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "EFFECT.id missing".into())
}

pub fn effects_dir() -> std::path::PathBuf {
    crate::settings::config_dir().join("effects")
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn spec_from_value(p: &Value) -> Option<ParamSpec> {
    let key = leak(p.get("key")?.as_str()?.to_string());
    let label = leak(p.get("label")?.as_str()?.to_string());
    let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("slider");
    match kind {
        "toggle" => Some(ParamSpec::toggle(
            key,
            label,
            p.get("default").and_then(|v| v.as_bool()).unwrap_or(false),
        )),
        "select" => {
            let options: Vec<&'static str> = p
                .get("options")?
                .as_array()?
                .iter()
                .filter_map(|o| o.as_str())
                .map(|o| leak(o.to_string()))
                .collect();
            let default = p.get("default").and_then(|v| v.as_str()).unwrap_or(options.first()?);
            Some(ParamSpec::select(key, label, options.clone(), leak(default.to_string())))
        }
        _ => Some(ParamSpec::slider(
            key,
            label,
            p.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
            p.get("max").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32,
            p.get("step").and_then(|v| v.as_f64()).unwrap_or(0.01) as f32,
            p.get("default").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
        )),
    }
}

/// Run one guarded eval sequence in a throwaway interpreter and return the
/// stringified value of the final expression.
fn eval_oneshot(source: &str, expr: &str, budget: Duration) -> Result<String, String> {
    let rt = rquickjs::Runtime::new().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + budget;
    rt.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    let ctx = rquickjs::Context::full(&rt).map_err(|e| e.to_string())?;
    ctx.with(|c| {
        c.eval::<(), _>(source.as_bytes()).map_err(|e| format!("{e}"))?;
        c.eval::<String, _>(expr.as_bytes()).map_err(|e| format!("{e}"))
    })
}

/// Extra param specs for a scripted effect id ([] for native effects).
pub fn extras_for(id: &str) -> Vec<ParamSpec> {
    meta().read().unwrap().get(id).map(|m| m.extras.clone()).unwrap_or_default()
}

pub fn is_scripted(id: &str) -> bool {
    meta().read().unwrap().contains_key(id)
}

/// Scan the effects dir and register every valid script. Called once at
/// startup before the engine spawns.
pub fn scan() -> Vec<EffectInfo> {
    let mut infos = Vec::new();
    let mut metas = HashMap::new();
    let dir = effects_dir();
    let _ = std::fs::create_dir_all(&dir);

    // first run: seed the folder with the examples shipped next to the exe
    // (the NSIS installer places them at <install>\examples\js-effects)
    let empty = std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true);
    if empty {
        if let Some(examples) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("examples").join("js-effects")))
        {
            if let Ok(rd) = std::fs::read_dir(examples) {
                for e in rd.flatten() {
                    if e.path().extension().and_then(|x| x.to_str()) == Some("js") {
                        let _ = std::fs::copy(e.path(), dir.join(e.file_name()));
                    }
                }
            }
        }
    }

    let mut statuses: Vec<Value> = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => {
            *meta().write().unwrap() = metas;
            *status_store().write().unwrap() = statuses;
            return infos;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("js") {
            continue;
        }
        let file = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut fail = |err: String, statuses: &mut Vec<Value>| {
            eprintln!("js effect {file}: {err}");
            statuses.push(json!({"file": file, "error": err}));
        };
        let Ok(source) = std::fs::read_to_string(&path) else {
            fail("unreadable file".into(), &mut statuses);
            continue;
        };
        let man = match eval_oneshot(&source, "JSON.stringify(EFFECT)", Duration::from_millis(400))
        {
            Ok(s) => s,
            Err(e) => {
                fail(e, &mut statuses);
                continue;
            }
        };
        let Ok(man) = serde_json::from_str::<Value>(&man) else {
            fail("EFFECT is not a plain data object".into(), &mut statuses);
            continue;
        };
        let Some(id) = man.get("id").and_then(|v| v.as_str()) else {
            fail("EFFECT.id missing".into(), &mut statuses);
            continue;
        };
        if super::builtin_by_id(id).is_some() || metas.contains_key(id) {
            fail(format!("effect id '{id}' already exists"), &mut statuses);
            continue;
        }
        let extras: Vec<ParamSpec> = man
            .get("params")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(spec_from_value).collect())
            .unwrap_or_default();
        let name = man.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let blurb = man.get("blurb").and_then(|v| v.as_str()).unwrap_or("User JavaScript effect.");
        let category = man.get("category").and_then(|v| v.as_str()).unwrap_or("Custom");
        let default_palette = man.get("palette").and_then(|v| v.as_str()).unwrap_or("aurora");
        infos.push(EffectInfo {
            id: leak(id.to_string()),
            name: leak(name.to_string()),
            category: leak(category.to_string()),
            blurb: leak(format!(
                "{blurb} ({})",
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
            needs_input: man.get("needs_input").and_then(|v| v.as_bool()).unwrap_or(false),
            default_palette: leak(default_palette.to_string()),
            extras: super::no_extras, // real extras come from script::extras_for
            make: |_, _| Box::new(DeadEffect), // engine special-cases scripted ids
        });
        statuses.push(json!({"file": file, "id": id, "name": name}));
        metas.insert(id.to_string(), ScriptMeta { source, extras });
    }
    *meta().write().unwrap() = metas;
    *status_store().write().unwrap() = statuses;
    infos
}

/// Spawn the interpreter thread for a scripted effect.
pub fn make(id: &str, layout: &Layout, seed: u64) -> Box<dyn Effect> {
    let source = match meta().read().unwrap().get(id) {
        Some(m) => m.source.clone(),
        None => return Box::new(DeadEffect),
    };

    let keys: Vec<Value> = layout
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            json!({"i": i, "led": k.led, "cx": k.cx, "cy": k.cy, "row": k.row, "col": k.col, "name": k.name})
        })
        .collect();
    let keys_json = Value::Array(keys).to_string();
    let n_keys = layout.keys.len();

    let latest: Arc<Mutex<Vec<Col>>> = Arc::new(Mutex::new(vec![Col::BLACK; n_keys]));
    let alive = Arc::new(AtomicBool::new(true));
    let stop = Arc::new(AtomicBool::new(false));
    // shared deadline (ms since thread epoch); 0 = no eval in progress
    let deadline_ms = Arc::new(AtomicU64::new(u64::MAX));
    let (req_tx, req_rx): (SyncSender<String>, Receiver<String>) = sync_channel(1);

    {
        let latest = latest.clone();
        let alive = alive.clone();
        let stop = stop.clone();
        let deadline_ms = deadline_ms.clone();
        std::thread::Builder::new()
            .name("js-effect".into())
            .spawn(move || {
                let epoch = Instant::now();
                let rt = match rquickjs::Runtime::new() {
                    Ok(rt) => rt,
                    Err(_) => {
                        alive.store(false, Ordering::Relaxed);
                        return;
                    }
                };
                {
                    let deadline_ms = deadline_ms.clone();
                    let stop = stop.clone();
                    rt.set_interrupt_handler(Some(Box::new(move || {
                        stop.load(Ordering::Relaxed)
                            || epoch.elapsed().as_millis() as u64
                                >= deadline_ms.load(Ordering::Relaxed)
                    })));
                }
                let ctx = match rquickjs::Context::full(&rt) {
                    Ok(c) => c,
                    Err(_) => {
                        alive.store(false, Ordering::Relaxed);
                        return;
                    }
                };

                let mut guard = |budget: Duration| {
                    deadline_ms
                        .store((epoch.elapsed() + budget).as_millis() as u64, Ordering::Relaxed);
                };
                let done = |deadline_ms: &AtomicU64| deadline_ms.store(u64::MAX, Ordering::Relaxed);

                // load script + init globals + optional setup()
                guard(Duration::from_millis(500));
                let init = ctx.with(|c| {
                    c.eval::<(), _>(source.as_bytes()).map_err(|e| format!("{e}"))?;
                    let boot = format!(
                        "globalThis.keys = {keys_json}; globalThis.state = {{}}; \
                         globalThis.seed = {seed}; \
                         if (typeof setup === 'function') setup();"
                    );
                    c.eval::<(), _>(boot.as_bytes()).map_err(|e| format!("{e}"))
                });
                done(&deadline_ms);
                if let Err(e) = init {
                    eprintln!("js effect init failed: {e}");
                    alive.store(false, Ordering::Relaxed);
                    return;
                }

                let mut faults = 0u32;
                while !stop.load(Ordering::Relaxed) {
                    let req = match req_rx.recv_timeout(Duration::from_millis(300)) {
                        Ok(r) => r,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    guard(Duration::from_millis(EVAL_BUDGET_MS));
                    let out = ctx.with(|c| {
                        let call = format!("JSON.stringify(render({req}))");
                        c.eval::<String, _>(call.as_bytes()).map_err(|e| format!("{e}"))
                    });
                    done(&deadline_ms);
                    match out.ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
                        Some(v) => {
                            faults = 0;
                            let mut buf = vec![Col::BLACK; n_keys];
                            match v {
                                Value::Array(arr) => {
                                    for (i, c) in arr.iter().take(n_keys).enumerate() {
                                        if let Some(c) = c.as_array() {
                                            buf[i] = col_from(c);
                                        }
                                    }
                                }
                                Value::Object(map) => {
                                    for (k, c) in map {
                                        if let (Ok(i), Some(c)) = (k.parse::<usize>(), c.as_array())
                                        {
                                            if i < n_keys {
                                                buf[i] = col_from(c);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                            *latest.lock().unwrap() = buf;
                        }
                        None => {
                            faults += 1;
                            if faults >= MAX_FAULTS {
                                alive.store(false, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                }
            })
            .ok();
    }

    Box::new(ScriptEffect { req_tx, latest, alive, stop, in_flight: Arc::new(AtomicU32::new(0)) })
}

fn col_from(c: &[Value]) -> Col {
    Col::rgb(
        (c.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 / 255.0).clamp(0.0, 1.0),
        (c.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 / 255.0).clamp(0.0, 1.0),
        (c.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 / 255.0).clamp(0.0, 1.0),
    )
}

struct ScriptEffect {
    req_tx: SyncSender<String>,
    latest: Arc<Mutex<Vec<Col>>>,
    alive: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    in_flight: Arc<AtomicU32>,
}

impl Effect for ScriptEffect {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        if self.alive.load(Ordering::Relaxed) {
            let pal: Vec<[u8; 3]> = (0..16)
                .map(|i| {
                    let c = ctx.palette.sample(i as f32 / 16.0).clamp01();
                    [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8]
                })
                .collect();
            let taps: Vec<Value> = ctx.taps.iter().map(|t| json!([t.key, t.cx, t.cy])).collect();
            let req = json!({
                "t": ctx.t,
                "dt": ctx.dt,
                "params": ctx.params,
                "palette": pal,
                "taps": taps,
                "audio": if ctx.audio.active {
                    json!({"level": ctx.audio.level, "bass": ctx.audio.bass, "mid": ctx.audio.mid,
                           "treble": ctx.audio.treble, "beat": ctx.audio.beat})
                } else { Value::Null },
            })
            .to_string();
            // capacity-1 channel: drop the request when the thread is busy
            match self.req_tx.try_send(req) {
                Ok(()) | Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Disconnected(_)) => self.alive.store(false, Ordering::Relaxed),
            }
        }

        let latest = self.latest.lock().unwrap();
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            if let Some(c) = latest.get(i) {
                out.set(k.led, *c);
            }
        }
        drop(latest);
        let _ = &self.in_flight;

        if !self.alive.load(Ordering::Relaxed) {
            // dead script: dim red heartbeat on Esc so the failure is visible
            let pulse = 0.25 + 0.2 * (ctx.t * 3.0).sin();
            if let Some(k) = ctx.layout.keys.first() {
                out.max(k.led, Col::rgb(pulse, 0.0, 0.0));
            }
        }
    }
}

impl Drop for ScriptEffect {
    fn drop(&mut self) {
        // interrupt handler observes `stop`, so even a mid-eval runaway
        // aborts and the thread exits (capture-thread lesson)
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Placeholder for unknown ids / spawn failures.
struct DeadEffect;

impl Effect for DeadEffect {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let pulse = 0.25 + 0.2 * (ctx.t * 3.0).sin();
        if let Some(k) = ctx.layout.keys.first() {
            out.set(k.led, Col::rgb(pulse, 0.0, 0.0));
        }
    }
}
