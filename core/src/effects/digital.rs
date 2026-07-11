//! Digital / Glitch — crisp row/column logic, quantized steps, corrupted
//! signals. These effects lean into the keyboard being a coarse grid: state
//! lives on (row, col) cells and steppiness is a feature, not a bug.

use super::*;
use crate::color::{smoothstep, Col};
use crate::math::{fbm3, noise2};
use crate::params::{get_bool, get_f32, get_str, ParamSpec};
use std::f32::consts::TAU;

pub fn effects() -> Vec<EffectInfo> {
    vec![
        EffectInfo {
            id: "glitch_cascade",
            name: "Glitch Cascade",
            category: "Digital",
            blurb: "A serene gradient corrupted in bursts — shears, channel splits and bad quantization that fully heal.",
            needs_input: false,
            default_palette: "synthwave",
            extras: || {
                vec![
                    ParamSpec::slider("rate", "Glitch rate", 0.2, 3.0, 0.05, 1.0),
                    ParamSpec::slider("severity", "Severity", 0.1, 1.0, 0.05, 0.6),
                ]
            },
            make: |_, seed| {
                Box::new(GlitchCascade { seed: seed as u32, glitches: Vec::new(), burst: 0.0, lull: 1.0 })
            },
        },
        EffectInfo {
            id: "bad_signal",
            name: "Bad Signal",
            category: "Digital",
            blurb: "Analog TV on its last legs: rolling hum bars, crackling static, drifting tint and channel changes.",
            needs_input: false,
            default_palette: "mono",
            extras: || {
                vec![
                    ParamSpec::slider("static", "Static amount", 0.0, 1.0, 0.05, 0.35),
                    ParamSpec::slider("roll", "Roll rate", 0.2, 3.0, 0.05, 1.0),
                ]
            },
            make: |_, seed| {
                let mut r = Rng::new(seed);
                Box::new(BadSignal {
                    seed: seed as u32,
                    gx: r.range(0.25, 0.5),
                    gy: r.range(-0.4, 0.4),
                    off: r.f32(),
                    next: r.range(6.0, 12.0),
                    flash: 0.0,
                })
            },
        },
        EffectInfo {
            id: "packet_flow",
            name: "Packet Flow",
            category: "Digital",
            blurb: "Packets route between keys on Manhattan paths; crossing traffic collides in a white flash.",
            needs_input: false,
            default_palette: "hologram",
            extras: || {
                vec![
                    ParamSpec::slider("rate", "Packet rate", 0.2, 3.0, 0.05, 1.0),
                    ParamSpec::slider("trail", "Trail", 0.0, 1.0, 0.05, 0.5),
                ]
            },
            make: |layout, seed| {
                Box::new(PacketFlow {
                    grid: Grid::of(layout),
                    trail: vec![Col::BLACK; layout.keys.len()],
                    packets: Vec::new(),
                    flashes: Vec::new(),
                    seed: seed as u32,
                })
            },
        },
        EffectInfo {
            id: "game_of_life",
            name: "Game of Life",
            category: "Digital",
            blurb: "Conway on the key grid with color genetics: newborns inherit their parents' hue, colonies drift apart.",
            needs_input: false,
            default_palette: "viridian",
            extras: || {
                vec![
                    ParamSpec::slider("step_rate", "Step rate", 0.5, 8.0, 0.25, 3.0),
                    ParamSpec::slider("mutation", "Mutation", 0.0, 1.0, 0.05, 0.35),
                ]
            },
            make: |layout, seed| {
                let grid = Grid::of(layout);
                let n = grid.w * grid.h;
                let mut r = Rng::new(seed);
                let mut alive = vec![false; n];
                let mut gene = vec![0.0f32; n];
                for i in 0..n {
                    alive[i] = r.chance(0.3);
                    gene[i] = r.f32();
                }
                Box::new(Life {
                    grid,
                    alive,
                    gene,
                    age: vec![0.0; n],
                    ghost: vec![0.0; n],
                    ghost_gene: vec![0.0; n],
                    acc: 0.0,
                    recent: [0; 4],
                    recent_i: 0,
                    stagnant: 0,
                })
            },
        },
        EffectInfo {
            id: "rule_cascade",
            name: "Rule Cascade",
            category: "Digital",
            blurb: "An elementary cellular automaton printing generations down the board like a waterfall teletype.",
            needs_input: false,
            default_palette: "matrix",
            extras: || {
                vec![
                    ParamSpec::select("rule", "Rule", vec!["30", "90", "110", "auto"], "auto"),
                    ParamSpec::slider("gen_rate", "Generation rate", 1.0, 12.0, 0.5, 5.0),
                ]
            },
            make: |layout, seed| {
                let grid = Grid::of(layout);
                let n = grid.w * grid.h;
                let mut r = Rng::new(seed);
                let mut hist = vec![false; n];
                for c in 0..grid.w {
                    hist[c] = r.chance(0.35);
                }
                let old = hist.clone();
                Box::new(RuleCascade { grid, hist, old, acc: 0.0, rule: 30, auto_i: 0, auto_t: 30.0, wipe: 1.0 })
            },
        },
        EffectInfo {
            id: "bitcrush",
            name: "Bitcrush",
            category: "Digital",
            blurb: "A smooth gradient fed through a struggling codec — bit depth collapses into ordered dither and back.",
            needs_input: false,
            default_palette: "candy",
            extras: || {
                vec![
                    ParamSpec::slider("crush", "Crush depth", 0.0, 1.0, 0.05, 0.7),
                    ParamSpec::toggle("dither", "Dither", true),
                ]
            },
            make: |_, seed| Box::new(Bitcrush { seed: seed as u32 }),
        },
        EffectInfo {
            id: "firewall",
            name: "Firewall",
            category: "Digital",
            blurb: "A security sweep probes the board; intrusions flash red, get boxed in and flushed off the edge.",
            needs_input: false,
            default_palette: "bloodmoon",
            extras: || {
                vec![
                    ParamSpec::slider("scan_rate", "Scan rate", 0.2, 3.0, 0.05, 1.0),
                    ParamSpec::slider("threat", "Threat rate", 0.0, 1.0, 0.05, 0.5),
                ]
            },
            make: |layout, _| {
                Box::new(Firewall {
                    wmax: layout.keys.iter().map(|k| k.col as i32).max().unwrap_or(0),
                    scan: 0.0,
                    dir: 1.0,
                    blips: Vec::new(),
                    incident: None,
                })
            },
        },
    ]
}

