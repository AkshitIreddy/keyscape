//! Organic / Nature — living systems: deep ocean, meadow, storm, reef, vine.
//!
//! Every effect here layers several motion timescales (slow fields, mid-speed
//! drift, fast accents) so the board reads as alive rather than animated.

use super::*;
use crate::color::{smoothstep, Col};
use crate::math::{fbm3, noise2};
use crate::params::get_f32;
use std::f32::consts::TAU;

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "bioluminescence",
            name: "Bioluminescence",
            category: "Organic",
            blurb: "Invisible currents stir plankton awake in a pitch-black sea.",
            needs_input: false,
            default_palette: "oceanic",
            extras: || {
                vec![
                    ParamSpec::slider("density", "Plankton density", 0.2, 2.0, 0.05, 1.0),
                    ParamSpec::slider("pulses", "Pulse rate", 0.1, 2.0, 0.05, 0.8),
                ]
            },
            make: |layout, seed| {
                let n = layout.keys.len();
                Box::new(Bioluminescence {
                    seed: seed as u32,
                    charge: vec![0.0; n],
                    mote: vec![0.0; n],
                    scratch: vec![0.0; n],
                })
            },
        },
        EffectInfo {
            id: "firefly_meadow",
            name: "Firefly Meadow",
            category: "Organic",
            blurb: "Wandering fireflies slowly fall into blinking in unison, then drift apart.",
            needs_input: false,
            default_palette: "forest",
            extras: || {
                vec![
                    ParamSpec::slider("fireflies", "Fireflies", 6.0, 30.0, 1.0, 18.0),
                    ParamSpec::slider("sync", "Sync strength", 0.0, 1.0, 0.05, 0.6),
                ]
            },
            make: |_, seed| {
                let mut rng = Rng::new(seed);
                let flies = (0..FLY_MAX)
                    .map(|_| Firefly {
                        phase: rng.range(0.0, TAU),
                        omega: rng.range(1.3, 2.1),
                        ox: rng.range(0.0, 100.0),
                        oy: rng.range(0.0, 100.0),
                    })
                    .collect();
                Box::new(FireflyMeadow { seed: seed as u32, flies })
            },
        },
        EffectInfo {
            id: "aurora_veil",
            name: "Aurora Veil",
            category: "Organic",
            blurb: "Swaying light curtains with a shimmering lower border and rare surges.",
            needs_input: false,
            default_palette: "aurora",
            extras: || {
                vec![
                    ParamSpec::slider("curtains", "Curtain count", 1.0, 6.0, 0.5, 3.0),
                    ParamSpec::slider("shimmer", "Shimmer", 0.0, 1.0, 0.05, 0.6),
                ]
            },
            make: |_, seed| {
                Box::new(AuroraVeil {
                    seed: seed as u32,
                    surge_age: 100.0,
                    next_surge: 6.0,
                    surge_dir: 1.0,
                })
            },
        },
        EffectInfo {
            id: "thunderstorm",
            name: "Thunderstorm",
            category: "Organic",
            blurb: "Dim rain and distant rumbles, split by forked lightning down the keys.",
            needs_input: false,
            default_palette: "midnight",
            extras: || {
                vec![
                    ParamSpec::slider("storm", "Storm intensity", 0.0, 1.0, 0.05, 0.6),
                    ParamSpec::slider("strikes", "Strike rate", 0.2, 2.5, 0.05, 1.0),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let first = rng.range(2.0, 6.0);
                Box::new(Thunderstorm {
                    seed: seed as u32,
                    drops: vec![0.0; layout.keys.len()],
                    bolt: Vec::new(),
                    bolt_age: 100.0,
                    next_strike: first,
                    bleach: false,
                })
            },
        },
        EffectInfo {
            id: "ocean_tide",
            name: "Ocean Tide",
            category: "Organic",
            blurb: "Foam-capped swells roll in while the tide slowly claims and frees the shore.",
            needs_input: false,
            default_palette: "oceanic",
            extras: || {
                vec![
                    ParamSpec::slider("wave_scale", "Wave scale", 0.5, 2.5, 0.05, 1.0),
                    dir_param("right"),
                ]
            },
            make: |_, seed| Box::new(OceanTide { seed: seed as u32 }),
        },
        EffectInfo {
            id: "ivy_growth",
            name: "Ivy Growth",
            category: "Organic",
            blurb: "A vine creeps across the board, blooms, turns with the season and falls.",
            needs_input: false,
            default_palette: "forest",
            extras: || {
                vec![
                    ParamSpec::slider("growth", "Growth rate", 0.3, 3.0, 0.05, 1.0),
                    ParamSpec::slider("bloom", "Bloom amount", 0.0, 1.0, 0.05, 0.6),
                ]
            },
            make: |layout, seed| {
                let n = layout.keys.len();
                let mut ivy = IvyGrowth {
                    seed: seed as u32,
                    rng: Rng::new(seed),
                    covered: vec![false; n],
                    cover_t: vec![0.0; n],
                    bud: vec![-1.0; n],
                    fall_at: vec![0.0; n],
                    tips: Vec::new(),
                    acc: 0.0,
                    clock: 0.0,
                    autumn: false,
                    autumn_t: 0.0,
                    n_covered: 0,
                };
                ivy.reseed(layout);
                Box::new(ivy)
            },
        },
        EffectInfo {
            id: "coral_reef",
            name: "Coral Reef",
            category: "Organic",
            blurb: "Slow-breathing coral heads with tiny fish darting through the channels.",
            needs_input: false,
            default_palette: "miami",
            extras: || {
                vec![
                    ParamSpec::slider("patches", "Coral patches", 4.0, 8.0, 1.0, 6.0),
                    ParamSpec::slider("fish", "Fish", 1.0, 5.0, 1.0, 3.0),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let nk = layout.keys.len();
                let fish = (0..FISH_MAX)
                    .map(|_| Fish {
                        from: rng.below(nk),
                        to: rng.below(nk),
                        prog: 1.0,
                        dur: 0.4,
                        pause: rng.range(0.3, 1.5),
                    })
                    .collect();
                Box::new(CoralReef { seed: seed as u32, fish })
            },
        },
        EffectInfo {
            id: "pollen_drift",
            name: "Pollen Drift",
            category: "Organic",
            blurb: "Pollen riding a curling breeze, dusting the keys wherever it lingers.",
            needs_input: false,
            default_palette: "sakura",
            extras: || {
                vec![
                    ParamSpec::slider("particles", "Pollen count", 8.0, 40.0, 1.0, 25.0),
                    ParamSpec::slider("wind", "Wind strength", 0.3, 2.5, 0.05, 1.0),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let grains = (0..POLLEN_MAX)
                    .map(|_| Grain {
                        x: rng.range(0.0, layout.aspect),
                        y: rng.range(0.0, 1.0),
                        vx: 0.0,
                        vy: 0.0,
                        tw: rng.range(0.0, 50.0),
                    })
                    .collect();
                Box::new(PollenDrift {
                    seed: seed as u32,
                    grains,
                    glow: vec![0.0; layout.keys.len()],
                    gust_x: 0.0,
                    gust_y: 0.0,
                    next_gust: rng.range(4.0, 10.0),
                    warp: 0.0,
                })
            },
        },
    ]
}

// ------------------------------------------------------- Bioluminescence

struct Bioluminescence {
    seed: u32,
    /// Plankton excitation per key: charges as a current band passes, fades.
    charge: Vec<f32>,
    /// Diffusing mote-pulse field over the neighbor graph.
    mote: Vec<f32>,
    scratch: Vec<f32>,
}

impl Effect for Bioluminescence {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let density = get_f32(ctx.params, "density", 1.0);
        let pulses = get_f32(ctx.params, "pulses", 0.8);
        let t = ctx.t;
        let dt = ctx.dt;
        let n = ctx.layout.keys.len();

        // Two invisible current sheets drift at different headings; where
        // their sum crests, plankton charge up quickly and fade slowly.
        let th = 0.64 - 0.10 * density;
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let b1 = noise2(k.cx * 1.5 - t * 0.16, k.cy * 1.5 + t * 0.045, self.seed);
            let b2 = noise2(k.cx * 2.3 + t * 0.11, k.cy * 2.3 - t * 0.07, self.seed ^ 0x9D2C);
            let field = b1 * 0.62 + b2 * 0.38;
            let excite = smoothstep(th, th + 0.14, field);
            let c = &mut self.charge[i];
            if excite > *c {
                *c += (excite - *c) * (1.0 - (-dt * 5.0).exp());
            } else {
                *c *= (-dt * 0.7).exp();
            }
        }

        // Mote pulses: one plankter fires bright and its light diffuses
        // outward along the neighbor graph over roughly half a second.
        if ctx.rng.chance((dt * pulses * 0.5).min(1.0)) {
            let i = ctx.rng.below(n);
            self.mote[i] += 1.4;
        }
        self.scratch.copy_from_slice(&self.mote);
        let spread = (dt * 3.2).min(0.45);
        for i in 0..n {
            let give = self.scratch[i] * spread;
            if give > 1e-4 {
                let nb = &ctx.layout.neighbors[i];
                if !nb.is_empty() {
                    self.mote[i] -= give;
                    let share = give * 0.8 / nb.len() as f32;
                    for &j in nb {
                        self.mote[j] += share;
                    }
                }
            }
        }
        for m in self.mote.iter_mut() {
            *m *= (-dt * 2.4).exp();
        }

        // Compose: abyssal near-black, drifting plankton glow, mote flashes.
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let hue = 0.42 + 0.3 * noise2(k.cx * 0.9 + 31.0, k.cy * 0.9 + t * 0.025, self.seed ^ 0x77);
            let tw = 0.7 + 0.3 * noise2(i as f32 * 5.3, t * 2.6, self.seed ^ 0xC1);
            let glow = self.charge[i].powf(1.6) * tw;
            let mut col = ctx.palette.sample_clamped(0.04).scale(0.035);
            col = col.add(ctx.palette.sample_clamped(hue).scale(glow * 0.9));
            let m = self.mote[i].min(1.2);
            if m > 3e-3 {
                col = col.add(ctx.palette.sample_clamped(0.92).scale(m));
            }
            out.set(k.led, col);
        }
    }
}

