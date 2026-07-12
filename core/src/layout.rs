//! Physical keyboard layout. On startup this is parsed live from the machine's
//! own ASUS per-key DeviceContent CSV (so Keyscape adapts to whatever ROG
//! N-KEY laptop it runs on); if those vendor files are absent it falls back to
//! the bundled layout JSON. The offline tool tools/parse-layout.mjs produces
//! that bundled JSON and is the reference implementation of this parser.

use crate::frame::{LedMask, LED_COUNT};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct RawKey {
    led: u16,
    name: String,
    row: u8,
    col: u8,
    scan: Option<u16>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Deserialize)]
struct RawAux {
    led: u16,
    name: String,
}

#[derive(Deserialize)]
struct RawLayout {
    keys: Vec<RawKey>,
    aux: Vec<RawAux>,
    #[serde(default)]
    aspect: Option<f32>,
}

/// Aux LED map for the N-KEY family (logo / front bar / rear), not derived
/// from the CSV — see tools/parse-layout.mjs for why.
const AUX_MAP: [(u16, &str); 10] = [
    (167, "Logo"), (168, "Logo2"), (174, "BarL1"), (173, "BarL2"), (172, "BarL3"),
    (171, "BarR3"), (170, "BarR2"), (169, "BarR1"), (176, "RearL"), (177, "RearR"),
];

/// Vendor note -> friendly key name (mirrors the JS tool).
fn rename(note: &str) -> String {
    match note {
        "VOL_DN" => "VolDn", "VOL_UP" => "VolUp", "Mic On/Off" => "Mic", "HyperFan" => "Fan",
        "Armoury Crate" => "Rog", "Delete" => "Del", "~" => "`", "Minus" => "-", "Equal" => "=",
        "Back" => "Backspace", "PLAY" => "Play", "STOP" => "Stop", "Cap" => "Caps", "\"" => "'",
        "ENTER" => "Enter", "PREV" => "Prev", "L_Shift" => "LShift", "?" => "/", "R_Shift" => "RShift",
        "UP_ARROW" => "Up", "NEXT" => "Next", "L_Ctrl" => "LCtrl", "L_Fn" => "Fn", "L_Alt" => "LAlt",
        "R_Alt" => "RAlt", "PRTSC" => "PrtSc", "R_Ctrl" => "RCtrl", "L_ARROW" => "Left",
        "DN_ARROW" => "Down", "R_ARROW" => "Right",
        other => other,
    }
    .to_string()
}

