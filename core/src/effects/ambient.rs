//! Ambient / Calm — slow, quiet, low-stimulation scenes.

use super::*;
use crate::color::{smoothstep, Col};
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
        EffectInfo {
            id: "zen_garden",
            name: "Zen Garden",
            category: "Ambient",
            blurb: "Raked sand circling stone keys; an invisible rake slowly redraws the pattern.",
            needs_input: false,
            default_palette: "copper",
            extras: || {
                vec![
                    ParamSpec::slider("spacing", "Line spacing", 0.08, 0.3, 0.01, 0.16),
                    ParamSpec::slider("redraw_sec", "Redraw every (s)", 15.0, 120.0, 5.0, 45.0),
                ]
            },
            make: |layout, seed| Box::new(ZenGarden::new(layout, seed)),
        },
        EffectInfo {
            id: "moon_phases",
            name: "Moon Phases",
            category: "Ambient",
            blurb: "A full lunar cycle sweeps its terminator across the board; stars own the dark side.",
            needs_input: false,
            default_palette: "mono",
            extras: || {
                vec![
                    ParamSpec::slider("cycle_sec", "Cycle length (s)", 20.0, 300.0, 10.0, 90.0),
                    ParamSpec::slider("stars", "Star density", 0.0, 1.0, 0.05, 0.5),
                ]
            },
            make: |_, seed| Box::new(MoonPhases { seed: seed as u32 }),
        },
        EffectInfo {
            id: "ink_water",
            name: "Ink in Water",
            category: "Ambient",
            blurb: "Ink drops bloom and diffuse through still dark water, each a different hue.",
            needs_input: false,
            default_palette: "ultraviolet",
            extras: || {
                vec![
                    ParamSpec::slider("drop_sec", "Drop every (s)", 2.0, 20.0, 0.5, 6.0),
                    ParamSpec::slider("diffusion", "Diffusion", 0.2, 3.0, 0.1, 1.2),
                ]
            },
            make: |layout, seed| {
                Box::new(InkWater {
                    seed: seed as u32,
                    conc: vec![0.0; layout.keys.len()],
                    hue: vec![0.0; layout.keys.len()],
                    next_drop: 0.0,
                })
            },
        },
        EffectInfo {
            id: "solar_sync",
            name: "Solar Sync",
            category: "Ambient",
            blurb: "The board follows your actual local time — dawn, day, dusk and starlit night.",
            needs_input: false,
            default_palette: "sunset",
            extras: || {
                vec![
                    ParamSpec::slider("offset_h", "Time offset (h)", -12.0, 12.0, 0.5, 0.0),
                    ParamSpec::toggle("demo", "Demo (day in 2 min)", false),
                ]
            },
            make: |_, seed| Box::new(SolarSync { seed: seed as u32 }),
        },
        EffectInfo {
            id: "deep_field",
            name: "Deep Field",
            category: "Ambient",
            blurb: "Parallax starfield adrift in nebula haze; once in a while, a slow comet.",
            needs_input: false,
            default_palette: "midnight",
            extras: || {
                vec![
                    ParamSpec::slider("stars", "Star density", 0.2, 2.0, 0.1, 1.0),
                    ParamSpec::slider("comet_sec", "Comet every (s)", 8.0, 90.0, 2.0, 30.0),
                ]
            },
            make: |_, seed| Box::new(DeepField { seed: seed as u32, comet_at: 6.0, comet_from_left: true }),
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

// ---------------------------------------------------------------- ZenGarden

struct ZenGarden {
    seed: u32,
    stones: Vec<(f32, f32)>,
    phase_a: f32,
    phase_b: f32,
    rake_x: f32,
    next_redraw: f32,
}

impl ZenGarden {
    fn new(layout: &crate::layout::Layout, seed: u64) -> ZenGarden {
        let mut rng = crate::math::Rng::new(seed);
        let stones = (0..3)
            .map(|_| {
                let k = &layout.keys[rng.below(layout.keys.len())];
                (k.cx, k.cy)
            })
            .collect();
        ZenGarden { seed: seed as u32, stones, phase_a: 0.0, phase_b: 0.0, rake_x: 99.0, next_redraw: 20.0 }
    }

    /// Groove brightness at a point: circular rake lines near stones blend
    /// into straight horizontal lines in open sand.
    fn pattern(&self, x: f32, y: f32, phase: f32, spacing: f32) -> f32 {
        let mut ring = 0.0_f32;
        let mut weight = 0.0_f32;
        for (sx, sy) in &self.stones {
            let d = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
            let w = smoothstep(spacing * 4.5, spacing * 1.5, d);
            let g = ((d / spacing + phase) * std::f32::consts::TAU).sin().abs();
            ring += g * w;
            weight += w;
        }
        let flat = (((y * 0.85 + x * 0.06) / spacing + phase * 2.3) * std::f32::consts::TAU).sin().abs();
        let w = weight.min(1.0);
        (1.0 - flat) * (1.0 - w) + (if weight > 0.0 { ring / weight.max(1.0) } else { 0.0 }) * w
    }
}

impl Effect for ZenGarden {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let spacing = get_f32(ctx.params, "spacing", 0.16);
        let redraw = get_f32(ctx.params, "redraw_sec", 45.0);

        if ctx.t >= self.next_redraw && self.rake_x > ctx.layout.aspect + 0.5 {
            self.rake_x = -0.4;
            self.phase_b = ctx.rng.f32();
            self.next_redraw = ctx.t + redraw.max(10.0);
        }
        let raking = self.rake_x <= ctx.layout.aspect + 0.5;
        if raking {
            self.rake_x += ctx.dt * 0.22;
            if self.rake_x > ctx.layout.aspect + 0.5 {
                self.phase_a = self.phase_b;
            }
        }

        for k in &ctx.layout.keys {
            let old = self.pattern(k.cx, k.cy, self.phase_a, spacing);
            let v = if raking {
                let mix = smoothstep(self.rake_x - 0.12, self.rake_x + 0.12, k.cx);
                let new = self.pattern(k.cx, k.cy, self.phase_b, spacing);
                new * (1.0 - mix) + old * mix
            } else {
                old
            };

            // stones sit dark with a faint warm rim
            let mut stone = 0.0_f32;
            for (sx, sy) in &self.stones {
                let d = ((k.cx - sx).powi(2) + (k.cy - sy).powi(2)).sqrt();
                stone = stone.max(smoothstep(spacing * 1.2, spacing * 0.3, d));
            }

            let grain = 0.92 + 0.08 * noise2(k.cx * 14.0, k.cy * 14.0, self.seed);
            let sand = ctx.palette.sample_clamped(0.22 + 0.5 * v).scale((0.18 + 0.72 * v) * grain);
            let rim = ctx.palette.sample_clamped(0.85).scale(0.25);
            let mut c = Col::lerp(sand, rim, stone * 0.8);
            if raking {
                let front = smoothstep(0.14, 0.0, (k.cx - self.rake_x).abs());
                c = c.add(ctx.palette.sample_clamped(0.9).scale(front * 0.35));
            }
            out.set(k.led, c);
        }
    }
}