// ---------------------------------------------------------------- Grid

/// (row, col) -> key index map for the effects that think in cells. Cells
/// with no physical key still simulate; they just don't render.
struct Grid {
    w: usize,
    h: usize,
    cell: Vec<Option<usize>>,
}

impl Grid {
    fn of(layout: &Layout) -> Grid {
        let w = layout.keys.iter().map(|k| k.col as usize).max().unwrap_or(0) + 1;
        let h = layout.keys.iter().map(|k| k.row as usize).max().unwrap_or(0) + 1;
        let mut cell = vec![None; w * h];
        for (i, k) in layout.keys.iter().enumerate() {
            cell[k.row as usize * w + k.col as usize] = Some(i);
        }
        Grid { w, h, cell }
    }

    fn key(&self, row: usize, col: usize) -> Option<usize> {
        if row < self.h && col < self.w {
            self.cell[row * self.w + col]
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------- GlitchCascade

struct Glitch {
    /// 0 = shear (sample offset), 1 = RGB channel split, 2 = bad quantize.
    kind: u8,
    /// Band across rows (true, tested on cy) or columns (false, on cx).
    row_axis: bool,
    lo: f32,
    hi: f32,
    mag: f32,
    wrong: f32,
    levels: f32,
    life: f32,
}

struct GlitchCascade {
    seed: u32,
    glitches: Vec<Glitch>,
    burst: f32,
    lull: f32,
}

impl Effect for GlitchCascade {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rate = get_f32(ctx.params, "rate", 1.0);
        let sev = get_f32(ctx.params, "severity", 0.6);

        // Glitches arrive in clustered bursts separated by clean lulls, so
        // the base gradient gets moments to fully heal.
        if self.burst <= 0.0 {
            self.lull -= ctx.dt;
            if self.lull <= 0.0 {
                self.burst = ctx.rng.range(0.35, 1.1);
                self.lull = ctx.rng.range(2.5, 7.0) / rate.max(0.05);
            }
        } else {
            self.burst -= ctx.dt;
            if self.glitches.len() < 7 && ctx.rng.chance(ctx.dt * (6.0 + 10.0 * rate)) {
                let row_axis = ctx.rng.chance(0.6);
                let extent = if row_axis { 1.0 } else { ctx.layout.aspect };
                let c = ctx.rng.range(0.0, extent);
                let half = ctx.rng.range(0.06, 0.16 + 0.12 * sev);
                let kind = ctx.rng.below(3) as u8;
                let sign = if ctx.rng.chance(0.5) { 1.0 } else { -1.0 };
                let mag = match kind {
                    // shear in whole key-pitch steps so blocks visibly jump
                    0 => sign * 0.155 * (1 + ctx.rng.below(3)) as f32 * (0.5 + sev),
                    1 => 0.04 + 0.10 * sev * ctx.rng.f32(),
                    _ => 0.0,
                };
                self.glitches.push(Glitch {
                    kind,
                    row_axis,
                    lo: c - half,
                    hi: c + half,
                    mag,
                    wrong: ctx.rng.range(0.2, 0.8),
                    levels: (2 + ctx.rng.below(3)) as f32,
                    life: ctx.rng.range(0.06, 0.12 + 0.30 * sev),
                });
            }
        }
        for g in self.glitches.iter_mut() {
            g.life -= ctx.dt;
        }
        self.glitches.retain(|g| g.life > 0.0);

        let t = ctx.t;
        let base_u = |x: f32, y: f32| x * 0.18 + y * 0.30 + t * 0.02;
        for k in &ctx.layout.keys {
            let mut dx = 0.0;
            let mut split = 0.0f32;
            let mut quant: Option<(f32, f32)> = None;
            let mut hot = false;
            for g in &self.glitches {
                let p = if g.row_axis { k.cy } else { k.cx };
                if p < g.lo || p > g.hi {
                    continue;
                }
                hot = true;
                match g.kind {
                    0 => dx += g.mag,
                    1 => split = split.max(g.mag),
                    _ => quant = Some((g.levels, g.wrong)),
                }
            }
            let mut u = base_u(k.cx + dx, k.cy);
            if let Some((lv, wrong)) = quant {
                // snap to a coarse level, then land in the wrong palette spot
                u = (u * lv).floor() / lv + wrong;
            }
            let c = if split > 0.0 {
                let a = ctx.palette.sample(u - split);
                let m = ctx.palette.sample(u);
                let b = ctx.palette.sample(u + split);
                Col::rgb(a.r, m.g, b.b)
            } else {
                ctx.palette.sample(u)
            };
            // corrupted keys flicker on a coarse clock so damage reads digital
            let br = if hot {
                let tq = (t * 30.0).floor();
                0.55 + 0.55 * noise2(k.led as f32 * 3.1, tq, self.seed)
            } else {
                0.82 + 0.18 * noise2(k.cx * 1.3, k.cy * 1.3 + t * 0.05, self.seed)
            };
            out.set(k.led, c.scale(br));
        }
    }
}

// ---------------------------------------------------------------- BadSignal

struct BadSignal {
    seed: u32,
    // current "channel": a random linear gradient scene
    gx: f32,
    gy: f32,
    off: f32,
    next: f32,
    flash: f32,
}

impl Effect for BadSignal {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let stat = get_f32(ctx.params, "static", 0.35);
        let roll = get_f32(ctx.params, "roll", 1.0);

        self.next -= ctx.dt;
        if self.next <= 0.0 {
            // channel change: a beat of loud static, then a new scene snaps in
            self.flash = 0.10;
            self.gx = ctx.rng.range(0.2, 0.55) * if ctx.rng.chance(0.3) { -1.0 } else { 1.0 };
            self.gy = ctx.rng.range(-0.45, 0.45);
            self.off = ctx.rng.f32();
            self.next = ctx.rng.range(6.0, 16.0);
        }
        let flashing = self.flash > 0.0;
        if flashing {
            self.flash -= ctx.dt;
        }

        // crackle clock: static re-rolls in discrete ticks, not per frame
        let tq = (ctx.t * 22.0).floor();
        // tuning drift: the whole picture slowly slides out of tune
        let detune = 0.05 * (ctx.t * 0.31).sin() + 0.06 * (noise2(ctx.t * 0.17, 5.0, self.seed) - 0.5);

        for (i, k) in ctx.layout.keys.iter().enumerate() {
            if flashing {
                let n = noise2(i as f32 * 17.3, tq * 1.7 + 3.0, self.seed ^ 0xA5A5);
                let g = n * n;
                out.set(k.led, Col::rgb(g, g, g).max(ctx.palette.sample(n).scale(0.3)));
                continue;
            }
            // hum bars rolling upward, one broad and one tight
            let bar = 0.62
                + 0.28 * smoothstep(0.3, 0.95, (TAU * (k.cy * 2.0 + ctx.t * roll * 0.35)).sin())
                + 0.10 * smoothstep(0.5, 0.95, (TAU * (k.cy * 5.0 + ctx.t * roll * 0.9)).sin());
            // the scene, with chroma pulled apart by the detune
            let u = k.cx * self.gx + k.cy * self.gy + self.off + detune * 0.4;
            let base = ctx.palette.sample(u);
            let c = Col::rgb(
                ctx.palette.sample(u + detune).r,
                base.g,
                ctx.palette.sample(u - detune).b,
            );
            // low-level per-key crackle
            let n = noise2(i as f32 * 13.7, tq, self.seed);
            let crackle = n * n * n * stat * 0.8;
            out.set(k.led, c.scale(bar).add(Col::rgb(crackle, crackle, crackle)));
        }
    }
}

// ---------------------------------------------------------------- PacketFlow

struct Packet {
    path: Vec<usize>,
    pos: f32,
    speed: f32,
    hue: f32,
}

struct PacketFlow {
    grid: Grid,
    /// Own decaying tail buffer, per key index (frames arrive black).
    trail: Vec<Col>,
    packets: Vec<Packet>,
    flashes: Vec<(usize, f32)>,
    seed: u32,
}

impl PacketFlow {
    /// Manhattan route: along the source row to the destination column, then
    /// down that column. Grid gaps (no physical key) are simply skipped.
    fn route(&self, layout: &Layout, a: usize, b: usize) -> Vec<usize> {
        let (r0, c0) = (layout.keys[a].row as i32, layout.keys[a].col as i32);
        let (r1, c1) = (layout.keys[b].row as i32, layout.keys[b].col as i32);
        let mut path: Vec<usize> = Vec::new();
        let push = |r: i32, c: i32, path: &mut Vec<usize>| {
            if let Some(k) = self.grid.key(r as usize, c as usize) {
                if path.last() != Some(&k) {
                    path.push(k);
                }
            }
        };
        let mut c = c0;
        loop {
            push(r0, c, &mut path);
            if c == c1 {
                break;
            }
            c += (c1 - c0).signum();
        }
        let mut r = r0;
        while r != r1 {
            r += (r1 - r0).signum();
            push(r, c1, &mut path);
        }
        path
    }
}

impl Effect for PacketFlow {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let rate = get_f32(ctx.params, "rate", 1.0);
        let trail_p = get_f32(ctx.params, "trail", 0.5);
        let layout = ctx.layout;
        let nk = layout.keys.len();

