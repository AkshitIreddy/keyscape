//! Ambient / Calm — slow, quiet, low-stimulation scenes.

use super::*;
use crate::color::smoothstep;
use crate::math::{fbm3, noise2};
use crate::params::{get_f32, ParamSpec};

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "nebula_drift",
            name: "Nebula Drift",
            category: "Ambient",
            blurb: "Interstellar dust clouds slowly folding through the palette.",
            needs_input: false,
            default_palette: "ultraviolet",
            extras: || {
                vec![
                    ParamSpec::slider("scale", "Cloud scale", 0.5, 3.0, 0.1, 1.2),
                    ParamSpec::slider("contrast", "Contrast", 0.5, 2.5, 0.05, 1.4),
                ]
            },
            make: |_, seed| Box::new(Nebula { seed: seed as u32 }),
        },
        EffectInfo {
            id: "candlelight",
            name: "Candlelight",
            category: "Ambient",
            blurb: "Warm flame flicker with the occasional draft rolling across the desk.",
            needs_input: false,
            default_palette: "ember",
            extras: || vec![ParamSpec::slider("draftiness", "Draftiness", 0.0, 1.0, 0.05, 0.4)],
            make: |_, seed| Box::new(Candle { seed: seed as u32, gust: 0.0, gust_v: 0.0 }),
        },
    ]
}

// ---------------------------------------------------------------- Nebula

struct Nebula {
    seed: u32,
}

impl Effect for Nebula {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let scale = get_f32(ctx.params, "scale", 1.2);
        let contrast = get_f32(ctx.params, "contrast", 1.4);
        let t = ctx.t * 0.045;
        for k in &ctx.layout.keys {
            let n = fbm3(k.cx * scale, k.cy * scale, t, self.seed);
            // push midtones apart for cloud structure
            let shaped = ((n - 0.5) * contrast + 0.5).clamp(0.0, 1.0);
            // second noise field steers *where* in the palette we sample, so
            // hue structure decouples from brightness structure
            let hue = fbm3(k.cx * scale * 0.6 + 40.0, k.cy * scale * 0.6, t * 0.7, self.seed ^ 0xBEEF);
            let c = ctx.palette.sample(hue * 0.6 + ctx.t * 0.006);
            out.set(k.led, c.scale(0.15 + 0.85 * shaped * shaped));
        }
    }
}

// ---------------------------------------------------------------- Candle

struct Candle {
    seed: u32,
    gust: f32,
    gust_v: f32,
}

impl Effect for Candle {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let draft = get_f32(ctx.params, "draftiness", 0.4);

        // Occasional gusts: a spring pulled toward 0, randomly kicked.
        if ctx.rng.chance(ctx.dt * draft * 0.8) {
            self.gust_v -= ctx.rng.range(1.5, 4.0);
        }
        self.gust_v += (-self.gust * 6.0 - self.gust_v * 2.5) * ctx.dt;
        self.gust += self.gust_v * ctx.dt;
        let gust = (1.0 + self.gust * 0.35).clamp(0.35, 1.15);

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            // per-key flicker: two detuned noise streams read at different rates
            let f1 = noise2(i as f32 * 3.7, ctx.t * 1.9, self.seed);
            let f2 = noise2(i as f32 * 3.7 + 9.0, ctx.t * 6.3, self.seed ^ 0x55);
            // spatial coherence: the draft arrives as a wave from the left
            let wave = smoothstep(-0.4, 0.4, (ctx.t * 0.9).sin() - (k.cx - 1.2) * 0.3);
            let flick = (0.55 + 0.3 * f1 + 0.15 * f2) * (0.8 + 0.2 * wave) * gust;
            let c = ctx.palette.sample_clamped(0.25 + 0.55 * flick);
            out.set(k.led, c.scale(flick.clamp(0.05, 1.0)));
        }
    }
}
