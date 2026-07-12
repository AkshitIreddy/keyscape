//! User-supplied Python effects.
//!
//! Drop a `.py` file into `%APPDATA%\Keyscape\effects\` defining an `EFFECT`
//! manifest dict and a `render()` function (see docs/python-effects.md and
//! examples/python-effects/). Scripts are discovered at core startup, appear
//! in the UI under their manifest category (default "Custom"), and run in a
//! sandboxed-by-nothing subprocess — only add scripts you trust.
//!
//! Runtime model: one `python py_runner.py <script>` child per active
//! effect. The engine's render tick sends a JSON request line (time, params,
//! taps, audio, palette samples) whenever no reply is outstanding, and a
//! reader thread parses reply lines into a latest-frame buffer. A slow or
//! dead script never blocks the engine — the last good frame persists, and a
//! dead child shows a dim red heartbeat on Esc so the failure is visible.

use super::{Effect, EffectInfo, RenderCtx};
use crate::color::Col;
use crate::frame::Frame;
use crate::layout::Layout;
use crate::params::ParamSpec;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::process::CommandExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// script path + extra param specs per python effect id (EffectInfo's fn
/// pointers can't capture per-script data, so these live in side maps).
struct PyMeta {
    script: std::path::PathBuf,
    extras: Vec<ParamSpec>,
}

static META: OnceLock<HashMap<String, PyMeta>> = OnceLock::new();

pub fn effects_dir() -> std::path::PathBuf {
    crate::settings::config_dir().join("effects")
}

fn python_exe() -> Option<&'static str> {
    static EXE: OnceLock<Option<&'static str>> = OnceLock::new();
    *EXE.get_or_init(|| {
        for cand in ["python", "py"] {
            let ok = Command::new(cand)
                .arg("--version")
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Some(cand);
            }
        }
        None
    })
}

fn runner_path() -> std::path::PathBuf {
    let p = crate::settings::config_dir().join("py_runner.py");
    let src = include_str!("../../assets/py_runner.py");
    // keep the deployed runner in sync with the build
    if std::fs::read_to_string(&p).map(|cur| cur != src).unwrap_or(true) {
        let _ = std::fs::create_dir_all(crate::settings::config_dir());
        let _ = std::fs::write(&p, src);
    }
    p
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn spec_from_value(p: &Value) -> Option<ParamSpec> {
    let key = leak(p.get("key")?.as_str()?.to_string());
    let label = leak(p.get("label")?.as_str()?.to_string());
    let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("slider");
    match kind {
        "toggle" => Some(ParamSpec::toggle(key, label, p.get("default").and_then(|v| v.as_bool()).unwrap_or(false))),
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

/// Extra param specs for a python effect id ([] for native effects).
pub fn extras_for(id: &str) -> Vec<ParamSpec> {
    META.get()
        .and_then(|m| m.get(id))
        .map(|m| m.extras.clone())
        .unwrap_or_default()
}

pub fn is_python(id: &str) -> bool {
    META.get().map(|m| m.contains_key(id)).unwrap_or(false)
}

/// Scan the effects dir, probe each script's EFFECT manifest, and build
/// registry entries. Called once at startup before the engine spawns.
pub fn scan() -> Vec<EffectInfo> {
    let mut infos = Vec::new();
    let mut metas = HashMap::new();
    let dir = effects_dir();
    let _ = std::fs::create_dir_all(&dir);

    let Some(py) = python_exe() else {
        let _ = META.set(metas);
        return infos;
    };
    let runner = runner_path();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => {
            let _ = META.set(metas);
            return infos;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("py") {
            continue;
        }
        let out = Command::new(py)
            .arg(&runner)
            .arg(&path)
            .arg("--manifest")
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let Ok(out) = out else { continue };
        if !out.status.success() {
            eprintln!("python effect {} failed manifest probe", path.display());
            continue;
        }
        let Ok(man) = serde_json::from_slice::<Value>(&out.stdout) else {
            eprintln!("python effect {} has invalid EFFECT manifest", path.display());
            continue;
        };
        let Some(id) = man.get("id").and_then(|v| v.as_str()) else { continue };
        if super::by_id(id).is_some() || metas.contains_key(id) {
            eprintln!("python effect id '{id}' collides; skipping {}", path.display());
            continue;
        }
        let extras: Vec<ParamSpec> = man
            .get("params")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(spec_from_value).collect())
            .unwrap_or_default();
        let name = man.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let blurb = man.get("blurb").and_then(|v| v.as_str()).unwrap_or("User Python effect.");
        let category = man.get("category").and_then(|v| v.as_str()).unwrap_or("Custom");
        let default_palette = man.get("palette").and_then(|v| v.as_str()).unwrap_or("aurora");
        infos.push(EffectInfo {
            id: leak(id.to_string()),
            name: leak(name.to_string()),
            category: leak(category.to_string()),
            blurb: leak(format!("{blurb} ({})", path.file_name().unwrap_or_default().to_string_lossy())),
            needs_input: man.get("needs_input").and_then(|v| v.as_bool()).unwrap_or(false),
            default_palette: leak(default_palette.to_string()),
            extras: super::no_extras, // real extras come from python::extras_for
            make: |_, _| Box::new(DeadEffect), // engine special-cases python ids
        });
        metas.insert(id.to_string(), PyMeta { script: path, extras });
    }
    let _ = META.set(metas);
    infos
}

/// Spawn the runner for a python effect. Returns a visible-failure effect if
/// anything goes wrong so the board never just sits there dark.
pub fn make(id: &str, layout: &Layout, seed: u64) -> Box<dyn Effect> {
    let Some(meta) = META.get().and_then(|m| m.get(id)) else {
        return Box::new(DeadEffect);
    };
    let Some(py) = python_exe() else { return Box::new(DeadEffect) };

    let child = Command::new(py)
        .arg(runner_path())
        .arg(&meta.script)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return Box::new(DeadEffect) };
    let Some(mut stdin) = child.stdin.take() else { return Box::new(DeadEffect) };
    let Some(stdout) = child.stdout.take() else { return Box::new(DeadEffect) };

    let keys: Vec<Value> = layout
        .keys
        .iter()
        .enumerate()
        .map(|(i, k)| {
            json!({"i": i, "led": k.led, "cx": k.cx, "cy": k.cy, "row": k.row, "col": k.col, "name": k.name})
        })
        .collect();
    let init = json!({"keys": keys, "seed": seed}).to_string() + "\n";
    if stdin.write_all(init.as_bytes()).is_err() {
        let _ = child.kill();
        return Box::new(DeadEffect);
    }

    let latest: Arc<Mutex<Vec<Col>>> = Arc::new(Mutex::new(vec![Col::BLACK; layout.keys.len()]));
    let alive = Arc::new(AtomicBool::new(true));
    let in_flight = Arc::new(AtomicU32::new(0));

    let reader_latest = latest.clone();
    let reader_alive = alive.clone();
    let reader_flight = in_flight.clone();
    let n_keys = layout.keys.len();
    std::thread::Builder::new()
        .name("py-effect".into())
        .spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            // handshake ("ready") line
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                reader_alive.store(false, Ordering::Relaxed);
                return;
            }
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                reader_flight.fetch_sub(1, Ordering::Relaxed);
                if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&line) {
                    let mut buf = vec![Col::BLACK; n_keys];
                    for (i, c) in arr.iter().take(n_keys).enumerate() {
                        if let Some(c) = c.as_array() {
                            buf[i] = Col::rgb(
                                c.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 / 255.0,
                                c.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 / 255.0,
                                c.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 / 255.0,
                            );
                        }
                    }
                    *reader_latest.lock().unwrap() = buf;
                }
            }
            reader_alive.store(false, Ordering::Relaxed);
        })
        .ok();

    Box::new(PyEffect { child, stdin, latest, alive, in_flight })
}

