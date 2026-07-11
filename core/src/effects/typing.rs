//! Typing — reactive effects driven by the live key-down stream. Every effect
//! keeps a quiet idle base layer so the board never goes dead between words.

use super::*;
use crate::color::{smoothstep, Col};
use crate::math::noise2;
use crate::params::{get_f32, ParamSpec};

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "typing_heatmap",
            name: "Typing Heatmap",
            category: "Typing",
            blurb: "Every keystroke banks heat that bleeds into neighbours and cools over half a minute.",
            needs_input: true,
            default_palette: "inferno",
            extras: || {
                vec![
                    ParamSpec::slider("heat_gain", "Heat per press", 0.05, 0.6, 0.01, 0.22),
                    ParamSpec::slider("cooling", "Cooling rate", 0.02, 0.4, 0.01, 0.12),
                ]
            },
            make: |layout, seed| {
                Box::new(Heatmap {
                    seed: seed as u32,
                    heat: vec![0.0; layout.keys.len()],
                    scratch: vec![0.0; layout.keys.len()],
                })
            },
        },
        EffectInfo {
            id: "combo_meter",
            name: "Combo Meter",
            category: "Typing",
            blurb: "Sustained typing builds a combo; the board fills like a power gauge until the chain breaks.",
            needs_input: true,
            default_palette: "ember",
            extras: || {
                vec![
                    ParamSpec::slider("tier_size", "Tier size", 4.0, 20.0, 1.0, 8.0),
                    ParamSpec::slider("grace", "Decay grace", 0.4, 3.0, 0.05, 1.2),
                ]
            },
            make: |_, _| Box::new(Combo { combo: 0, last_t: -1e9, fill: 0.0, slump: 0.0, flash: 0.0 }),
        },
        EffectInfo {
            id: "echo_trails",
            name: "Echo Trails",
            category: "Typing",
            blurb: "Presses shed glowing ghosts that drift upward and sway like rising embers.",
            needs_input: true,
            default_palette: "sakura",
            extras: || {
                vec![
                    ParamSpec::slider("life", "Ghost life", 0.6, 3.0, 0.05, 1.6),
                    ParamSpec::slider("drift", "Drift", 0.1, 1.0, 0.05, 0.4),
                ]
            },
            make: |_, seed| Box::new(Echo { seed: seed as u32, ghosts: Vec::new(), count: 0 }),
        },
        EffectInfo {
            id: "chain_lightning",
            name: "Chain Lightning",
            category: "Typing",
            blurb: "Consecutive keystrokes link up as jagged arcs; fast typing keeps a crackling web alive.",
            needs_input: true,
            default_palette: "glacier",
            extras: || {
                vec![
                    ParamSpec::slider("arc_life", "Arc life", 0.15, 1.2, 0.05, 0.45),
                    ParamSpec::slider("jitter", "Jitter", 0.0, 1.0, 0.05, 0.5),
                ]
            },
            make: |_, seed| {
                Box::new(Lightning { seed: seed as u32, bolts: Vec::new(), prev: None, prev_t: -1e9, rate: 0.0 })
            },
        },
        EffectInfo {
            id: "ink_splash",
            name: "Ink Splash",
            category: "Typing",
            blurb: "Keystrokes land as ink splats that bleed down the board like wet calligraphy.",
            needs_input: true,
            default_palette: "midnight",
            extras: || {
                vec![
                    ParamSpec::slider("splat", "Splat size", 0.0, 1.0, 0.05, 0.5),
                    ParamSpec::slider("drip", "Drip length", 0.2, 1.0, 0.05, 0.55),
                ]
            },
            make: |layout, _| {
                // For each key, the neighbours strictly below it — the drip graph.
                let below: Vec<Vec<usize>> = (0..layout.keys.len())
                    .map(|i| {
                        layout.neighbors[i]
                            .iter()
                            .copied()
                            .filter(|&j| layout.keys[j].cy > layout.keys[i].cy + 0.05)
                            .collect()
                    })
                    .collect();
                Box::new(Ink {
                    ink: vec![0.0; layout.keys.len()],
                    pos: vec![0.5; layout.keys.len()],
                    below,
                    presses: 0,
                })
            },
        },
        EffectInfo {
            id: "tempo_pulse",
            name: "Tempo Pulse",
            category: "Typing",
            blurb: "Learns your typing tempo and pulses on the beat, winding down when you stop.",
            needs_input: true,
            default_palette: "synthwave",
            extras: || {
                vec![
                    ParamSpec::slider("sharpness", "Pulse sharpness", 1.0, 8.0, 0.1, 3.5),
                    ParamSpec::slider("adapt", "Adapt rate", 0.05, 0.8, 0.05, 0.3),
                ]
            },
            make: |layout, seed| {
                Box::new(Tempo {
                    seed: seed as u32,
                    avg: 0.4,
                    last_t: -1e9,
                    phase: 0.0,
                    energy: 0.0,
                    ox: layout.aspect * 0.5,
                    oy: 0.55,
                })
            },
        },
        EffectInfo {
            id: "whack_a_key",
            name: "Whack-a-Key",
            category: "Typing",
            blurb: "Chase the glowing target key — hits build a streak, misses flicker red and reset it.",
            needs_input: true,
            default_palette: "toxic",
            extras: || {
                vec![
                    ParamSpec::slider("window", "Base window", 0.8, 3.0, 0.05, 1.8),
                    ParamSpec::slider("ramp", "Difficulty ramp", 0.0, 1.0, 0.05, 0.5),
                ]
            },
            make: |layout, seed| {
                let mut rng = Rng::new(seed);
                let mut strip: Vec<usize> =
                    (0..layout.keys.len()).filter(|&i| layout.keys[i].row == 1).collect();
                strip.sort_by(|&a, &b| layout.keys[a].cx.partial_cmp(&layout.keys[b].cx).unwrap());
                let mut pool: Vec<usize> =
                    (0..layout.keys.len()).filter(|&i| layout.keys[i].row >= 2).collect();
                if pool.is_empty() {
                    pool = (0..layout.keys.len()).collect();
                }
                let target = pool[rng.below(pool.len())];
                Box::new(Whack {
                    strip,
                    pool,
                    target,
                    born: -1.0,
                    streak: 0,
                    fx_kind: 0,
                    fx_t: -1e9,
                    fx_x: 0.0,
                    fx_y: 0.0,
                })
            },
        },
    ]
}