// -------------------------------------------------------- Firefly Meadow

const FLY_MAX: usize = 30;

struct Firefly {
    phase: f32,
    omega: f32,
    ox: f32,
    oy: f32,
}

struct FireflyMeadow {
    seed: u32,
    flies: Vec<Firefly>,
}

impl Effect for FireflyMeadow {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let n = (get_f32(ctx.params, "fireflies", 18.0).round() as usize).clamp(6, FLY_MAX);
        let sync = get_f32(ctx.params, "sync", 0.6);
        let t = ctx.t;
        let dt = ctx.dt;
        let aspect = ctx.layout.aspect;

        // Kuramoto coupling waxes over ~half a minute, holds while the meadow
        // blinks in unison, then releases and the phases drift apart again.
        let cyc = (t / 48.0).fract();
        let coupling =
            sync * 3.0 * smoothstep(0.06, 0.42, cyc) * (1.0 - smoothstep(0.72, 0.95, cyc));

        let (mut sx, mut sy) = (0.0f32, 0.0f32);
        for f in self.flies[..n].iter() {
            sx += f.phase.cos();
            sy += f.phase.sin();
        }
        let r = (sx * sx + sy * sy).sqrt() / n as f32;
        let psi = sy.atan2(sx);
        for f in &mut self.flies[..n] {
            f.phase += (f.omega + coupling * r * (psi - f.phase).sin()) * dt;
            if f.phase > TAU {
                f.phase -= TAU;
            }
        }