/// Split one CSV line, honoring quoted fields (the quote key is `""""`).
fn parse_line(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_q {
            if c == '"' && chars.get(i + 1) == Some(&'"') {
                cur.push('"');
                i += 1;
            } else if c == '"' {
                in_q = false;
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_q = true;
        } else if c == ',' {
            cells.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
        i += 1;
    }
    cells.push(cur);
    cells
}

/// Parse an ASUS per-key CSV into (keys, aux, aspect). Returns None if the
/// text isn't a plausible per-key table.
fn parse_csv(content: &str) -> Option<(Vec<RawKey>, Vec<RawAux>, f32)> {
    let content = content.trim_start_matches('\u{feff}');
    let mut tmp: Vec<(RawKey, [f32; 4])> = Vec::new();
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        let c = parse_line(line);
        if !c.first().map(|s| s.starts_with("LED ")).unwrap_or(false) {
            continue;
        }
        let led: u16 = match c[0][4..].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let num = |idx: usize| c.get(idx).and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(0.0);
        let (gx, gy, exist) = (num(1) as u8, num(2) as u8, num(3) as i32);
        let px = [num(4), num(5), num(6), num(7)];
        let note = c.get(9).map(String::as_str).unwrap_or("");
        let key_code = c.get(11).map(String::as_str).unwrap_or("");
        if exist != 1 || led > 166 {
            continue;
        }
        let name = rename(note);
        let mut scan = if !key_code.is_empty() && key_code != "NULL" {
            u16::from_str_radix(key_code.trim(), 16).ok()
        } else {
            None
        };
        // vendor CSV swaps these; real PS/2 set-1 is LShift=0x2A, LAlt=0x38
        if name == "LShift" {
            scan = Some(0x2A);
        }
        if name == "LAlt" {
            scan = Some(0x38);
        }
        tmp.push((RawKey { led, name, row: gy, col: gx, scan, x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, px));
    }
    if tmp.len() < 40 {
        return None;
    }
    let min_x = tmp.iter().map(|(_, p)| p[0]).fold(f32::MAX, f32::min);
    let min_y = tmp.iter().map(|(_, p)| p[1]).fold(f32::MAX, f32::min);
    let max_x = tmp.iter().map(|(_, p)| p[2]).fold(f32::MIN, f32::max);
    let max_y = tmp.iter().map(|(_, p)| p[3]).fold(f32::MIN, f32::max);
    let (w, h) = ((max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
    let mut keys: Vec<RawKey> = tmp
        .into_iter()
        .map(|(mut k, p)| {
            k.x = (p[0] - min_x) / w;
            k.y = (p[1] - min_y) / h;
            k.w = (p[2] - p[0]) / w;
            k.h = (p[3] - p[1]) / h;
            k
        })
        .collect();
    keys.sort_by_key(|k| k.led);
    let aux = AUX_MAP.iter().map(|(led, name)| RawAux { led: *led, name: name.to_string() }).collect();
    Some((keys, aux, w / h))
}

/// Look for this machine's own per-key CSV under ASUS's DeviceContent and
/// parse it, so Keyscape adapts to whatever ROG laptop it runs on. Falls back
/// to the bundled layout when the vendor files aren't present.
fn discover_and_parse() -> Option<(Vec<RawKey>, Vec<RawAux>, f32)> {
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into());
    let base = std::path::Path::new(&program_data)
        .join("ASUS")
        .join("ROG Live Service")
        .join("DeviceContent");
    for entry in std::fs::read_dir(&base).ok()?.flatten() {
        let model = entry.file_name();
        let model = model.to_string_lossy();
        let csv = base.join(model.as_ref()).join(format!("{model}_US_PERKEY.csv"));
        if csv.exists() {
            if let Ok(content) = std::fs::read_to_string(&csv) {
                if let Some(parsed) = parse_csv(&content) {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct Key {
    pub led: usize,
    pub name: String,
    pub row: u8,
    pub col: u8,
    pub scan: Option<u16>,
    /// Key rect, normalized to the keyboard bounding box (y: 0 top .. 1 bottom).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Key center in isotropic space: cx in 0..ASPECT, cy in 0..1, so that
    /// distances mean the same thing horizontally and vertically.
    pub cx: f32,
    pub cy: f32,
}

pub struct Layout {
    pub keys: Vec<Key>,
    pub aux: Vec<(usize, String)>,
    /// Rear light strip LEDs (chassis rear, under the lid logo), ordered
    /// left-to-right in lid space.
    pub rear: Vec<usize>,
    /// Width of isotropic space (height is 1.0).
    pub aspect: f32,
    /// key index by LED index (None for gaps / aux).
    pub key_of_led: Vec<Option<usize>>,
    /// key index by PS/2 scan code (with 0xE0-extended codes as 0x80|code).
    pub key_of_scan: HashMap<u16, usize>,
    /// Neighbor key indices (within ~1.35 key pitch), for graph effects.
    pub neighbors: Vec<Vec<usize>>,
}

/// Lid logo is 167 per OpenRGB/g-helper; 168 is mirrored as a safety net.
pub const LOGO_LEDS: [usize; 2] = [167, 168];
/// Front light bar, physical left → right.
pub const LIGHTBAR_LEDS: [usize; 6] = [174, 173, 172, 171, 170, 169];

impl Layout {
    pub fn load() -> Layout {
        // Prefer this machine's own vendor layout (so Keyscape adapts to any
        // ASUS ROG N-KEY laptop automatically); fall back to the bundled one.
        let (raw_keys, raw_aux, aspect) = match discover_and_parse() {
            Some((k, a, asp)) => {
                eprintln!("layout: auto-detected from ASUS DeviceContent ({} keys)", k.len());
                (k, a, asp)
            }
            None => {
                let raw: RawLayout = serde_json::from_str(include_str!("../assets/layout_us.json"))
                    .expect("embedded layout JSON is valid");
                let asp = raw.aspect.unwrap_or(2.47914);
                (raw.keys, raw.aux, asp)
            }
        };

        let keys: Vec<Key> = raw_keys
            .iter()
            .map(|k| Key {
                led: k.led as usize,
                name: k.name.clone(),
                row: k.row,
                col: k.col,
                scan: k.scan,
                x: k.x,
                y: k.y,
                w: k.w,
                h: k.h,
                cx: (k.x + k.w * 0.5) * aspect,
                cy: k.y + k.h * 0.5,
            })
            .collect();

        let mut key_of_led = vec![None; LED_COUNT];
        let mut key_of_scan = HashMap::new();
        for (i, k) in keys.iter().enumerate() {
            key_of_led[k.led] = Some(i);
            if let Some(s) = k.scan {
                key_of_scan.insert(s, i);
            }
        }

        // Average pitch = median horizontal center distance between adjacent
        // letters; ~0.16 in iso space. Neighbors = centers within 1.35 pitch.
        let pitch = 0.155_f32;
        let mut neighbors = vec![Vec::new(); keys.len()];
        for i in 0..keys.len() {
            for j in 0..keys.len() {
                if i == j {
                    continue;
                }
                let dx = keys[i].cx - keys[j].cx;
                let dy = keys[i].cy - keys[j].cy;
                if (dx * dx + dy * dy).sqrt() < pitch * 1.35 {
                    neighbors[i].push(j);
                }
            }
        }

        let mut rear: Vec<usize> = raw_aux
            .iter()
            .filter(|a| a.name.starts_with("Rear"))
            .map(|a| a.led as usize)
            .collect();
        rear.sort();

        Layout {
            keys,
            aux: raw_aux.iter().map(|a| (a.led as usize, a.name.clone())).collect(),
            rear,
            aspect,
            key_of_led,
            key_of_scan,
            neighbors,
        }
    }

    pub fn key_by_name(&self, name: &str) -> Option<&Key> {
        self.keys.iter().find(|k| k.name == name)
    }

    /// Resolve a named mask to a LED bitset. Unknown names resolve to "all".
    pub fn mask(&self, name: &str) -> LedMask {
        let mut m = LedMask::none();
        let mut add = |pred: &dyn Fn(&Key) -> bool| {
            for k in self.keys.iter().filter(|k| pred(k)) {
                m.set(k.led);
            }
        };
        match name {
            "letters" => add(&|k| k.name.len() == 1 && k.name.chars().next().unwrap().is_ascii_alphabetic()),
            "wasd" => add(&|k| matches!(k.name.as_str(), "W" | "A" | "S" | "D")),
            "arrows" => add(&|k| matches!(k.name.as_str(), "Up" | "Down" | "Left" | "Right")),
            "function" => add(&|k| k.name.starts_with('F') && k.name.len() <= 3 && k.name[1..].chars().all(|c| c.is_ascii_digit())),
            "numbers" => add(&|k| k.row == 2 && k.name.len() == 1 && k.name.chars().next().unwrap().is_ascii_digit()),
            "modifiers" => add(&|k| {
                matches!(
                    k.name.as_str(),
                    "LShift" | "RShift" | "LCtrl" | "RCtrl" | "LAlt" | "RAlt" | "Win" | "Fn" | "Caps" | "Tab" | "Enter" | "Backspace" | "Esc" | "Space"
                )
            }),
            "media" => add(&|k| {
                k.row == 0 || matches!(k.name.as_str(), "Play" | "Stop" | "Prev" | "Next")
            }),
            "main" => add(&|k| k.row >= 1 && !matches!(k.name.as_str(), "Play" | "Stop" | "Prev" | "Next")),
            "edge" => {
                // Perimeter of the main block: top F-row, bottom row, and the
                // leftmost/rightmost key of every row.
                add(&|k| k.row == 1 || k.row == 6);
                for row in 2..=5u8 {
                    let mut in_row: Vec<&Key> = self.keys.iter().filter(|k| k.row == row).collect();
                    in_row.sort_by(|a, b| a.cx.partial_cmp(&b.cx).unwrap());
                    if let Some(k) = in_row.first() {
                        m.set(k.led);
                    }
                    if let Some(k) = in_row.last() {
                        m.set(k.led);
                    }
                }
            }
            _ => {
                m = LedMask::all();
            }
        }
        m
    }

    pub fn mask_names() -> &'static [&'static str] {
        &["all", "main", "letters", "wasd", "arrows", "function", "numbers", "modifiers", "media", "edge"]
    }
}