// ---------------------------------------------------------------- Heatmap

struct Heatmap {
    seed: u32,
    heat: Vec<f32>,
    scratch: Vec<f32>,
}

impl Effect for Heatmap {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let gain = get_f32(ctx.params, "heat_gain", 0.22);
        let cool = get_f32(ctx.params, "cooling", 0.12);
        let layout = ctx.layout;

        for tap in ctx.taps {
            if tap.key >= self.heat.len() {
                continue;
            }
            self.heat[tap.key] += gain;
            for &n in &layout.neighbors[tap.key] {
                self.heat[n] += gain * 0.3;
            }
        }

        // Bleed into neighbours (relaxation toward the local average), then cool.
        self.scratch.copy_from_slice(&self.heat);
        let d = (ctx.dt * 0.5).min(0.4);
        let decay = (-cool * ctx.dt).exp();
        for i in 0..self.heat.len() {
            let ns = &layout.neighbors[i];
            if !ns.is_empty() {
                let mut avg = 0.0;
                for &j in ns {
                    avg += self.scratch[j];
                }
                avg /= ns.len() as f32;
                self.heat[i] += (avg - self.scratch[i]) * d * 0.5;
            }
            self.heat[i] *= decay;
        }

        let mut hot = 0usize;
        for i in 1..self.heat.len() {
            if self.heat[i] > self.heat[hot] {
                hot = i;
            }
        }

        for (i, k) in layout.keys.iter().enumerate() {
            // soft-knee temperature: stacked presses saturate instead of clipping
            let temp = 1.0 - (-self.heat[i]).exp();
            let mut b = 0.14 + 0.86 * temp.powf(0.85);
            if i == hot && self.heat[i] > 0.35 {
                b *= 0.82 + 0.30 * noise2(ctx.t * 9.0, i as f32 * 3.3, self.seed);
            }
            out.set(k.led, ctx.palette.sample_clamped(0.10 + 0.86 * temp).scale(b));
        }
    }
}

// ---------------------------------------------------------------- Combo

struct Combo {
    combo: u32,
    last_t: f32,
    fill: f32,
    slump: f32,
    flash: f32,
}

impl Effect for Combo {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let tier_size = get_f32(ctx.params, "tier_size", 8.0).round().max(1.0);
        let grace = get_f32(ctx.params, "grace", 1.2);
        let ts = tier_size as u32;