        // Dim moonlit grass so the dark isn't dead; slightly denser low down.
        for k in &ctx.layout.keys {
            let g = fbm3(k.cx * 1.6, k.cy * 1.6, t * 0.02, self.seed);
            let sway = 0.8 + 0.2 * noise2(k.cx * 2.0 - t * 0.15, k.cy * 4.0, self.seed ^ 0x33);
            let b = (0.03 + 0.05 * g * k.cy) * sway;
            out.set(k.led, ctx.palette.sample_clamped(0.06 + 0.12 * g).scale(b));
        }

        // Fireflies: smooth noise wander, sharp phase-gated blink, faint
        // ember body between blinks so you can track each one.
        for f in self.flies[..n].iter() {
            let px = aspect
                * (((noise2(t * 0.055 + f.ox, f.ox * 1.7, self.seed ^ 0x5A) - 0.5) * 1.7 + 0.5)
                    .clamp(0.02, 0.98));
            let py = (((noise2(t * 0.05 + f.oy, f.oy * 2.3, self.seed ^ 0xA5) - 0.5) * 1.7) + 0.5)
                .clamp(0.02, 0.98);
            let w = 0.5 + 0.5 * f.phase.sin();
            let blink = w.powf(14.0);
            let body = 0.05 + 0.95 * blink;
            let rad = 0.16 + 0.06 * blink;
            let c = ctx.palette.sample_clamped(0.78 + 0.14 * blink);
            let r2 = rad * rad;
            for k in &ctx.layout.keys {
                let dx = k.cx - px;
                let dy = k.cy - py;
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let fall = (1.0 - d2 / r2).powi(2);
                out.add(k.led, c.scale(body * fall));
            }
        }
    }
}

// ----------------------------------------------------------- Aurora Veil

struct AuroraVeil {
    seed: u32,
    surge_age: f32,
    next_surge: f32,
    surge_dir: f32,
}

impl Effect for AuroraVeil {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let curtains = get_f32(ctx.params, "curtains", 3.0);
        let shimmer = get_f32(ctx.params, "shimmer", 0.6);
        let t = ctx.t;
        let aspect = ctx.layout.aspect;

        self.next_surge -= ctx.dt;
        self.surge_age += ctx.dt;
        if self.next_surge <= 0.0 {
            self.next_surge = ctx.rng.range(9.0, 20.0);
            self.surge_age = 0.0;
            self.surge_dir = if ctx.rng.chance(0.5) { 1.0 } else { -1.0 };
        }
        let surge_dur = 2.4;
        let surging = self.surge_age < surge_dur;
        let sx = if self.surge_dir > 0.0 {
            -0.5 + (self.surge_age / surge_dur) * (aspect + 1.0)
        } else {
            aspect + 0.5 - (self.surge_age / surge_dur) * (aspect + 1.0)
        };
        let surge_env = smoothstep(0.0, 0.25, self.surge_age)
            * (1.0 - smoothstep(surge_dur - 0.5, surge_dur, self.surge_age));