        // decay tails
        let keep = 0.5f32.powf(ctx.dt / (0.05 + 0.35 * trail_p));
        for c in self.trail.iter_mut() {
            *c = c.scale(keep);
        }

        // traffic intensity ebbs and flows on a slow noise tide
        let ebb = 0.25 + 0.75 * noise2(ctx.t * 0.08, 11.0, self.seed);
        if self.packets.len() < 9 && ctx.rng.chance(ctx.dt * rate * (2.0 + 5.0 * ebb)) {
            for _ in 0..6 {
                let a = ctx.rng.below(nk);
                let b = ctx.rng.below(nk);
                let man = (layout.keys[a].row as i32 - layout.keys[b].row as i32).abs()
                    + (layout.keys[a].col as i32 - layout.keys[b].col as i32).abs();
                if a == b || man < 6 {
                    continue;
                }
                let path = self.route(layout, a, b);
                if path.len() < 4 {
                    continue;
                }
                let hue = ctx.rng.f32();
                let speed = ctx.rng.range(7.0, 13.0);
                self.packets.push(Packet { path, pos: 0.0, speed, hue });
                break;
            }
        }

        for p in self.packets.iter_mut() {
            p.pos += p.speed * ctx.dt;
        }

        // collisions: two heads on one key -> white flash, both die
        let mut dead = vec![false; self.packets.len()];
        let heads: Vec<usize> = self
            .packets
            .iter()
            .map(|p| p.path[(p.pos.round() as usize).min(p.path.len() - 1)])
            .collect();
        for a in 0..heads.len() {
            for b in a + 1..heads.len() {
                if heads[a] == heads[b] && !(dead[a] && dead[b]) {
                    dead[a] = true;
                    dead[b] = true;
                    self.flashes.push((heads[a], 0.3));
                }
            }
        }
        for (pi, p) in self.packets.iter().enumerate() {
            if p.pos >= (p.path.len() - 1) as f32 {
                dead[pi] = true; // delivered
            }
        }
        let mut di = 0;
        self.packets.retain(|_| {
            let d = dead[di];
            di += 1;
            !d
        });

