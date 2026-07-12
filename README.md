<div align="center">

<img src="ui/src-tauri/icons/icon-128.png" alt="Keyscape" width="96">

# Keyscape

**Per-key RGB lighting engine for ASUS ROG laptops — 50 hand-built effects, your own effects in JavaScript, music reactivity, and a premium desktop UI at a fraction of Armoury Crate's footprint.**

![Version](https://img.shields.io/badge/version-0.4.2-7c5cff)
![Platform](https://img.shields.io/badge/platform-Windows%2011-0078d4)
![Rust](https://img.shields.io/badge/core-Rust-orange)
![UI](https://img.shields.io/badge/ui-Tauri%202-24C8DB)
![License](https://img.shields.io/badge/license-MIT-22d3a5)

*~13 MB RAM · ~2% of one CPU core while animating · ~5% GPU with the window open · zero when the window is closed*

</div>

---

## Table of contents

- [Why](#why)
- [Features](#features)
- [The effects](#the-effects)
- [Custom effects (JavaScript)](#custom-effects-javascript)
- [Tech stack](#tech-stack)
- [Architecture](#architecture)
- [Hardware protocol](#hardware-protocol)
- [Known limitations](#known-limitations)
- [Installation](#installation)
- [Usage](#usage)
- [Settings & customization](#settings--customization)
- [Performance](#performance)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## Why

Armoury Crate's lighting stack idles at hundreds of MB for a handful of stock
effects. Keyscape replaces it with a tiny always-on daemon plus an optional
window — the per-key keyboard, lid logo and front light bar driven directly
over HID, 50 effects you won't find anywhere else, and a scripting system so
you can write (or have an AI write) your own.

It targets the **ASUS "N-KEY" per-key keyboard** (USB `0B05:19B6`) found in
2021-and-newer ASUS ROG laptops. The lighting protocol is shared across that
whole family; the bundled key-layout map is generated per model, so a model
other than the one it was built on needs its own layout file (a one-command
step — see [Development](#development)).

## Features

- **50 unique effects** in 7 categories — deliberately *no* static color, plain
  rainbow wave, breathing, or basic keypress ripple
- **Write your own in JavaScript** — drop a `.js` file in, it appears in the
  gallery instantly; can't code? download the AI prompt and let any chatbot
  write it ([details](#custom-effects-javascript))
- **Everything parameterized** — speed, intensity, palette (22 built-ins), key
  masks, plus per-effect controls; the UI builds every editor from the engine's
  schema, all editing live
- **Playlist / shuffle** — rotate any subset of effects on a timer
- **Typing-reactive effects** that see key *positions* only, with the input
  hook alive only while such an effect runs
- **Music mode (strictly opt-in)** — WASAPI loopback (what's playing, never the
  mic), FFT + beat detection off the render path, modulating the *active
  effect* rather than replacing it
- **Deep customization** — 8 accent themes, fonts, interface scale, sound
  themes, motion, transitions — all searchable in Settings
- **First-run tour** walking through every feature, replayable any time
- **Live preview** — the UI paints the exact wire bytes on the real key
  geometry, streamed from the daemon
- **Vendor-service guard** — detects Armoury Crate's LightingService, counters
  it with keepalive resends, and offers a reversible one-click permanent fix
- **Near-zero cost** — event-driven renderer, HID writes only when pixels
  change, 4 fps self-throttle on static scenes, capture threads that exist only
  while needed

## The effects

| Category | Effects |
| --- | --- |
| **Organic** | Bioluminescence · Firefly Meadow (Kuramoto sync) · Aurora Veil · Thunderstorm · Ocean Tide · Ivy Growth · Coral Reef · Pollen Drift |
| **Physics** | Sandfall · Lava Lamp · Magnetic Poles · Pendulum Wave · Chaos Pendulum · Gravity Wells · Reaction Diffusion · Cloth Ripple |
| **Cosmic** | Meteor Storm · Pulsar · Supernova Cycle · Constellation · Black Hole · Solar Wind |
| **Digital** | Glitch Cascade · Bad Signal · Packet Flow · Game of Life · Rule Cascade · Bitcrush · Firewall |
| **Typing** | Typing Heatmap · Combo Meter · Echo Trails · Chain Lightning · Ink Splash · Tempo Pulse · Whack-a-Key |
| **Ambient** | Nebula Drift · Candlelight · Zen Garden · Moon Phases · Ink in Water · Solar Sync (real local time) · Deep Field |
| **Kinetic** | Swarm · Comet Billiards · Radar Sweep · Snake Trio · Spiral Bloom · Loom Weave · Orrery |

Every effect and its parameters are auto-documented in
[docs/effects.md](docs/effects.md).

## Custom effects (JavaScript)

Effects are plain `.js` files running on an embedded QuickJS engine inside the
core — nothing to install. A file is an `EFFECT` manifest plus a
`render(req)` function returning one `[r,g,b]` per key at ~30 fps, with the
user's palette, key taps and audio handed in each frame:

```js
const EFFECT = { id: "my_waves", name: "My Waves", palette: "oceanic",
  params: [{ key: "scale", label: "Scale", kind: "slider",
             min: 0.5, max: 3, step: 0.1, default: 1 }] };

function render(req) {
  return keys.map(k => {
    const f = (Math.sin(k.cx * (req.params.scale * 3) + req.t) + 1) / 2;
    const c = req.palette[Math.floor(f * 15)];
    return [c[0] * f, c[1] * f, c[2] * f];
  });
}
```

Add it in the app's **Custom** tab (upload validates it and reports exact
errors), or drop it in `%APPDATA%\Keyscape\effects\`. A 60 ms per-frame
interrupt budget means a runaway script costs one aborted frame, not a hung
engine. **Can't code?** The Custom tab has a **Download AI prompt** button — a
self-contained spec file; paste it into ChatGPT / Claude / any AI with a
one-line idea, and upload what it writes. Full tutorial:
[docs/js-effects.md](docs/js-effects.md), examples in
[examples/js-effects](examples/js-effects).

## Tech stack

| Layer | Built with |
| --- | --- |
| **Lighting core** (`keyscape-core.exe`) | Rust — no async runtime, ~6 lazily-spawned threads |
| Device I/O | [`hidapi`](https://crates.io/crates/hidapi) — raw HID feature reports, no vendor SDK |
| Audio (music mode) | [`cpal`](https://crates.io/crates/cpal) — WASAPI loopback capture + a hand-rolled FFT |
| Custom effects | [`rquickjs`](https://crates.io/crates/rquickjs) — an embedded QuickJS engine |
| Control API | [`tungstenite`](https://crates.io/crates/tungstenite) — JSON over a loopback WebSocket |
| Tray, keyboard hook, autostart | [`windows-sys`](https://crates.io/crates/windows-sys) — direct Win32 |
| **Desktop shell** (`Keyscape.exe`) | [Tauri 2](https://tauri.app) over the OS WebView2 |
| Frontend | [Vite](https://vitejs.dev) + TypeScript, **zero runtime dependencies** (~27 KB bundled) |
| Installer | Tauri's NSIS bundler; per-user, no admin |
| CI / releases | GitHub Actions on `windows-latest` |

Everything runs locally; the only network access anywhere is the loopback
WebSocket between the two processes.

## Architecture

The defining decision is a **two-process split**:

```
┌────────────────────────────┐        ┌─────────────────────────────┐
│  Keyscape.exe (UI shell)   │        │  keyscape-core.exe (daemon) │
│  Tauri 2 window            │  WS    │  effect engine @ 30 fps cap │
│  live preview canvas       │◄──────►│  HID transport (dirty-block)│
│  gallery / custom / config │ 53971  │  QuickJS user effects       │
│  runs only while open      │        │  WASAPI loopback DSP         │
└────────────────────────────┘        │  keyboard hook (on demand)  │
                                      │  LightingService guard       │
                                      │  tray icon · settings        │
                                      └──────────────┬──────────────┘
                                                     │ HID feature reports
                                      ┌──────────────▼──────────────┐
                                      │ ASUS N-KEY device 0B05:19B6 │
                                      │ 88 keys · logo · front bar  │
                                      └─────────────────────────────┘
```

Closing the window never interrupts the lighting. The window auto-starts the
core; the core's tray icon opens the window back up. The control API is JSON
over WebSocket on `127.0.0.1:53971` (loopback only) — see
[core/src/ipc.rs](core/src/ipc.rs) for the ops. More in
[docs/architecture.md](docs/architecture.md).

## Hardware protocol

Keyscape drives the ASUS N-KEY device (`0B05:19B6`) directly — no OpenRGB
server, Aura SDK, or vendor service at runtime. Reverse-engineered against this
exact machine and cross-checked with OpenRGB, asusctl's `rog-aura` and
g-helper:

| Command | Bytes | Purpose |
| --- | --- | --- |
| Per-key color | `5D BC 00 01 01 01 <start> <count> 00` + RGB | keyboard LEDs 0-166, 16 per packet |
| Aux color | `5D BC 00 01 04 00 00 00 00` + RGB×11 | positional indices 167-177 (logo 167, front bar 169-174) |
| Brightness | `5D BA C5 C4 <0-3>` | must be nonzero or colors are invisible |
| Zone power | `5D BD 01 3F 0F 77 77 FF` | enables each zone per power state |

The 178-LED map comes from ASUS's own per-key CSV (under
`ROG Live Service/DeviceContent/<model>/`), extracted into
[core/assets/layout_us.json](core/assets/layout_us.json). Two vendor-data bugs
are corrected: swapped LShift/LAlt scan codes, and a 1-based-vs-0-based aux
index (the lid logo is 167, not 168). Full details in
[docs/protocol.md](docs/protocol.md).

## Known limitations

- **The rear lid strip can't show live colors.** It's a firmware-effect-only
  zone that ignores per-LED data and can't hold a color while the keyboard
  streams per-key frames (which it must, for effects) — the two are mutually
  exclusive on this controller, verified exhaustively. It's therefore **off by
  default**; the "Fixed color"/"Follow" options are experimental (they flash
  the board to paint it and won't persist). A color committed once via Armoury
  Crate does survive. The lid logo and front bar are unaffected.
- **One layout bundled.** The effect engine is layout-agnostic and the HID
  transport covers the whole N-KEY family, but the key→LED map shipped here was
  generated for a single model. Other ASUS ROG laptops light up but may map
  keys wrongly until a layout file is generated for them
  ([Development](#development)); non-ASUS laptops are not supported at all.

## Installation

**Requirements:** Windows 11 (or Windows 10 with WebView2, which is
preinstalled on 11), and an ASUS ROG laptop with the N-KEY per-key keyboard
(`0B05:19B6`). No admin rights needed; nothing is installed system-wide.

### Option A — installer (recommended)

1. Download `Keyscape_x64-setup.exe` from the [Releases](../../releases) page.
2. Run it. It's a per-user NSIS installer (no UAC prompt) that places the app
   and lighting core under `%LOCALAPPDATA%\Keyscape` and adds a Start Menu
   entry.
3. Launch **Keyscape** from the Start Menu. On first run it registers the
   lighting core to start at login, seeds the example custom effects, and shows
   a short welcome tour.

A portable zip (`Keyscape-<ver>-windows-x64-portable.zip`, with docs and
examples) is published alongside each release for people who'd rather not run
an installer.

### Option B — build from source

Prerequisites: [Rust](https://rustup.rs) (MSVC toolchain) and
[Node 20+](https://nodejs.org).

```powershell
git clone <this-repo-url> keyscape && cd keyscape
powershell -ExecutionPolicy Bypass -File tools/install.ps1
```

`install.ps1` builds the frontend, the core daemon and the shell, copies the
binaries to `%LOCALAPPDATA%\Keyscape\bin`, adds a **Start Menu shortcut**,
registers the core to **start at login**, and launches it. Re-run it any time
to update. To build just the distributable installer instead, run
`tools/bundle-installer.ps1` (output in `target/release/bundle/nsis/`).

### First steps after installing

- If Armoury Crate is installed, go to **Settings → ASUS lighting service →
  Disable service** so it stops fighting Keyscape for the keyboard (reversible).
- Pick an effect in the gallery; tweak it live in the panel on the right.

### Updating and uninstalling

Update by re-running the installer (or `install.ps1`). To remove everything:

```powershell
Remove-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name Keyscape
Remove-Item "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Keyscape.lnk"
Remove-Item -Recurse "$env:LOCALAPPDATA\Keyscape"
Remove-Item -Recurse "$env:APPDATA\Keyscape"   # settings + custom effects
```

## Usage

- **Pick effects** in the gallery; every parameter edits live
- **Custom** tab: upload, manage and delete your own `.js` effects
- **Tray icon** (always there while the core runs): left-click opens the
  window, right-click pauses lighting or quits the core
- **Core CLI:** `keyscape-core --version | --list | --identify |
  --solid RRGGBB | --dump-docs | run <effect_id>`. `--zone-test` maps which LED
  index drives which physical zone.

## Settings & customization

Grouped **General / ASUS service / Appearance & sound / Performance** in the UI
with a **search box**, persisted at `%APPDATA%\Keyscape\config.json`. The full
list is in [docs/settings.md](docs/settings.md). Highlights:

- **Appearance** — 8 accent themes (recolor the whole app live), fonts,
  interface scale, sound themes, motion, effect-transition length
- **ASUS lighting service** — while it runs, Keyscape counters it with 2 s
  keepalive resends; *Disable service* stops it permanently via one UAC prompt
  (reversible). Recommended.
- **Music mode is off by default** and only captures after you enable it
- **Typing effects** and **autostart** are toggleable; the input hook sees scan
  codes, not characters, and nothing leaves the process
- **Rear bar** — off by default (see [Known limitations](#known-limitations))

The current version shows in **Settings → Performance → About**.

## Performance

| Metric | Value |
| --- | --- |
| Core RAM | ~13 MB |
| Core CPU (animating) | ~2% of one core (debug build measured; release lower) |
| Core CPU (static scene) | ~0 (4 fps idle tick, no HID writes) |
| UI GPU (window open) | ~5% of a 3D engine |
| UI when closed | not running |
| Full-board HID write | ~16 ms — which is why the cap defaults to 30 fps |

## Development

```powershell
cd ui && npm run dev          # frontend with HMR on :5173
cargo run -p keyscape-core    # daemon (talks to real hardware)
cargo build --release         # both binaries
```

- Built-in effects live in `core/src/effects/<category>.rs`; implement the
  `Effect` trait, register an `EffectInfo`, and the UI picks it up
  automatically. User effects are JavaScript (see above).
- **Adding another model:** `tools/parse-layout.mjs` regenerates the layout
  JSON from ASUS's own per-key CSV (it auto-discovers the CSV under
  `%ProgramData%\ASUS\ROG Live Service\DeviceContent\`), so supporting a
  different N-KEY laptop is usually just re-running it on that machine.
  `tools/gen-icon.ps1` + `tools/make-ico.mjs` regenerate the app icon;
  `keyscape-core --dump-docs` regenerates `docs/effects.md`.
- CI builds the workspace on Windows for every push
  ([.github/workflows/ci.yml](.github/workflows/ci.yml)); tagging `v*` builds
  and publishes the installer + portable zip
  ([.github/workflows/release.yml](.github/workflows/release.yml)).

## Contributing

Issues and PRs welcome. Keep commits atomic and
[Conventional](https://www.conventionalcommits.org/) (`feat:`, `fix:`,
`perf:`, …), one concern per commit, working states only. New effects should be
genuinely distinctive — if it looks like a stock vendor effect, it doesn't ship.

## License

[MIT](LICENSE) © 2026 Keyscape contributors