        for k in &ctx.layout.keys {
            // Rays sway; the bottom of the curtain swings farther than the top.
            let sway = (noise2(k.cy * 1.2 + t * 0.1, t * 0.06, self.seed ^ 0xA1) - 0.5)
                * 0.6
                * (0.35 + 0.65 * k.cy);
            let x = k.cx + sway;
            // Layered 1/f ray brightness across x.
            let ray = fbm3(x * curtains * 0.9, 7.3, t * 0.055, self.seed);
            let rays = smoothstep(0.32, 0.72, ray);
            // Hanging bottom edge, different per ray column.
            let edge = 0.5 + 0.68 * (noise2(x * 1.15, t * 0.08, self.seed ^ 0x3B) - 0.5);
            let above = 1.0 - smoothstep(edge - 0.04, edge + 0.12, k.cy);
            let de = (k.cy - edge) * 5.5;
            let border = (-de * de).exp();
            // High-frequency shimmer concentrated near the lower border.
            let sh = noise2(x * 7.0, t * 6.5, self.seed ^ 0x55);
            let shim = 1.0 + shimmer * (sh - 0.5) * 2.0 * (0.25 + 0.75 * border);
            let height = (k.cy / edge.max(0.05)).clamp(0.0, 1.0);
            let fill = above * (0.25 + 0.75 * height * height);
            let mut b = rays * (0.5 * fill + 0.9 * border) * shim;
            if surging {
                let gx = (k.cx - sx) / 0.45;
                b += (-gx * gx).exp() * surge_env * (0.25 + 0.75 * rays) * 0.8;
            }
            // Palette-relative hue: low near the glowing border, climbing
            // toward the far end overhead, with a very slow global drift.
            let hue = 0.7 - 0.45 * height + 0.08 * (ray - 0.5) + t * 0.004;
            let col = ctx
                .palette
                .sample(hue)
                .scale(b.min(1.4))
                .add(ctx.palette.sample_clamped(0.05).scale(0.018));
            out.set(k.led, col);
        }
    }
}

// ---------------------------------------------------------- Thunderstorm

struct Thunderstorm {
    seed: u32,
    drops: Vec<f32>,
    /// Forked bolt as (key index, branch intensity) pairs.
    bolt: Vec<(usize, f32)>,
    bolt_age: f32,
    next_strike: f32,
    bleach: bool,
}

impl Thunderstorm {
    fn build_bolt(&mut self, layout: &Layout, rng: &mut Rng) {
        self.bolt.clear();
        let tops: Vec<usize> = layout
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.cy < 0.25)
            .map(|(i, _)| i)
            .collect();
        if tops.is_empty() {
            return;
        }
        let mut cur = tops[rng.below(tops.len())];
        self.bolt.push((cur, 1.0));
        let mut branches: Vec<usize> = Vec::new();
        for _ in 0..14 {
            let down: Vec<usize> = layout.neighbors[cur]
                .iter()
                .copied()
                .filter(|&j| layout.keys[j].cy > layout.keys[cur].cy + 0.02)
                .collect();
            if down.is_empty() {
                break;
            }
            let nxt = down[rng.below(down.len())];
            if down.len() > 1 && rng.chance(0.35) {
                let alt = down[rng.below(down.len())];
                if alt != nxt {
                    branches.push(alt);
                }
            }
            self.bolt.push((nxt, 1.0));
            cur = nxt;
            if layout.keys[cur].cy > 0.92 {
                break;
            }
        }
        // Side branches: dimmer, wander a couple of keys further down.
        for start in branches {
            let mut c = start;
            let amp = 0.55;
            self.bolt.push((c, amp));
            for step in 0..3 {
                let down: Vec<usize> = layout.neighbors[c]
                    .iter()
                    .copied()
                    .filter(|&j| layout.keys[j].cy > layout.keys[c].cy + 0.02)
                    .collect();
                if down.is_empty() || rng.chance(0.3) {
                    break;
                }
                c = down[rng.below(down.len())];
                self.bolt.push((c, amp * (0.85 - 0.15 * step as f32)));
            }
        }
    }
}

impl Effect for Thunderstorm {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let storm = get_f32(ctx.params, "storm", 0.6);
        let rate = get_f32(ctx.params, "strikes", 1.0);
        let t = ctx.t;
        let dt = ctx.dt;
        let nk = ctx.layout.keys.len();

        // Sparse rain: brief per-key flickers that decay fast.
        for d in self.drops.iter_mut() {
            *d *= (-dt * 7.0).exp();
        }
        let spawn = dt * (1.5 + 9.0 * storm);
        for _ in 0..2 {
            if ctx.rng.chance((spawn * 0.5).min(1.0)) {
                let i = ctx.rng.below(nk);
                self.drops[i] = self.drops[i].max(ctx.rng.range(0.4, 1.0));
            }
        }