        if self.combo > 0 && ctx.t - self.last_t > grace {
            self.combo = 0;
            self.slump = 1.0; // the chain snapped — dim while the gauge drains
        }
        for tap in ctx.taps {
            self.combo += 1;
            self.last_t = tap.t;
            self.slump = 0.0;
            if self.combo % ts == 0 {
                self.flash = 1.0; // tier up
            }
        }

        // gauge fills fast toward the tier target, drains slowly when broken
        let target = (self.combo as f32 / (tier_size * 5.0)).min(1.0);
        if target >= self.fill {
            self.fill += (target - self.fill) * (ctx.dt * 6.0).min(1.0);
        } else {
            self.fill = (self.fill - ctx.dt * 0.4).max(target);
        }
        self.slump = (self.slump - ctx.dt * 0.5).max(0.0);
        self.flash = (self.flash - ctx.dt * 2.5).max(0.0);

        let tier = self.combo as f32 / tier_size;
        let dim = 1.0 - 0.55 * self.slump;
        let vib = ((tier - 2.0) * 0.010).clamp(0.0, 0.030);
        let wob = (ctx.t * 36.0).sin() * vib;
        let breath = 0.5 + 0.5 * (ctx.t * 2.2).sin();

        for k in &ctx.layout.keys {
            let fb = 1.0 - k.cy; // 0 at the space row, 1 at the media row
            let lit = smoothstep(fb - 0.06, fb + 0.02, self.fill + wob);
            let hgt = (fb / self.fill.max(0.08)).clamp(0.0, 1.0);
            let pos = 0.08 + 0.80 * target * hgt;
            let edge = 1.0 - smoothstep(0.0, 0.05, (self.fill + wob - fb).abs());
            let b = (0.10 + lit * (0.50 + 0.08 * breath)) * dim + self.flash * lit * 0.45;
            let c = ctx
                .palette
                .sample_clamped(pos)
                .scale(b)
                .add(ctx.palette.sample_clamped(0.90).scale(edge * (0.10 + 0.30 * lit) * dim));
            out.set(k.led, c);
        }
    }
}

// ---------------------------------------------------------------- Echo

struct Ghost {
    x: f32,
    y: f32,
    born: f32,
    hue: f32,
    sway: f32,
}

struct Echo {
    seed: u32,
    ghosts: Vec<Ghost>,
    count: u32,
}

impl Effect for Echo {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let life = get_f32(ctx.params, "life", 1.6);
        let drift = get_f32(ctx.params, "drift", 0.4);
        let layout = ctx.layout;

        let taps = ctx.taps;
        for tap in taps {
            self.count += 1;
            self.ghosts.push(Ghost {
                x: tap.cx,
                y: tap.cy,
                born: tap.t,
                hue: (self.count as f32 * 0.137).fract(),
                sway: ctx.rng.range(0.0, 80.0),
            });
        }
        // the keyboard dreams a little while idle
        if ctx.rng.chance(ctx.dt * 0.12) && !layout.keys.is_empty() {
            let k = &layout.keys[ctx.rng.below(layout.keys.len())];
            self.count += 1;
            self.ghosts.push(Ghost {
                x: k.cx,
                y: k.cy,
                born: ctx.t,
                hue: (self.count as f32 * 0.137).fract(),
                sway: ctx.rng.range(0.0, 80.0),
            });
        }
        let t = ctx.t;
        let rise = 0.12 + drift * 0.5;
        self.ghosts.retain(|g| t - g.born < life && g.y - (t - g.born) * rise > -0.25);
        if self.ghosts.len() > 90 {
            let cut = self.ghosts.len() - 90;
            self.ghosts.drain(0..cut);
        }

        // dim breathing base wash
        for k in &layout.keys {
            let b = 0.055 + 0.025 * (0.5 + 0.5 * (ctx.t * 0.5 + k.cx * 1.2).sin());
            out.set(k.led, ctx.palette.sample_clamped(0.14 + 0.10 * (1.0 - k.cy)).scale(b));
        }

        for g in &self.ghosts {
            let age = ctx.t - g.born;
            let u = (age / life).clamp(0.0, 1.0);
            let gy = g.y - age * rise;
            // sway widens as the ghost climbs
            let gx = g.x + (noise2(g.sway, age * 1.5, self.seed) - 0.5) * 0.22 * (0.25 + u);
            let r = 0.16 * (1.0 - 0.55 * u);
            let bright = (1.0 - u).powf(1.5) * 0.85;
            let c = ctx.palette.sample(g.hue + u * 0.15);
            for k in &layout.keys {
                let dx = k.cx - gx;
                if dx.abs() > r {
                    continue;
                }
                let dy = k.cy - gy;
                if dy.abs() > r {
                    continue;
                }
                let w = 1.0 - smoothstep(0.0, r, (dx * dx + dy * dy).sqrt());
                out.add(k.led, c.scale(w * w * bright));
            }
        }
    }
}