        // routes glow faintly while in use
        for p in &self.packets {
            let g = ctx.palette.sample(p.hue).scale(0.05);
            for &k in &p.path {
                out.add(layout.keys[k].led, g);
            }
        }
        // tails
        for (ki, c) in self.trail.iter().enumerate() {
            out.max(layout.keys[ki].led, *c);
        }
        // heads: hot tip interpolated across the cell boundary
        for p in &self.packets {
            let i = p.pos.floor().max(0.0) as usize;
            let f = p.pos - p.pos.floor();
            let col = ctx.palette.sample(p.hue);
            let head = col.max(Col::WHITE.scale(0.35));
            let k0 = p.path[i.min(p.path.len() - 1)];
            self.trail[k0] = self.trail[k0].max(col);
            out.max(layout.keys[k0].led, head.scale(1.0 - 0.6 * f));
            if i + 1 < p.path.len() {
                out.max(layout.keys[p.path[i + 1]].led, head.scale(f));
            }
        }
        // collision flashes bleed onto neighbors
        for f in self.flashes.iter_mut() {
            f.1 -= ctx.dt;
        }
        self.flashes.retain(|f| f.1 > 0.0);
        for &(k, life) in &self.flashes {
            let a = (life / 0.3).clamp(0.0, 1.0);
            out.max(layout.keys[k].led, Col::WHITE.scale(a * a));
            for &n in &layout.neighbors[k] {
                out.max(layout.keys[n].led, Col::WHITE.scale(a * a * 0.35));
            }
        }
    }
}