        // Strike scheduling.
        self.next_strike -= dt;
        self.bolt_age += dt;
        if self.next_strike <= 0.0 {
            self.next_strike = ctx.rng.range(4.0, 10.0) / rate.max(0.1);
            self.build_bolt(ctx.layout, ctx.rng);
            self.bolt_age = 0.0;
            self.bleach = true;
        }

        // Distant rumbles: slow dim swells rolling under the cloud deck.
        let rum = fbm3(t * 0.11, 3.7, t * 0.045, self.seed ^ 0x66);
        let swell = smoothstep(0.5, 0.85, rum);
        let base = 0.018 + storm * (0.02 + 0.075 * swell);

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let tex = fbm3(k.cx * 1.1, k.cy * 1.1, t * 0.07, self.seed);
            let mut col = ctx
                .palette
                .sample_clamped(0.08 + 0.18 * tex)
                .scale(base * (0.5 + tex));
            let d = self.drops[i];
            if d > 3e-3 {
                col = col.add(ctx.palette.sample_clamped(0.72).scale(d * 0.55));
            }
            out.set(k.led, col);
        }

        // The bolt: near-white flash tinting into the palette as it fades.
        if self.bolt_age < 1.1 && !self.bolt.is_empty() {
            let env = (-self.bolt_age * 4.2).exp();
            let tint = smoothstep(0.02, 0.35, self.bolt_age);
            let c = Col::lerp(Col::WHITE, ctx.palette.sample_clamped(0.95), tint);
            for &(ki, amp) in &self.bolt {
                out.max(ctx.layout.keys[ki].led, c.scale(amp * env));
            }
        }

        // Whole-board bleach for exactly the strike frame.
        if self.bleach {
            self.bleach = false;
            for k in &ctx.layout.keys {
                out.add(k.led, Col::WHITE.scale(0.3));
            }
        }
    }
}

// ------------------------------------------------------------ Ocean Tide

struct OceanTide {
    seed: u32,
}

impl Effect for OceanTide {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let scale = get_f32(ctx.params, "wave_scale", 1.0);
        let (dx, dy) = dir_vec(ctx.params);
        // Roll the crests ~27 degrees off the chosen axis so they arrive
        // diagonally rather than as flat stripes.
        let (dxr, dyr) = (dx * 0.89 - dy * 0.46, dx * 0.46 + dy * 0.89);
        let t = ctx.t;

        // The waterline breathes over ~2 minutes, with a small fast slosh.
        let wl = 0.47 - 0.3 * (t * TAU / 120.0).cos() + 0.03 * (t * 0.5).sin();

        for k in &ctx.layout.keys {
            let along = k.cx * dxr + k.cy * dyr;
            let warp = (noise2(k.cx * 1.4, k.cy * 1.4 + t * 0.06, self.seed) - 0.5) * 0.6;
            let ph = (along + warp) * 4.6 * scale - t * 1.25;
            let w = 0.5 + 0.5 * ph.sin();
            let crest = w * w * w;
            let sub = smoothstep(wl - 0.05, wl + 0.07, k.cy);

            // Submerged: palette water, brighter and later-in-palette at crests.
            let depth = ((k.cy - wl) / 0.6).clamp(0.0, 1.0);
            let water_b = 0.16 + 0.1 * (1.0 - depth) + 0.55 * crest;
            let water = ctx
                .palette
                .sample_clamped(0.16 + 0.14 * (1.0 - depth) + 0.5 * crest)
                .scale(water_b);

            // Dry shore: faint warm sand with a slow sun-shimmer (physical
            // accent so "dry" reads the same under any palette).
            let sh = noise2(k.cx * 3.2, k.cy * 3.2 + t * 0.05, self.seed ^ 0x44);
            let dry = Col::rgb(1.0, 0.62, 0.3)
                .scale(0.045 + 0.025 * sh)
                .add(ctx.palette.sample_clamped(0.85).scale(0.015));

            let mut col = Col::lerp(dry, water, sub);

            // Foam: broken sparkle riding the crest line, wet side only.
            let foam = smoothstep(0.8, 0.97, w)
                * (0.45 + 0.55 * noise2(k.cx * 6.5 + t * 2.6, k.cy * 6.5, self.seed ^ 0xF0));
            col = col.add(ctx.palette.sample_clamped(0.96).scale(foam * 0.75 * sub));

            // Swash: flickering foam band where the tide meets the sand.
            let de = (k.cy - wl) * 9.0;
            let e = (-de * de).exp();
            let fl = 0.35 + 0.65 * noise2(k.cx * 5.0 - t * 0.9, t * 1.8, self.seed ^ 0x2A);
            col = col.add(ctx.palette.sample_clamped(0.9).scale(e * fl * 0.5));

            out.set(k.led, col);
        }
    }
}

// ------------------------------------------------------------ Ivy Growth