// ---------------------------------------------------------------- Lightning

struct Bolt {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    born: f32,
    seed: f32,
    strength: f32,
}

struct Lightning {
    seed: u32,
    bolts: Vec<Bolt>,
    prev: Option<(f32, f32)>,
    prev_t: f32,
    rate: f32,
}

impl Effect for Lightning {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let life = get_f32(ctx.params, "arc_life", 0.45).max(0.05);
        let jit = get_f32(ctx.params, "jitter", 0.5) * 0.10;
        let layout = ctx.layout;

        let taps = ctx.taps;
        for tap in taps {
            if let Some((px, py)) = self.prev {
                let interval = (tap.t - self.prev_t).max(0.02);
                if interval < 1.6 {
                    self.rate = self.rate * 0.65 + (1.0 / interval).min(12.0) * 0.35;
                    let strength = 0.40 + 0.60 * (self.rate / 8.0).min(1.0);
                    let span = ((tap.cx - px).powi(2) + (tap.cy - py).powi(2)).sqrt();
                    if span > 0.05 {
                        self.bolts.push(Bolt {
                            x0: px,
                            y0: py,
                            x1: tap.cx,
                            y1: tap.cy,
                            born: tap.t,
                            seed: ctx.rng.range(0.0, 90.0),
                            strength,
                        });
                    }
                } else {
                    self.rate *= 0.5;
                }
            }
            self.prev = Some((tap.cx, tap.cy));
            self.prev_t = tap.t;
        }
        let t = ctx.t;
        self.bolts.retain(|b| t - b.born < life);
        if self.bolts.len() > 24 {
            let cut = self.bolts.len() - 24;
            self.bolts.drain(0..cut);
        }

        // idle base: faint wash plus wandering static-charge glints
        for (i, k) in layout.keys.iter().enumerate() {
            let glint = smoothstep(0.86, 0.99, noise2(i as f32 * 9.7, ctx.t * 2.4, self.seed));
            let c = ctx
                .palette
                .sample_clamped(0.12)
                .scale(0.09)
                .add(ctx.palette.sample_clamped(0.78).scale(glint * 0.22));
            out.set(k.led, c);
        }

        let core = ctx.palette.sample_clamped(0.95);
        let sheath = ctx.palette.sample_clamped(0.55);
        for b in &self.bolts {
            let age = ctx.t - b.born;
            let u = (age / life).clamp(0.0, 1.0);
            let flick = 0.65 + 0.35 * noise2(b.seed, ctx.t * 24.0, self.seed);
            let env = (1.0 - u) * (1.0 - u) * flick * b.strength;
            if env < 0.01 {
                continue;
            }
            let dx = b.x1 - b.x0;
            let dy = b.y1 - b.y0;
            let len = (dx * dx + dy * dy).sqrt().max(1e-4);
            let (ux, uy) = (dx / len, dy / len);
            for k in &layout.keys {
                let rx = k.cx - b.x0;
                let ry = k.cy - b.y0;
                let s = (rx * ux + ry * uy) / len;
                let sc = s.clamp(0.0, 1.0);
                let over = (s - sc).abs() * len;
                let perp = rx * (-uy) + ry * ux;
                // noise-perturbed centreline, pinned at both endpoint keys
                let off = (noise2(sc * 6.0 + b.seed, ctx.t * 7.0, self.seed) - 0.5)
                    * 2.0
                    * jit
                    * (sc * std::f32::consts::PI).sin();
                let d = ((perp - off).powi(2) + over * over).sqrt();
                let w = 1.0 - smoothstep(0.015, 0.10, d);
                if w > 0.001 {
                    out.add(k.led, sheath.scale(w * env * 0.45).add(core.scale(w * w * env * 0.8)));
                }
            }
        }
    }
}

// ---------------------------------------------------------------- Ink

struct Ink {
    ink: Vec<f32>,
    pos: Vec<f32>,
    below: Vec<Vec<usize>>,
    presses: u32,
}