struct PyEffect {
    child: Child,
    stdin: ChildStdin,
    latest: Arc<Mutex<Vec<Col>>>,
    alive: Arc<AtomicBool>,
    in_flight: Arc<AtomicU32>,
}

impl Effect for PyEffect {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        if self.alive.load(Ordering::Relaxed) && self.in_flight.load(Ordering::Relaxed) == 0 {
            // 16 palette samples so scripts can honor the user's theme
            let pal: Vec<[u8; 3]> = (0..16)
                .map(|i| {
                    let c = ctx.palette.sample(i as f32 / 16.0).clamp01();
                    [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8]
                })
                .collect();
            let taps: Vec<Value> =
                ctx.taps.iter().map(|t| json!([t.key, t.cx, t.cy])).collect();
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
            .to_string()
                + "\n";
            self.in_flight.fetch_add(1, Ordering::Relaxed);
            if self.stdin.write_all(req.as_bytes()).is_err() {
                self.alive.store(false, Ordering::Relaxed);
            }
        }

        let latest = self.latest.lock().unwrap();
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            if let Some(c) = latest.get(i) {
                out.set(k.led, *c);
            }
        }
        drop(latest);

        if !self.alive.load(Ordering::Relaxed) {
            // dead script: dim red heartbeat on Esc so the failure is visible
            let pulse = 0.25 + 0.2 * (ctx.t * 3.0).sin();
            if let Some(k) = ctx.layout.keys.first() {
                out.max(k.led, Col::rgb(pulse, 0.0, 0.0));
            }
        }
    }
}

impl Drop for PyEffect {
    fn drop(&mut self) {
        // never leave orphan interpreters running (capture-thread lesson)
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Placeholder for spawn failures / unknown ids.
struct DeadEffect;

impl Effect for DeadEffect {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let pulse = 0.25 + 0.2 * (ctx.t * 3.0).sin();
        if let Some(k) = ctx.layout.keys.first() {
            out.set(k.led, Col::rgb(pulse, 0.0, 0.0));
        }
    }
}