struct IvyGrowth {
    seed: u32,
    rng: Rng,
    covered: Vec<bool>,
    cover_t: Vec<f32>,
    /// < 0 means no bud, otherwise the bud's pulse phase.
    bud: Vec<f32>,
    /// Autumn schedule: when (on the autumn clock) each key falls away.
    fall_at: Vec<f32>,
    tips: Vec<usize>,
    acc: f32,
    clock: f32,
    autumn: bool,
    autumn_t: f32,
    n_covered: usize,
}

impl IvyGrowth {
    fn edge_keys(layout: &Layout) -> Vec<usize> {
        let a = layout.aspect;
        layout
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.cy < 0.28 || k.cy > 0.82 || k.cx < 0.2 || k.cx > a - 0.2)
            .map(|(i, _)| i)
            .collect()
    }

    fn reseed(&mut self, layout: &Layout) {
        for v in self.covered.iter_mut() {
            *v = false;
        }
        for v in self.bud.iter_mut() {
            *v = -1.0;
        }
        self.tips.clear();
        self.n_covered = 0;
        self.autumn = false;
        self.autumn_t = 0.0;
        self.acc = 0.0;
        let edges = Self::edge_keys(layout);
        let s = edges[self.rng.below(edges.len())];
        self.cover(s);
        self.tips.push(s);
    }

    fn cover(&mut self, i: usize) {
        if !self.covered[i] {
            self.covered[i] = true;
            self.cover_t[i] = self.clock;
            self.n_covered += 1;
            if self.rng.chance(0.24) {
                self.bud[i] = self.rng.range(0.0, TAU);
            }
        }
    }

    fn start_autumn(&mut self, layout: &Layout) {
        self.autumn = true;
        self.autumn_t = 0.0;
        self.tips.clear();
        let mut order: Vec<usize> = (0..layout.keys.len()).filter(|&i| self.covered[i]).collect();
        for i in (1..order.len()).rev() {
            let j = self.rng.below(i + 1);
            order.swap(i, j);
        }
        for (rank, &i) in order.iter().enumerate() {
            // Hold amber for a beat, then leaves drop one by one.
            self.fall_at[i] = 1.6 + rank as f32 * 0.07;
        }
    }

    fn grow_step(&mut self, layout: &Layout) {
        if self.tips.is_empty() {
            // Relight a dormant node that still has room to creep.
            let n = layout.keys.len();
            let start = self.rng.below(n);
            let mut found = None;
            for off in 0..n {
                let i = (start + off) % n;
                if self.covered[i] && layout.neighbors[i].iter().any(|&j| !self.covered[j]) {
                    found = Some(i);
                    break;
                }
            }
            match found {
                Some(i) => self.tips.push(i),
                None => {
                    self.start_autumn(layout);
                    return;
                }
            }
        }
        let ti = self.rng.below(self.tips.len());
        let cur = self.tips[ti];
        let open: Vec<usize> = layout.neighbors[cur]
            .iter()
            .copied()
            .filter(|&j| !self.covered[j])
            .collect();
        if open.is_empty() {
            self.tips.swap_remove(ti);
            return;
        }
        let nxt = open[self.rng.below(open.len())];
        self.cover(nxt);
        self.tips[ti] = nxt;
        // Occasionally the vine forks.
        if open.len() > 1 && self.tips.len() < 6 && self.rng.chance(0.16) {
            let alt = open[self.rng.below(open.len())];
            if alt != nxt {
                self.cover(alt);
                self.tips.push(alt);
            }
        }
        if self.n_covered as f32 >= layout.keys.len() as f32 * 0.8 {
            self.start_autumn(layout);
        }
    }
}

impl Effect for IvyGrowth {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rate = get_f32(ctx.params, "growth", 1.0);
        let bloom = get_f32(ctx.params, "bloom", 0.6);
        self.clock += ctx.dt;
        let t = self.clock;

        if !self.autumn {
            self.acc += ctx.dt * rate * 7.0;
            while self.acc >= 1.0 {
                self.acc -= 1.0;
                self.grow_step(ctx.layout);
                if self.autumn {
                    break;
                }
            }
        } else {
            self.autumn_t += ctx.dt * rate;
            let end = 1.6 + self.n_covered as f32 * 0.07 + 1.2;
            if self.autumn_t > end {
                self.reseed(ctx.layout);
            }
        }

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            if !self.covered[i] {
                // Bare soil: near-black with the faintest breathing.
                out.set(k.led, ctx.palette.sample_clamped(0.05).scale(0.02));
                continue;
            }
            let age = t - self.cover_t[i];
            // A fresh tip glows bright and pale, settling into a dim stem.
            let tipglow = (-age * 1.8).exp();
            let sway =
                0.85 + 0.15 * noise2(k.cx * 2.2 + t * 0.12, k.cy * 2.2, self.seed ^ 0x51);
            let leaf = noise2(k.cx * 2.6, k.cy * 2.6 + i as f32 * 0.01, self.seed);
            let mut hue = 0.3 + 0.18 * leaf + tipglow * 0.3;
            let mut b = (0.2 + 0.1 * leaf) * sway + tipglow * 0.85;
            // Flower buds pulse brighter, sampled further up the palette.
            if self.bud[i] >= 0.0 {
                let p = 0.5 + 0.5 * (t * 1.6 + self.bud[i]).sin();
                let pb = p * p * bloom;
                b += pb * 0.55;
                hue += pb * 0.35;
            }
            let mut col = ctx.palette.sample_clamped(hue.min(0.95)).scale(b);
            if self.autumn {
                // The whole vine turns toward the palette's warm/bright end...
                let turn = smoothstep(0.0, 1.6, self.autumn_t);
                let warm = ctx.palette.sample_clamped(0.88 + 0.08 * leaf);
                col = Col::lerp(col, warm.scale(b.max(0.3)), turn * 0.85);
                // ...then leaves detach one by one: a brief flare, then gone.
                let f = self.autumn_t - self.fall_at[i];
                if f > 0.0 {
                    let flare = (-f * 5.0).exp() * 0.5;
                    let fade = (1.0 - f / 0.7).clamp(0.0, 1.0);
                    col = col.scale(fade).add(warm.scale(flare * fade));
                }
            }
            out.set(k.led, col);
        }
    }
}