impl Ink {
    /// Add ink to a key, blending its palette position by weight.
    fn deposit(&mut self, i: usize, amt: f32, p: f32) {
        let w = self.ink[i];
        self.pos[i] = if w + amt > 1e-5 { (self.pos[i] * w + p * amt) / (w + amt) } else { p };
        self.ink[i] = (w + amt).min(2.5);
    }
}

impl Effect for Ink {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let splat = get_f32(ctx.params, "splat", 0.5);
        let drip = get_f32(ctx.params, "drip", 0.55);
        let layout = ctx.layout;

        let taps = ctx.taps;
        for tap in taps {
            if tap.key >= self.ink.len() {
                continue;
            }
            self.presses += 1;
            // each drop lands somewhere else in the palette, so layers tint
            let p = 0.25 + (self.presses as f32 * 0.161).fract() * 0.60;
            self.deposit(tap.key, 0.9 + splat * 0.4, p);
            for &n in &layout.neighbors[tap.key] {
                if ctx.rng.chance(0.20 + splat * 0.55) {
                    let amt = ctx.rng.range(0.15, 0.30 + splat * 0.5);
                    self.deposit(n, amt, p);
                }
            }
        }

        // gravity: ink seeps to the keys below, thinning on the way down
        let flow = (ctx.dt * 1.5).min(0.5);
        let evap = (-(1.7 - drip * 1.1) * ctx.dt).exp();
        for i in 0..self.ink.len() {
            let amt = self.ink[i];
            if amt < 0.004 {
                continue;
            }
            let nb = self.below[i].len();
            if nb > 0 {
                let moved = amt * flow;
                self.ink[i] = amt - moved;
                let share = moved * (0.35 + 0.5 * drip) / nb as f32;
                let p = self.pos[i];
                for bi in 0..nb {
                    let j = self.below[i][bi];
                    self.deposit(j, share, p);
                }
            }
        }
        for v in self.ink.iter_mut() {
            *v *= evap;
        }

        let paper = ctx.palette.sample_clamped(0.97).scale(0.05);
        for (i, k) in layout.keys.iter().enumerate() {
            let v = 1.0 - (-self.ink[i] * 1.7).exp();
            let wet = ctx.palette.sample_clamped(self.pos[i]).scale(0.9 * v);
            out.set(k.led, paper.max(wet));
        }
    }
}

// ---------------------------------------------------------------- Tempo

struct Tempo {
    seed: u32,
    avg: f32,
    last_t: f32,
    phase: f32,
    energy: f32,
    ox: f32,
    oy: f32,
}

impl Effect for Tempo {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let sharp = get_f32(ctx.params, "sharpness", 3.5);
        let adapt = get_f32(ctx.params, "adapt", 0.3);

        for tap in ctx.taps {
            let interval = tap.t - self.last_t;
            if interval > 0.05 && interval < 2.0 {
                self.avg += (interval - self.avg) * adapt;
            }
            self.last_t = tap.t;
            // origin slides toward where the hands actually are
            self.ox += (tap.cx - self.ox) * 0.25;
            self.oy += (tap.cy - self.oy) * 0.25;
            // gentle phase lock: pull the nearest beat onto this press
            let err = if self.phase < 0.5 { self.phase } else { self.phase - 1.0 };
            self.phase = (self.phase - err * 0.5 + 1.0).fract();
        }

        self.energy = (1.0 - (ctx.t - self.last_t) / 5.0).clamp(0.0, 1.0);
        // the metronome slows as it winds down
        let period = self.avg.clamp(0.12, 1.5) * (1.0 + (1.0 - self.energy) * 1.8);
        self.phase = (self.phase + ctx.dt / period).fract();

        let amp = 0.06 + 0.72 * self.energy;
        for k in &ctx.layout.keys {
            let dx = k.cx - self.ox;
            let dy = k.cy - self.oy;
            let d = (dx * dx + dy * dy).sqrt();
            let lp = (self.phase - d * 0.20 + 2.0).fract();
            let env = (1.0 - lp).powf(sharp);
            let tex = 0.88 + 0.24 * noise2(k.cx * 1.8, k.cy * 1.8 + ctx.t * 0.15, self.seed);
            let c = ctx.palette.sample(0.08 + 0.55 * env + d * 0.10 + ctx.t * 0.008);
            out.set(k.led, c.scale((0.09 + env * amp) * tex));
        }
    }
}

// ---------------------------------------------------------------- Whack

struct Whack {
    strip: Vec<usize>,
    pool: Vec<usize>,
    target: usize,
    born: f32,
    streak: u32,
    /// 0 = none, 1 = hit burst, 2 = miss flicker.
    fx_kind: u8,
    fx_t: f32,
    fx_x: f32,
    fx_y: f32,
}