// ---------------------------------------------------------------- MoonPhases

struct MoonPhases {
    seed: u32,
}

impl Effect for MoonPhases {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let cycle = get_f32(ctx.params, "cycle_sec", 90.0).max(10.0);
        let stars = get_f32(ctx.params, "stars", 0.5);
        let p = (ctx.t / cycle).fract();
        let waxing = p < 0.5;
        // fraction of the disc that is lit, for star visibility
        let vis = 1.0 - (p - 0.5).abs() * 2.0; // 0 new .. 1 full
        let mx = ctx.layout.aspect * 0.5 + 0.15 * (ctx.t * 0.02).sin();
        let my = 0.5;
        let r = 0.44;

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let dx = (k.cx - mx) / r;
            let dy = (k.cy - my) / r;
            let rr = dx * dx + dy * dy;
            if rr <= 1.15 {
                let w = (1.0 - dy * dy).max(0.0).sqrt();
                let s = if waxing {
                    dx - (p * std::f32::consts::TAU).cos() * w
                } else {
                    ((p - 0.5) * std::f32::consts::TAU).cos() * w - dx
                };
                let lit = smoothstep(-0.12, 0.12, s) * smoothstep(1.1, 0.95, rr);
                let mare = 0.75 + 0.25 * fbm3(dx * 1.8, dy * 1.8, 0.0, self.seed);
                let c = ctx.palette.sample_clamped(0.45 + 0.4 * lit);
                out.set(k.led, c.scale((0.05 + 0.95 * lit * mare).min(1.0)));
            } else {
                let n = noise2(k.cx * 9.0, k.cy * 9.0, self.seed ^ 0x51A2);
                let star = smoothstep(0.97 - stars * 0.06, 1.0, n);
                let tw = 0.5 + 0.5 * noise2(i as f32 * 7.3, ctx.t * 1.1, self.seed);
                let b = star * tw * (0.25 + 0.75 * vis);
                let sky = ctx.palette.sample_clamped(0.08).scale(0.05);
                out.set(k.led, sky.add(ctx.palette.sample_clamped(0.95).scale(b * 0.8)));
            }
        }
    }
}

// ---------------------------------------------------------------- InkWater