// ------------------------------------------------------------ Coral Reef

const FISH_MAX: usize = 5;
const PATCH_MAX: usize = 8;

struct Fish {
    from: usize,
    to: usize,
    prog: f32,
    dur: f32,
    pause: f32,
}

struct CoralReef {
    seed: u32,
    fish: Vec<Fish>,
}

impl Effect for CoralReef {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let patches = (get_f32(ctx.params, "patches", 6.0).round() as usize).clamp(3, PATCH_MAX);
        let nfish = (get_f32(ctx.params, "fish", 3.0).round() as usize).clamp(1, FISH_MAX);
        let t = ctx.t;
        let aspect = ctx.layout.aspect;

        // Slowly wandering coral anchors (Voronoi seeds).
        let mut ax = [0.0f32; PATCH_MAX];
        let mut ay = [0.0f32; PATCH_MAX];
        for i in 0..patches {
            let fi = i as f32;
            ax[i] = aspect
                * (((noise2(t * 0.02 + fi * 17.3, fi * 5.9, self.seed) - 0.5) * 1.5 + 0.5)
                    .clamp(0.05, 0.95));
            ay[i] = (((noise2(t * 0.017 + fi * 11.1, fi * 7.7, self.seed ^ 0x1F) - 0.5) * 1.5)
                + 0.5)
                .clamp(0.05, 0.95);
        }

        for k in &ctx.layout.keys {
            let (mut d1, mut d2, mut pi) = (f32::MAX, f32::MAX, 0usize);
            for i in 0..patches {
                let ddx = k.cx - ax[i];
                let ddy = k.cy - ay[i];
                let d = (ddx * ddx + ddy * ddy).sqrt();
                if d < d1 {
                    d2 = d1;
                    d1 = d;
                    pi = i;
                } else if d < d2 {
                    d2 = d;
                }
            }
            // Each patch breathes through its own slice of the palette, out
            // of phase with its neighbors.
            let fi = pi as f32;
            let hue = fi / patches as f32 + 0.09 * (t * 0.1 + fi * 2.3).sin() + t * 0.003;
            let tex = fbm3(k.cx * 2.9, k.cy * 2.9, t * 0.05, self.seed.wrapping_add(pi as u32 * 91));
            let pulse = 0.85 + 0.15 * (t * 0.21 + fi * 1.9).sin();
            // Dark water channels along the Voronoi boundaries.
            let chan = smoothstep(0.0, 0.09, d2 - d1);
            let b = (0.24 + 0.5 * tex) * pulse * (0.3 + 0.7 * chan);
            out.set(k.led, ctx.palette.sample(hue).scale(b));
        }

        // Tiny bright fish darting key-to-key with easing and pauses.
        let nk = ctx.layout.keys.len();
        for f in self.fish[..nfish].iter_mut() {
            if f.pause > 0.0 {
                f.pause -= ctx.dt;
                if f.pause <= 0.0 {
                    f.from = f.to;
                    f.to = ctx.rng.below(nk);
                    f.dur = ctx.rng.range(0.28, 0.6);
                    f.prog = 0.0;
                }
            } else {
                f.prog += ctx.dt / f.dur;
                if f.prog >= 1.0 {
                    f.prog = 1.0;
                    f.pause = ctx.rng.range(0.5, 1.8);
                }
            }
            let a = &ctx.layout.keys[f.from];
            let bk = &ctx.layout.keys[f.to];
            let e = smoothstep(0.0, 1.0, f.prog);
            let px = a.cx + (bk.cx - a.cx) * e;
            let py = a.cy + (bk.cy - a.cy) * e;
            let glow = if f.pause <= 0.0 {
                1.0
            } else {
                0.55 + 0.15 * (t * 5.0 + f.from as f32 * 1.3).sin()
            };
            let c = ctx.palette.sample_clamped(0.97).add(Col::WHITE.scale(0.15));
            let r2 = 0.13f32 * 0.13;
            for k in &ctx.layout.keys {
                let ddx = k.cx - px;
                let ddy = k.cy - py;
                let d2 = ddx * ddx + ddy * ddy;
                if d2 > r2 {
                    continue;
                }
                let fall = (1.0 - d2 / r2).powi(2);
                out.add(k.led, c.scale(glow * fall * 0.9));
            }
        }
    }
}