impl Whack {
    fn relocate(&mut self, rng: &mut Rng, t: f32) {
        let old = self.target;
        for _ in 0..8 {
            self.target = self.pool[rng.below(self.pool.len())];
            if self.target != old {
                break;
            }
        }
        self.born = t;
    }
}

impl Effect for Whack {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let base_w = get_f32(ctx.params, "window", 1.8);
        let ramp = get_f32(ctx.params, "ramp", 0.5);
        let layout = ctx.layout;

        if self.born < 0.0 {
            self.born = ctx.t;
        }

        let taps = ctx.taps;
        for tap in taps {
            let (tx, ty) = {
                let k = &layout.keys[self.target];
                (k.cx, k.cy)
            };
            if tap.key == self.target {
                self.streak += 1;
                self.fx_kind = 1;
                self.fx_x = tap.cx;
                self.fx_y = tap.cy;
            } else {
                self.streak = 0;
                self.fx_kind = 2;
                self.fx_x = tx;
                self.fx_y = ty;
            }
            self.fx_t = ctx.t;
            self.relocate(ctx.rng, ctx.t);
        }

        // the window tightens as the streak grows
        let window = 0.35 + (base_w - 0.35) * (-(self.streak as f32) * ramp * 0.12).exp();
        if ctx.t - self.born > window {
            self.streak = 0;
            self.fx_kind = 2;
            self.fx_t = ctx.t;
            let k = &layout.keys[self.target];
            self.fx_x = k.cx;
            self.fx_y = k.cy;
            self.relocate(ctx.rng, ctx.t);
        }

        // dim scoreboard glow
        for k in &layout.keys {
            out.set(k.led, ctx.palette.sample_clamped(0.10).scale(0.08));
        }

        // streak strip along row 1
        let sl = self.strip.len().max(1);
        let n_lit = (self.streak as usize).min(self.strip.len());
        let over = self.streak as usize > self.strip.len();
        for (si, &ki) in self.strip.iter().enumerate() {
            let k = &layout.keys[ki];
            if si < n_lit {
                let mut c = ctx.palette.sample_clamped(0.30 + 0.60 * si as f32 / sl as f32).scale(0.5);
                if over {
                    c = c.scale(0.8 + 0.25 * (ctx.t * 6.0).sin());
                }
                out.set(k.led, c);
            } else {
                out.set(k.led, ctx.palette.sample_clamped(0.20).scale(0.12));
            }
        }

        // target urgency: blinks faster and burns hotter as time runs out
        let u = ((ctx.t - self.born) / window).clamp(0.0, 1.0);
        let saw = (ctx.t * (2.5 + 9.0 * u)).fract();
        let blink = 0.40 + 0.60 * (1.0 - saw) * (1.0 - saw);
        let tb = (0.30 + 0.70 * u) * blink + 0.18;
        let tc = ctx.palette.sample_clamped(0.60 + 0.35 * u);
        let tk = &layout.keys[self.target];
        out.max(tk.led, tc.scale(tb));
        for &n in &layout.neighbors[self.target] {
            out.max(layout.keys[n].led, tc.scale(tb * 0.30));
        }

        // hit / miss feedback
        let age = ctx.t - self.fx_t;
        if self.fx_kind == 1 && age < 0.45 {
            let f = (1.0 - age / 0.45).powi(2);
            let green = Col::rgb(0.15, 1.0, 0.30);
            for k in &layout.keys {
                let d = ((k.cx - self.fx_x).powi(2) + (k.cy - self.fx_y).powi(2)).sqrt();
                let w = 1.0 - smoothstep(0.04, 0.30, d);
                if w > 0.001 {
                    out.add(k.led, green.scale(w * f * 0.9));
                }
            }
        } else if self.fx_kind == 2 && age < 0.40 {
            let f = 1.0 - age / 0.40;
            let flicker = if (ctx.t * 26.0).fract() < 0.5 { 1.0 } else { 0.2 };
            let red = Col::rgb(1.0, 0.10, 0.06);
            for k in &layout.keys {
                let d = ((k.cx - self.fx_x).powi(2) + (k.cy - self.fx_y).powi(2)).sqrt();
                let w = 1.0 - smoothstep(0.04, 0.26, d);
                out.add(k.led, red.scale((w * 0.85 + 0.05) * f * flicker));
            }
        }
    }
}