struct InkWater {
    seed: u32,
    conc: Vec<f32>,
    hue: Vec<f32>,
    next_drop: f32,
}

impl Effect for InkWater {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let drop_every = get_f32(ctx.params, "drop_sec", 6.0);
        let diff = get_f32(ctx.params, "diffusion", 1.2);
        let n_keys = ctx.layout.keys.len();

        if ctx.t >= self.next_drop {
            let k0 = ctx.rng.below(n_keys);
            self.conc[k0] += 2.8;
            self.hue[k0] = ctx.rng.f32();
            for &nb in &ctx.layout.neighbors[k0] {
                self.conc[nb] += 0.5;
                self.hue[nb] = self.hue[k0];
            }
            self.next_drop = ctx.t + drop_every * ctx.rng.range(0.6, 1.5);
        }

        // graph diffusion + slow downward advection + decay
        let dt = ctx.dt.min(0.08);
        let old: Vec<f32> = self.conc.clone();
        for i in 0..n_keys {
            let nbs = &ctx.layout.neighbors[i];
            if nbs.is_empty() {
                continue;
            }
            let mut sum = 0.0;
            let mut hue_num = 0.0;
            let mut hue_den = 0.0;
            for &j in nbs {
                sum += old[j] - old[i];
                if old[j] > old[i] {
                    hue_num += self.hue[j] * old[j];
                    hue_den += old[j];
                }
            }
            self.conc[i] += dt * diff * 0.9 * sum / nbs.len() as f32;
            if hue_den > 0.01 {
                let target = hue_num / hue_den;
                self.hue[i] += (target - self.hue[i]) * (dt * diff * 0.8).min(1.0);
            }
            // ink is heavier than water: bleed a little toward lower keys
            let cy = ctx.layout.keys[i].cy;
            for &j in nbs {
                if ctx.layout.keys[j].cy > cy + 0.04 && old[i] > 0.02 {
                    let move_amt = old[i] * dt * 0.10;
                    self.conc[i] -= move_amt;
                    self.conc[j] += move_amt;
                }
            }
            self.conc[i] = (self.conc[i] * (1.0 - dt * 0.045)).max(0.0);
        }

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let c = self.conc[i];
            let density = 1.0 - (-c * 1.4).exp();
            let ink = ctx.palette.sample(self.hue[i]).scale(density);
            let shimmer = 0.015 + 0.01 * noise2(k.cx * 3.0, k.cy * 3.0 + ctx.t * 0.15, self.seed);
            let water = ctx.palette.sample_clamped(0.5).scale(shimmer);
            out.set(k.led, water.add(ink));
        }
    }
}

// ---------------------------------------------------------------- SolarSync

struct SolarSync {
    seed: u32,
}

/// (hour, zenith color, horizon color) keyframes for the sky dome.
const SKY: [(f32, u32, u32); 9] = [
    (0.0, 0x010208, 0x060a1c),
    (4.5, 0x050514, 0x1a1030),
    (6.0, 0x1e2a55, 0xc85a32),
    (7.5, 0x4a78c8, 0xf0b478),
    (12.0, 0x64a0e6, 0xb4d8f0),
    (16.5, 0x5a8cd2, 0xd2b478),
    (18.5, 0x2a3264, 0xe6643c),
    (20.0, 0x0a0e2a, 0x321e46),
    (24.0, 0x010208, 0x060a1c),
];

impl Effect for SolarSync {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let offset = get_f32(ctx.params, "offset_h", 0.0);
        let demo = crate::params::get_bool(ctx.params, "demo", false);