// ----------------------------------------------------------- Pollen Drift

const POLLEN_MAX: usize = 40;

struct Grain {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    tw: f32,
}

struct PollenDrift {
    seed: u32,
    grains: Vec<Grain>,
    /// Accumulated glow per key where pollen lingers; decays slowly.
    glow: Vec<f32>,
    gust_x: f32,
    gust_y: f32,
    next_gust: f32,
    /// Wind-field phase offset, jumped on each gust to reshuffle the flow.
    warp: f32,
}

impl PollenDrift {
    /// Divergence-free wind: curl of a noise potential by finite differences.
    fn wind(&self, x: f32, y: f32, t: f32) -> (f32, f32) {
        let s = 1.15;
        let e = 0.07;
        let ox = t * 0.06 + self.warp;
        let oy = -t * 0.04;
        let psi = |px: f32, py: f32| noise2(px * s + ox, py * s + oy, self.seed);
        let wx = (psi(x, y + e) - psi(x, y - e)) / (2.0 * e);
        let wy = -(psi(x + e, y) - psi(x - e, y)) / (2.0 * e);
        (wx, wy)
    }
}

impl Effect for PollenDrift {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let count = (get_f32(ctx.params, "particles", 25.0).round() as usize).clamp(4, POLLEN_MAX);
        let wind = get_f32(ctx.params, "wind", 1.0);
        let t = ctx.t;
        let dt = ctx.dt;
        let aspect = ctx.layout.aspect;

        // Gusts: a sudden shared push, and the field itself reshuffles.
        self.next_gust -= dt;
        if self.next_gust <= 0.0 {
            self.next_gust = ctx.rng.range(6.0, 14.0);
            let ang = ctx.rng.range(0.0, TAU);
            let amp = ctx.rng.range(0.35, 0.8) * wind;
            self.gust_x = ang.cos() * amp;
            self.gust_y = ang.sin() * amp * 0.6;
            self.warp += ctx.rng.range(3.0, 9.0);
        }
        let gd = (-dt * 0.9).exp();
        self.gust_x *= gd;
        self.gust_y *= gd;

        // Settled glow fades slowly; pollen keeps repainting where it lingers.
        for g in self.glow.iter_mut() {
            *g *= (-dt * 0.22).exp();
        }

        // Advect grains through the curl field with a little inertia.
        for gi in 0..count {
            let (wx, wy) = self.wind(self.grains[gi].x, self.grains[gi].y, t);
            let tx = wx * 0.22 * wind + self.gust_x;
            let ty = wy * 0.22 * wind + self.gust_y;
            let f = 1.0 - (-dt * 2.2).exp();
            let g = &mut self.grains[gi];
            g.vx += (tx - g.vx) * f;
            g.vy += (ty - g.vy) * f;
            g.x += g.vx * dt;
            g.y += g.vy * dt;
            if g.x < -0.12 {
                g.x += aspect + 0.24;
            }
            if g.x > aspect + 0.12 {
                g.x -= aspect + 0.24;
            }
            if g.y < -0.12 {
                g.y += 1.24;
            }
            if g.y > 1.12 {
                g.y -= 1.24;
            }
        }

        // Glow field underneath: hue climbs the palette where dust piles up.
        for (i, k) in ctx.layout.keys.iter().enumerate() {
            let gl = self.glow[i];
            let hue = 0.3 + 0.35 * gl.min(1.0)
                + 0.08 * noise2(k.cx * 1.7, k.cy * 1.7, self.seed ^ 0x19);
            out.set(k.led, ctx.palette.sample_clamped(hue).scale(0.02 + gl * 0.4));
        }

        // The grains themselves: soft twinkling motes that deposit glow.
        for gi in 0..count {
            let g = &self.grains[gi];
            let sparkle = 0.6 + 0.4 * noise2(g.tw, t * 2.2, self.seed ^ 0x77);
            let pc = ctx.palette.sample_clamped(0.82 + 0.1 * sparkle);
            let r2 = 0.12f32 * 0.12;
            for (i, k) in ctx.layout.keys.iter().enumerate() {
                let ddx = k.cx - g.x;
                let ddy = k.cy - g.y;
                let d2 = ddx * ddx + ddy * ddy;
                if d2 > r2 {
                    continue;
                }
                let fall = (1.0 - d2 / r2).powi(2);
                self.glow[i] = (self.glow[i] + dt * 1.4 * fall).min(1.1);
                out.add(k.led, pc.scale(fall * 0.55 * sparkle));
            }
        }
    }
}
