# Keyscape

Per-key RGB lighting engine for the **ASUS ROG Strix SCAR 16 (G634JZ)** with a
library of 50 hand-built effects, music reactivity, and a premium desktop UI —
at a fraction of Armoury Crate's footprint.

<p align="center"><img src="ui/src-tauri/icons/icon-128.png" alt="Keyscape icon"></p>

## Architecture: core / UI split

The defining design decision is **two processes**:

| Process | What it is | Cost |
| --- | --- | --- |
| `keyscape-core.exe` | Always-on daemon: device I/O, effect rendering, audio analysis, settings, tray icon, WebSocket API | ~13 MB RAM, ~2% of one core while animating, near-zero when static |
| `Keyscape.exe` | Tauri window over the core's WebSocket API | only runs while open |

Closing the UI never interrupts the lighting. The UI auto-starts the core if
it isn't running; the core's tray icon (left-click) opens the UI back up.

The renderer is event-driven: capped at 30 fps (configurable 15–60), it only
writes to the USB bus when pixels actually changed, drops itself to 4 fps when
the scene is static, and per-16-LED-block diffing keeps even animated frames
cheap. A full-board HID write costs ~16 ms, which is why 30 fps is the
hardware's sweet spot.

## Hardware path (Windows)

The keyboard is the ASUS N-KEY device `0B05:19B6`. Keyscape drives it
directly over HID feature reports (report `0x5D` on the `0xFF31/0x0079`
collection) using the per-key protocol from OpenRGB's Aura-USB family,
corrected against ASUS's own per-key table for this exact model
(`ROG Live Service/DeviceContent/G634/G634_US_PERKEY.csv`, checked into
[core/assets/layout_g634_us.json](core/assets/layout_g634_us.json) — 88 keys
with LED indices, physical key rects and scan codes, plus lid logo and light
bar). There is no dependency on the OpenRGB server, Aura SDK, or any vendor
service at runtime.

**Armoury Crate contention:** ASUS's `LightingService` writes to the same
device (last-writer-wins). Keyscape handles this in two tiers:

1. **Keepalive mode** (default, no elevation): the guard detects the service
   and the engine re-sends the full frame every 2 s, so Keyscape's output
   always wins within a moment.
2. **Permanent fix** (Settings → ASUS lighting service → *Disable service*):
   one UAC prompt stops and disables `LightingService`. Reversible in-app.
   If the core itself is run elevated it stops the service on startup and
   restarts it on exit (bookkept in `guard.json`, crash-safe).

## Effects

50 effects across 7 categories — deliberately **no** static color, plain
rainbow wave, simple breathing, or basic keypress ripple:

- **Organic** — Bioluminescence, Firefly Meadow (Kuramoto sync), Aurora Veil, Thunderstorm, Ocean Tide, Ivy Growth, Coral Reef, Pollen Drift
- **Physics** — Sandfall, Lava Lamp, Magnetic Poles, Pendulum Wave, Chaos Pendulum, Gravity Wells, Reaction Diffusion, Cloth Ripple
- **Cosmic** — Meteor Storm, Pulsar, Supernova Cycle, Constellation, Black Hole, Solar Wind
- **Digital** — Glitch Cascade, Bad Signal, Packet Flow, Game of Life, Rule Cascade, Bitcrush, Firewall
- **Typing** — Typing Heatmap, Combo Meter, Echo Trails, Chain Lightning, Ink Splash, Tempo Pulse, Whack-a-Key
- **Ambient** — Nebula Drift, Candlelight, Zen Garden, Moon Phases, Ink in Water, Solar Sync (follows real local time), Deep Field
- **Kinetic** — Swarm, Comet Billiards, Radar Sweep, Snake Trio, Spiral Bloom, Loom Weave, Orrery

Every effect exposes speed / intensity / palette (22 built-ins or custom
stops) / key-mask plus its own parameters, all editable live from the UI
(controls are generated from the engine's param schema). A playlist mode
shuffles or sequences any subset on a timer.

**Typing effects** use a low-level keyboard hook that sees key *positions*
only (scan code → LED), runs only while a typing effect is active, and nothing
leaves the engine process. Toggleable in Settings.

**Music mode** (off by default, strictly opt-in) captures what's playing via
WASAPI loopback — never the microphone — and runs a 1024-pt FFT off the render
path. Level, bass/mid/treble, beat onsets and spectral centroid modulate the
*active effect's* speed, brightness and palette rather than replacing it with
a bespoke visualizer.

## Building

Prereqs: Rust (MSVC), Node 20+. WebView2 ships with Windows 11.

```powershell
# frontend
cd ui; npm install; npm run build; cd ..

# core daemon + UI shell (both land in target/release)
cargo build --release
```

Run `target\release\keyscape-core.exe run` (tray icon appears), then open
`target\release\Keyscape.exe` — or just open Keyscape.exe, which starts the
core for you. Useful core commands: `--identify` (list HID interfaces),
`--solid RRGGBB` (transport smoke test), `--list` (effect registry),
`run <effect_id>` (start with a specific effect).

Settings persist at `%APPDATA%\Keyscape\config.json`. The control API is JSON
over WebSocket at `ws://127.0.0.1:53971` (loopback only) — see
[core/src/ipc.rs](core/src/ipc.rs) for the op list.

## Repo layout

```
core/            Rust daemon (no async runtime, ~6 threads, all lazy)
  src/effects/   the 50 effects, one file per category
  assets/        extracted key→LED layout for the G634
ui/              Vite + TypeScript frontend (no runtime deps, ~26 KB)
  src-tauri/     Tauri v2 shell
tools/           layout CSV parser, icon generator
```