// ---------------------------------------------------------------- Life

struct Life {
    grid: Grid,
    alive: Vec<bool>,
    /// Palette position each live cell carries; newborns inherit a circular
    /// mean of their three parents' genes, plus mutation.
    gene: Vec<f32>,
    age: Vec<f32>,
    ghost: Vec<f32>,
    ghost_gene: Vec<f32>,
    acc: f32,
    recent: [u64; 4],
    recent_i: usize,
    stagnant: u32,
}

impl Life {
    fn step(&mut self, rng: &mut Rng, mutation: f32) {
        let (w, h) = (self.grid.w, self.grid.h);
        let n = w * h;
        let mut next_alive = vec![false; n];
        let mut next_gene = self.gene.clone();
        for r in 0..h {
            for c in 0..w {
                let i = r * w + c;
                let mut cnt = 0;
                let (mut sx, mut sy) = (0.0f32, 0.0f32);
                for dr in [h - 1, 0, 1] {
                    for dc in [w - 1, 0, 1] {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let j = ((r + dr) % h) * w + (c + dc) % w;
                        if self.alive[j] {
                            cnt += 1;
                            let a = self.gene[j] * TAU;
                            sx += a.cos();
                            sy += a.sin();
                        }
                    }
                }
                if self.alive[i] {
                    next_alive[i] = cnt == 2 || cnt == 3;
                    if !next_alive[i] {
                        self.ghost[i] = 1.0;
                        self.ghost_gene[i] = self.gene[i];
                    }
                } else if cnt == 3 {
                    next_alive[i] = true;
                    let mean = sy.atan2(sx) / TAU;
                    next_gene[i] = (mean + mutation * rng.range(-0.12, 0.12)).rem_euclid(1.0);
                    self.age[i] = 0.0;
                }
            }
        }
        // stagnation watch: fnv over the live set catches still lifes and
        // short-period oscillators (up to p4 with a 4-slot history)
        let mut hsh = 0xcbf29ce484222325u64;
        let mut pop = 0usize;
        for (i, &a) in next_alive.iter().enumerate() {
            if a {
                pop += 1;
                hsh ^= i as u64 + 1;
                hsh = hsh.wrapping_mul(0x100000001b3);
            }
        }
        if self.recent.contains(&hsh) {
            self.stagnant += 1;
        } else {
            self.stagnant = 0;
        }
        self.recent[self.recent_i] = hsh;
        self.recent_i = (self.recent_i + 1) % 4;
        self.alive = next_alive;
        self.gene = next_gene;
        if pop < 3 || self.stagnant > 10 {
            self.reseed(rng);
        }
    }

    fn reseed(&mut self, rng: &mut Rng) {
        for i in 0..self.alive.len() {
            self.alive[i] = rng.chance(0.3);
            self.gene[i] = rng.f32();
            self.age[i] = 0.0;
        }
        self.recent = [0; 4];
        self.stagnant = 0;
    }
}

impl Effect for Life {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let step_rate = get_f32(ctx.params, "step_rate", 3.0);
        let mutation = get_f32(ctx.params, "mutation", 0.35);

        self.acc += ctx.dt * step_rate;
        let mut guard = 0;
        while self.acc >= 1.0 && guard < 4 {
            self.step(ctx.rng, mutation);
            self.acc -= 1.0;
            guard += 1;
        }
        self.acc = self.acc.min(1.0);

        for i in 0..self.alive.len() {
            if self.alive[i] {
                self.age[i] += ctx.dt;
            }
            self.ghost[i] = (self.ghost[i] - ctx.dt * 1.6).max(0.0);
        }

