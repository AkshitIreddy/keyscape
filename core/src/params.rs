//! Effect parameter model. Params travel as JSON objects (IPC-friendly);
//! `ParamSpec` describes each control so the UI can render editors without
//! hardcoding anything per effect.

use serde::Serialize;
use serde_json::{Map, Value};

pub type Params = Map<String, Value>;

#[derive(Clone, Serialize)]
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    /// "slider" | "toggle" | "select" | "palette" | "mask" | "color"
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f32>,
    pub default: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<&'static str>,
}

impl ParamSpec {
    pub fn slider(key: &'static str, label: &'static str, min: f32, max: f32, step: f32, default: f32) -> ParamSpec {
        ParamSpec {
            key,
            label,
            kind: "slider",
            min: Some(min),
            max: Some(max),
            step: Some(step),
            default: Value::from(default),
            options: vec![],
        }
    }

    pub fn toggle(key: &'static str, label: &'static str, default: bool) -> ParamSpec {
        ParamSpec { key, label, kind: "toggle", min: None, max: None, step: None, default: Value::from(default), options: vec![] }
    }

    pub fn select(key: &'static str, label: &'static str, options: Vec<&'static str>, default: &str) -> ParamSpec {
        ParamSpec { key, label, kind: "select", min: None, max: None, step: None, default: Value::from(default), options }
    }

    pub fn palette(default: &str) -> ParamSpec {
        ParamSpec { key: "palette", label: "Palette", kind: "palette", min: None, max: None, step: None, default: Value::from(default), options: vec![] }
    }

    pub fn mask() -> ParamSpec {
        ParamSpec {
            key: "mask",
            label: "Keys",
            kind: "mask",
            min: None,
            max: None,
            step: None,
            default: Value::from("all"),
            options: crate::layout::Layout::mask_names().to_vec(),
        }
    }
}

/// The params every effect shares. Effects read extras themselves via the
/// typed getters below.
pub fn common_specs(default_palette: &str) -> Vec<ParamSpec> {
    vec![
        ParamSpec::slider("speed", "Speed", 0.1, 3.0, 0.05, 1.0),
        ParamSpec::slider("intensity", "Intensity", 0.05, 1.0, 0.05, 1.0),
        ParamSpec::palette(default_palette),
        ParamSpec::mask(),
    ]
}

pub fn get_f32(p: &Params, key: &str, default: f32) -> f32 {
    p.get(key).and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(default)
}

pub fn get_bool(p: &Params, key: &str, default: bool) -> bool {
    p.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub fn get_str<'a>(p: &'a Params, key: &str, default: &'a str) -> &'a str {
    p.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

/// Fill in defaults from specs for any keys the stored params don't have.
pub fn with_defaults(stored: &Params, specs: &[ParamSpec]) -> Params {
    let mut out = stored.clone();
    for s in specs {
        out.entry(s.key.to_string()).or_insert_with(|| s.default.clone());
    }
    out
}