        let hour = if demo {
            (ctx.t / 120.0).fract() * 24.0
        } else {
            let mut st = windows_sys::Win32::Foundation::SYSTEMTIME {
                wYear: 0, wMonth: 0, wDayOfWeek: 0, wDay: 0,
                wHour: 0, wMinute: 0, wSecond: 0, wMilliseconds: 0,
            };
            unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st) };
            (st.wHour as f32 + st.wMinute as f32 / 60.0 + st.wSecond as f32 / 3600.0 + offset)
                .rem_euclid(24.0)
        };

        // interpolate sky keyframes
        let mut top = Col::hex(SKY[0].1);
        let mut bot = Col::hex(SKY[0].2);
        for w in SKY.windows(2) {
            let (h0, t0, b0) = w[0];
            let (h1, t1, b1) = w[1];
            if hour >= h0 && hour <= h1 {
                let f = (hour - h0) / (h1 - h0).max(0.01);
                top = Col::lerp(Col::hex(t0), Col::hex(t1), f);
                bot = Col::lerp(Col::hex(b0), Col::hex(b1), f);
                break;
            }
        }

        // sun arc 6h -> 18h; moon arcs the night
        let day = hour >= 6.0 && hour <= 18.0;
        let (bx, by, warm) = if day {
            let f = (hour - 6.0) / 12.0;
            let elev = (f * std::f32::consts::PI).sin();
            (f * ctx.layout.aspect, 1.05 - 0.95 * elev, true)
        } else {
            let f = ((hour + 6.0) % 24.0 / 12.0).fract();
            let elev = (f * std::f32::consts::PI).sin();
            (f * ctx.layout.aspect, 1.05 - 0.85 * elev, false)
        };

        let night_amt = smoothstep(7.5, 4.5, hour) + smoothstep(18.5, 21.5, hour);
        for k in &ctx.layout.keys {
            let mut c = Col::lerp(top, bot, k.cy).scale(0.9);
            // palette tint keeps themes meaningful
            c = Col::lerp(c, ctx.palette.sample(hour / 24.0), 0.18);
            let d2 = (k.cx - bx).powi(2) + (k.cy - by).powi(2);
            let glow = (-d2 * 9.0).exp();
            let body = if warm {
                Col::hex(0xFFE8B4).scale(glow * 1.1)
            } else {
                Col::hex(0xC8D8F0).scale(glow * 0.5)
            };
            c = c.add(body);
            if night_amt > 0.05 {
                let n = noise2(k.cx * 9.0 + 30.0, k.cy * 9.0, self.seed);
                let star = smoothstep(0.955, 1.0, n)
                    * (0.5 + 0.5 * noise2(k.cx * 40.0, ctx.t * 1.2, self.seed))
                    * night_amt.min(1.0);
                c = c.add(Col::hex(0xE6EEFF).scale(star * 0.7));
            }
            out.set(k.led, c);
        }
    }
}

// ---------------------------------------------------------------- DeepField

struct DeepField {
    seed: u32,
    comet_at: f32,
    comet_from_left: bool,
}

impl Effect for DeepField {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let density = get_f32(ctx.params, "stars", 1.0);
        let comet_every = get_f32(ctx.params, "comet_sec", 30.0);

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            // faint nebula haze
            let haze = fbm3(k.cx * 0.9, k.cy * 0.9, ctx.t * 0.012, self.seed);
            let mut c = ctx.palette.sample(0.25 + haze * 0.2).scale(0.045 * haze);

            // three parallax star layers, deeper = slower + dimmer
            for layer in 0..3u32 {
                let speed = 0.006 * (layer + 1) as f32 * (layer + 1) as f32;
                let freq = 6.0 + 3.0 * layer as f32;
                let n = noise2(k.cx * freq + ctx.t * speed * freq, k.cy * freq, self.seed + layer * 977);
                let th = 0.965 - 0.025 * density.min(1.6);
                let star = smoothstep(th, 1.0, n);
                if star > 0.0 {
                    let tw = 0.55 + 0.45 * noise2(i as f32 * 5.1 + layer as f32 * 31.0, ctx.t * (1.0 + layer as f32 * 0.6), self.seed);
                    let col = ctx.palette.sample_clamped(0.6 + 0.13 * layer as f32);
                    c = c.add(col.scale(star * tw * (0.25 + 0.3 * (3 - layer) as f32)));
                }
            }
            out.set(k.led, c);
        }

        // the slow comet
        let dur = 14.0;
        if ctx.t >= self.comet_at {
            let f = (ctx.t - self.comet_at) / dur;
            if f > 1.0 {
                self.comet_at = ctx.t + comet_every * ctx.rng.range(0.7, 1.4);
                self.comet_from_left = ctx.rng.chance(0.5);
            } else {
                let dir = if self.comet_from_left { 1.0 } else { -1.0 };
                let x = if self.comet_from_left { -0.2 } else { ctx.layout.aspect + 0.2 } + dir * f * (ctx.layout.aspect + 0.4);
                let y = 0.25 + 0.5 * noise2(self.comet_at, 3.7, self.seed) + 0.12 * (f * 5.0).sin();
                for k in &ctx.layout.keys {
                    // head
                    let d2 = (k.cx - x).powi(2) + (k.cy - y).powi(2);
                    let head = (-d2 * 55.0).exp();
                    // tail stretches behind the direction of travel
                    let bx = (k.cx - x) * dir;
                    let tail = if bx < 0.0 {
                        (-(bx.abs() / 0.7)).exp() * (-((k.cy - y).powi(2)) * 38.0).exp() * 0.5
                    } else {
                        0.0
                    };
                    let col = ctx.palette.sample_clamped(0.9);
                    out.max(k.led, col.scale((head * 1.2 + tail).min(1.0)));
                }
            }
        }
    }
}