        let w = self.grid.w;
        for k in &ctx.layout.keys {
            let i = k.row as usize * w + k.col as usize;
            if self.alive[i] {
                let a = self.age[i];
                let ramp = 0.45 + 0.55 * smoothstep(0.0, 0.3, a);
                let newborn = 1.0 - smoothstep(0.0, 0.18, a);
                let c = ctx.palette.sample(self.gene[i]).scale(ramp);
                out.set(k.led, c.add(Col::WHITE.scale(0.35 * newborn)));
            } else if self.ghost[i] > 0.0 {
                let g = self.ghost[i];
                out.set(k.led, ctx.palette.sample(self.ghost_gene[i]).scale(0.30 * g * g));
            }
        }
    }
}

// ---------------------------------------------------------------- RuleCascade

struct RuleCascade {
    grid: Grid,
    /// Waterfall history, row 0 = newest generation (top of the board).
    hist: Vec<bool>,
    /// Snapshot of the previous rule's output, shown below the wipe line.
    old: Vec<bool>,
    acc: f32,
    rule: u8,
    auto_i: usize,
    auto_t: f32,
    wipe: f32,
}

impl RuleCascade {
    fn switch(&mut self, rule: u8, rng: &mut Rng) {
        self.old.copy_from_slice(&self.hist);
        for v in self.hist.iter_mut() {
            *v = false;
        }
        for c in 0..self.grid.w {
            self.hist[c] = rng.chance(0.3);
        }
        self.rule = rule;
        self.wipe = 0.0;
    }

    fn step(&mut self, rng: &mut Rng) {
        let (w, h) = (self.grid.w, self.grid.h);
        let mut new = vec![false; w];
        let mut live = 0usize;
        for c in 0..w {
            let l = self.hist[(c + w - 1) % w] as u8;
            let m = self.hist[c] as u8;
            let r = self.hist[(c + 1) % w] as u8;
            let idx = (l << 2) | (m << 1) | r;
            let b = (self.rule >> idx) & 1 == 1;
            new[c] = b;
            live += b as usize;
        }
        // keep the tape printing: kick a dead/saturated row back to life
        if live == 0 {
            for _ in 0..3 {
                new[rng.below(w)] = true;
            }
        } else if live == w {
            for _ in 0..3 {
                new[rng.below(w)] = false;
            }
        }
        self.hist.copy_within(0..(h - 1) * w, w);
        self.hist[..w].copy_from_slice(&new);
    }
}

impl Effect for RuleCascade {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let sel = get_str(ctx.params, "rule", "auto");
        let gen_rate = get_f32(ctx.params, "gen_rate", 5.0);

        match sel {
            "30" | "90" | "110" => {
                let rb = if sel == "30" {
                    30
                } else if sel == "90" {
                    90
                } else {
                    110
                };
                if rb != self.rule {
                    self.switch(rb, ctx.rng);
                }
            }
            _ => {
                self.auto_t -= ctx.dt;
                if self.auto_t <= 0.0 {
                    self.auto_t = 30.0;
                    self.auto_i = (self.auto_i + 1) % 3;
                    let rb = [30u8, 90, 110][self.auto_i];
                    self.switch(rb, ctx.rng);
                }
            }
        }
        if self.wipe < 1.0 {
            self.wipe = (self.wipe + ctx.dt / 0.7).min(1.0);
        }

        self.acc += ctx.dt * gen_rate;
        let mut guard = 0;
        while self.acc >= 1.0 && guard < 6 {
            self.step(ctx.rng);
            self.acc -= 1.0;
            guard += 1;
        }
        self.acc = self.acc.min(1.0);

        let (w, h) = (self.grid.w, self.grid.h);
        // live cells color by how busy their column is
        let mut dens = vec![0.0f32; w];
        for c in 0..w {
            let mut cnt = 0;
            for r in 0..h {
                cnt += self.hist[r * w + c] as usize;
            }
            dens[c] = cnt as f32 / h as f32;
        }

        let boundary = self.wipe * (h as f32 + 1.0);
        for k in &ctx.layout.keys {
            let (r, c) = (k.row as usize, k.col as usize);
            let idx = r * w + c;
            let on = if (r as f32) < boundary - 0.5 { self.hist[idx] } else { self.old[idx] };
            if on {
                let fade = 1.0 - 0.62 * r as f32 / (h - 1).max(1) as f32;
                let mut col = ctx.palette.sample_clamped(0.2 + 0.7 * dens[c]).scale(fade);
                if r == 0 {
                    col = col.add(Col::WHITE.scale(0.25)); // fresh print head
                }
                out.set(k.led, col);
            } else {
                out.set(k.led, ctx.palette.sample_clamped(0.05).scale(0.05));
            }
            // the wipe line itself
            if self.wipe < 1.0 && ((r as f32 + 0.5) - boundary).abs() < 0.6 {
                out.max(k.led, ctx.palette.sample_clamped(0.95).max(Col::WHITE.scale(0.5)));
            }
        }
    }
}

