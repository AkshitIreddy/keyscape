<div align="center">

<img src="ui/src-tauri/icons/icon-128.png" alt="Keyscape" width="96">

# Keyscape

**Per-key RGB lighting engine for the ASUS ROG Strix SCAR 16 — 50 hand-built effects, music reactivity, and a premium desktop UI at a fraction of Armoury Crate's footprint.**

![Version](https://img.shields.io/badge/version-0.2.0-7c5cff)
![Platform](https://img.shields.io/badge/platform-Windows%2011-0078d4)
![Rust](https://img.shields.io/badge/core-Rust-orange)
![UI](https://img.shields.io/badge/ui-Tauri%202-24C8DB)
![License](https://img.shields.io/badge/license-MIT-22d3a5)

*13 MB RAM · ~2% of one CPU core while animating · ~5% GPU with the window open · zero when closed*

</div>

---

## Table of contents

- [Why](#why)
- [Features](#features)
- [The effects](#the-effects)
- [Architecture](#architecture)
- [Hardware protocol](#hardware-protocol)
- [Getting started](#getting-started)
- [Usage](#usage)
- [Settings & configuration](#settings--configuration)
- [Performance](#performance)
- [Development](#development)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

## Why

Armoury Crate ships a lighting stack that idles at hundreds of MB and offers a
handful of stock effects. Keyscape replaces it on the G634JZ with a tiny
always-on daemon plus an optional UI — every LED on the machine (per-key
keyboard, lid logo, front wrap-around bar, rear lid strip) driven directly
over HID, with effects you won't find anywhere else.

## Features

- **50 unique effects** in 7 categories — deliberately *no* static color, plain
  rainbow wave, breathing, or basic keypress ripple
- **Everything parameterized** — speed, intensity, palette (22 built-ins or
  custom stops), key masks, plus per-effect controls; the UI generates all
  editors from the engine's schema
- **Playlist / shuffle** — rotate any subset of effects on a timer
- **Typing-reactive effects** that see key *positions* only, with the input
  hook alive only while such an effect runs
- **Music mode (strictly opt-in)** — WASAPI loopback (what's playing, never
  the mic), FFT + beat detection off the render path, modulating the *active
  effect* rather than replacing it
- **All four lighting zones** — keyboard, lid logo, front light bar, and the
  33-segment rear lid strip, with zone power management
- **Live preview** — the UI paints the exact wire bytes on the real key
  geometry, streamed from the daemon
- **Vendor-service guard** — detects Armoury Crate's LightingService,
  counters it with keepalive resends, and offers a reversible one-click
  permanent fix
- **Near-zero cost** — event-driven renderer, HID writes only when pixels
  change, 4 fps self-throttle on static scenes, capture threads that exist
  only while needed

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

## Architecture

The defining decision is a **two-process split**:

```
┌────────────────────────────┐        ┌─────────────────────────────┐
│  Keyscape.exe (UI shell)   │        │  keyscape-core.exe (daemon) │
│  Tauri 2 window            │  WS    │  effect engine @ 30 fps cap │
│  live preview canvas       │◄──────►│  HID transport (dirty-block)│
│  gallery / params / config │ 53971  │  WASAPI loopback DSP        │
│  runs only while open      │        │  keyboard hook (on demand)  │
└────────────────────────────┘        │  LightingService guard      │
                                      │  tray icon · settings       │
                                      └──────────────┬──────────────┘
                                                     │ HID feature reports
                                      ┌──────────────▼──────────────┐
                                      │ ASUS N-KEY device 0B05:19B6 │
                                      │ keys · logo · bars (210 LED)│
                                      └─────────────────────────────┘
```

Closing the UI never interrupts the lighting. The UI auto-starts the core;
the core's tray icon opens the UI back up. The control API is JSON over
WebSocket on `127.0.0.1:53971` (loopback only) — see
[core/src/ipc.rs](core/src/ipc.rs) for the ops.

## Hardware protocol

Keyscape drives the ASUS N-KEY device (`0B05:19B6`) directly — no OpenRGB
server, Aura SDK, or vendor service at runtime. Established against this
exact machine (protocol lineage: OpenRGB's Aura-USB family + asusctl's
`rog-aura`, corrected with ASUS's own per-key table):

| Command | Bytes | Purpose |
| --- | --- | --- |
| Direct color | `5D BC 00 01 <bank> 01 <start> <count> 00` + RGB | 16-LED blocks; bank `01` = keys 0-166, bank `04` = aux 167-209 |
| Brightness | `5D BA C5 C4 <0-3>` | must be nonzero or colors are invisible |
| Zone power | `5D BD 01` + u32 LE flags | gates keyboard/logo/bars/rear per power state |

The authoritative key→LED map comes from ASUS's own
`DeviceContent/G634/G634_US_PERKEY.csv`, extracted into
[core/assets/layout_g634_us.json](core/assets/layout_g634_us.json)
(88 keys with physical rects and scan codes + logo, front bar, and the
33-segment rear strip). Two vendor-data bugs are corrected: swapped
LShift/LAlt scan codes, and the rear strip missing from the generic tables.

## Getting started

**Prereqs:** Rust (MSVC toolchain), Node 20+. WebView2 ships with Windows 11.

```powershell
git clone <this repo> keyscape && cd keyscape
powershell -ExecutionPolicy Bypass -File tools/install.ps1
```

The installer builds everything, copies the binaries to
`%LOCALAPPDATA%\Keyscape\bin`, adds a **Start Menu shortcut**, registers the
lighting core to **start at login**, and launches it. Open "Keyscape" from
the Start Menu.

To update after pulling changes, run the same script again. To uninstall:

```powershell
Remove-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name Keyscape
Remove-Item "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Keyscape.lnk"
Remove-Item -Recurse "$env:LOCALAPPDATA\Keyscape"
```

## Usage

- **Pick effects** in the gallery; every parameter edits live
- **Tray icon** (always there while the core runs): left-click opens the UI,
  right-click pauses lighting or quits the core
- **Core CLI:** `keyscape-core --version | --list | --identify |
  --solid RRGGBB | run <effect_id>`

## Settings & configuration

Grouped **General / ASUS service / Audio / Appearance / Performance** in the
UI; persisted at `%APPDATA%\Keyscape\config.json`. Highlights:

- **ASUS lighting service** — Armoury Crate's LightingService fights for the
  device. Default mode counters it with 2 s keepalive resends; *Settings →
  Disable service* stops it permanently via one UAC prompt (reversible).
- **Music mode is off by default** and only ever captures after you enable it.
- **Typing effects** can be disabled entirely; the hook sees scan codes, not
  characters, and nothing leaves the process.

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

- Effects live in `core/src/effects/<category>.rs`; implement the `Effect`
  trait, register an `EffectInfo`, and the UI picks it up automatically.
- `tools/parse-layout.mjs` regenerates the layout JSON from ASUS's CSV;
  `tools/gen-icon.ps1` + `tools/make-ico.mjs` regenerate the app icon.
- CI builds the workspace on Windows for every push
  ([.github/workflows/ci.yml](.github/workflows/ci.yml)); tagging `v*`
  builds and publishes a release zip
  ([.github/workflows/release.yml](.github/workflows/release.yml)).

## Roadmap

- [ ] Custom palette editor in the UI (engine already accepts custom stops)
- [ ] Per-effect key-mask painting on the preview canvas
- [ ] Effect thumbnails rendered from the actual engine
- [ ] Profiles (work / game / night) with hotkey switching
- [ ] More boards — the effect engine is layout-agnostic; only the HID
  transport and layout JSON are model-specific

## Contributing

Issues and PRs welcome. Keep commits atomic and
[Conventional](https://www.conventionalcommits.org/) (`feat:`, `fix:`,
`perf:`, …), one concern per commit, working states only. New effects should
be genuinely distinctive — if it looks like a stock vendor effect, it doesn't
ship.

## License

[MIT](LICENSE) © 2026 Akshit Ireddy