// ---------------------------------------------------------------- Bitcrush

struct Bitcrush {
    seed: u32,
}

impl Effect for Bitcrush {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let depth = get_f32(ctx.params, "crush", 0.7);
        let dither = get_bool(ctx.params, "dither", true);

        // the crush level itself wanders: slow swell, noisy modulation and
        // occasional sharp drops, like a codec fighting for bandwidth
        let swell = 0.5 + 0.5 * (ctx.t * 0.33).sin();
        let jitter = noise2(ctx.t * 0.21, 4.2, self.seed);
        let spike = smoothstep(0.72, 0.92, noise2(ctx.t * 0.9, 9.1, self.seed));
        let crush = (depth * (0.25 + 0.55 * swell * (0.5 + 0.5 * jitter) + 0.6 * spike)).clamp(0.0, 1.0);
        let bits = (8.0 - 7.0 * crush).round().clamp(1.0, 8.0);
        let lf = (2.0f32).powf(bits);

        // 2x2 Bayer thresholds keyed by (row, col) parity
        const BAYER: [f32; 4] = [0.125, 0.625, 0.875, 0.375];

        for k in &ctx.layout.keys {
            // at very low depth the sample grid itself goes blocky
            let (sx, sy) = if bits <= 2.5 {
                ((k.cx / 0.31).floor() * 0.31, (k.cy / 0.31).floor() * 0.31)
            } else {
                (k.cx, k.cy)
            };
            let u = fbm3(sx * 0.8, sy * 0.8, ctx.t * 0.05, self.seed);
            let v = fbm3(sx * 0.6 + 17.0, sy * 0.6, ctx.t * 0.045, self.seed ^ 0x51ED);
            let th = if dither { BAYER[((k.row & 1) * 2 + (k.col & 1)) as usize] } else { 0.5 };
            let uq = ((u * lf + th).floor() / lf).clamp(0.0, 1.0);
            let vq = ((v * lf + th).floor() / lf).clamp(0.0, 1.0);
            let c = ctx.palette.sample(uq * 0.88 + ctx.t * 0.01);
            out.set(k.led, c.scale(0.18 + 0.82 * vq));
        }
    }
}

// ---------------------------------------------------------------- Firewall

struct Incident {
    keys: Vec<usize>,
    r0: i32,
    r1: i32,
    c0: i32,
    c1: i32,
    /// 0 = alert flash, 1 = quarantine box, 2 = flush off the edge.
    stage: u8,
    t: f32,
    dir: i32,
}

struct Firewall {
    wmax: i32,
    scan: f32,
    dir: f32,
    /// (key, life, peak): probe pings are bright, diagnostic ticks are dim.
    blips: Vec<(usize, f32, f32)>,
    incident: Option<Incident>,
}

impl Effect for Firewall {
    fn render(&mut self, ctx: &mut RenderCtx, out: &mut Frame) {
        let scan_rate = get_f32(ctx.params, "scan_rate", 1.0);
        let threat = get_f32(ctx.params, "threat", 0.5);
        let layout = ctx.layout;

        // scanline ping-pongs across the board
        self.scan += self.dir * scan_rate * 0.55 * ctx.dt;
        if self.scan > layout.aspect {
            self.scan = layout.aspect;
            self.dir = -1.0;
        }
        if self.scan < 0.0 {
            self.scan = 0.0;
            self.dir = 1.0;
        }

        // advance the incident state machine
        let mut clear = false;
        if let Some(inc) = &mut self.incident {
            inc.t += ctx.dt;
            match inc.stage {
                0 => {
                    if inc.t > 0.9 {
                        inc.stage = 1;
                        inc.t = 0.0;
                    }
                }
                1 => {
                    if inc.t > 1.6 {
                        inc.stage = 2;
                        inc.t = 0.0;
                    }
                }
                _ => {
                    let off = (inc.t * 16.0) as i32 * inc.dir;
                    if (inc.dir > 0 && inc.c0 + off > self.wmax) || (inc.dir < 0 && inc.c1 + off < 0) {
                        clear = true;
                    }
                }
            }
        }
        if clear {
            self.incident = None;
        }
        let quiet = self.incident.is_none();

        // calm dark hum between events
        for k in &layout.keys {
            let breathe = 0.5 + 0.5 * (ctx.t * 0.5 + k.cx * 0.4).sin();
            let base = ctx.palette.sample_clamped(0.10 + 0.08 * breathe);
            out.set(k.led, base.scale(0.10 + 0.05 * breathe));
        }
        // faint diagnostic ticks
        if quiet && self.blips.len() < 14 && ctx.rng.chance(ctx.dt * 1.8) {
            self.blips.push((ctx.rng.below(layout.keys.len()), 0.4, 0.22));
        }

        // scanline glow + probes
        let scan_col = ctx.palette.sample_clamped(0.75);
        let mut spawn_at: Option<usize> = None;
        for (ki, k) in layout.keys.iter().enumerate() {
            let d = k.cx - self.scan;
            let g = (-d * d * 90.0).exp();
            if g > 0.003 {
                out.add(k.led, scan_col.scale(g * if quiet { 0.5 } else { 0.15 }));
            }
            if quiet && d.abs() < 0.09 && self.blips.len() < 14 && ctx.rng.chance(ctx.dt * 4.0) {
                self.blips.push((ki, 0.3, 0.9)); // probe ping
                if ctx.rng.chance(threat * 0.12) {
                    spawn_at = Some(ki);
                }
            }
        }

        // a probe found something: mark the infected cluster
        if let (Some(ki), true) = (spawn_at, quiet) {
            let mut members = vec![ki];
            for &n in &layout.neighbors[ki] {
                if ctx.rng.chance(0.8) {
                    members.push(n);
                }
            }
            if ctx.rng.chance(0.4) {
                let ring: Vec<usize> =
                    members.iter().flat_map(|&m| layout.neighbors[m].iter().copied()).collect();
                for n in ring {
                    if !members.contains(&n) && ctx.rng.chance(0.25) {
                        members.push(n);
                    }
                }
            }
            let (mut r0, mut r1, mut c0, mut c1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            for &m in &members {
                r0 = r0.min(layout.keys[m].row as i32);
                r1 = r1.max(layout.keys[m].row as i32);
                c0 = c0.min(layout.keys[m].col as i32);
                c1 = c1.max(layout.keys[m].col as i32);
            }
            let dir = if c0 + c1 > self.wmax { 1 } else { -1 };
            self.incident = Some(Incident { keys: members, r0, r1, c0, c1, stage: 0, t: 0.0, dir });
        }

        // blips
        for b in self.blips.iter_mut() {
            b.1 -= ctx.dt;
        }
        self.blips.retain(|b| b.1 > 0.0);
        for &(ki, life, peak) in &self.blips {
            let a = (life / 0.35).clamp(0.0, 1.0);
            let bright = peak * a * a;
            let c = ctx.palette.sample_clamped(0.9).scale(bright).max(Col::WHITE.scale(bright * 0.35));
            out.max(layout.keys[ki].led, c);
        }

        // incident theater
        if let Some(inc) = &self.incident {
            let red = Col::hex(0xFF2028); // alert accent, deliberately off-palette
            match inc.stage {
                0 => {
                    // hard square-wave flash: unmistakably an alarm
                    let on = (inc.t * 6.0) as i32 % 2 == 0;
                    let lvl = if on { 0.9 } else { 0.18 };
                    for &ki in &inc.keys {
                        out.max(layout.keys[ki].led, red.scale(lvl));
                    }
                }
                1 => {
                    let pulse = 0.35 + 0.35 * (0.5 + 0.5 * (inc.t * 7.0).sin());
                    for &ki in &inc.keys {
                        out.max(layout.keys[ki].led, red.scale(pulse));
                    }
                    // quarantine wall: the box border one cell out
                    let wall = ctx.palette.sample_clamped(0.9).max(Col::WHITE.scale(0.55));
                    let (br0, br1, bc0, bc1) = (inc.r0 - 1, inc.r1 + 1, inc.c0 - 1, inc.c1 + 1);
                    for k in &layout.keys {
                        let (r, c) = (k.row as i32, k.col as i32);
                        if r >= br0
                            && r <= br1
                            && c >= bc0
                            && c <= bc1
                            && (r == br0 || r == br1 || c == bc0 || c == bc1)
                        {
                            out.max(k.led, wall.scale(0.8));
                        }
                    }
                }
                _ => {
                    // flush: the whole boxed mass slides off the nearest edge
                    let off = (inc.t * 16.0) as i32 * inc.dir;
                    let front = if inc.dir > 0 { inc.c1 + off } else { inc.c0 + off };
                    for k in &layout.keys {
                        let (r, c) = (k.row as i32, k.col as i32);
                        if r < inc.r0 || r > inc.r1 {
                            continue;
                        }
                        if c >= inc.c0 + off && c <= inc.c1 + off {
                            out.max(k.led, red.scale(0.6));
                        }
                        if c == front {
                            out.max(k.led, Col::WHITE.scale(0.8));
                        }
                    }
                }
            }
        }
    }
}
