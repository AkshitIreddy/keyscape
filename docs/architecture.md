# Architecture

## Two processes

| | `keyscape-core.exe` | `Keyscape.exe` |
| --- | --- | --- |
| What | Always-on daemon: all device I/O, effects, settings | Tauri 2 window over the core's WebSocket API |
| Cost | ~13 MB RAM, ~2% of one core animating, ~0 static | Only exists while open (~5% GPU) |
| Lifetime | Login → logout (HKCU Run entry) | Opened from Start Menu / tray |

Closing the window never touches the lighting. The window auto-starts the
core if it's down; the core's tray icon (left-click) opens the window.

## Core threads — all lazy

| Thread | Exists when |
| --- | --- |
| engine | always — owns device, effect, settings (single writer) |
| ipc accept + per-connection | always / while a client is connected |
| guard | always — watches ASUS LightingService every 10 s |
| tray | always — win32 message loop |
| kbd-hook | only while a typing-reactive effect is active |
| audio | only while music mode is enabled |
| js-effect | only while a scripted effect is active |

Capture threads are started/stopped through engine lifecycle hooks — an
orphaned capture thread once burned a core forever, hence the design.

## Render loop (engine thread)

1. `recv_timeout` on the command channel until the next tick deadline —
   commands (param edits, effect switches, taps, audio features) interleave
   with rendering without locks.
2. Tick: advance effect-local time (speed- and music-scaled), render the
   effect into an f32 frame, apply key mask, derive logo/bar colors
   (hue-preserving boost + palette glow floor), quantize with gamma.
3. Send only changed 16-LED blocks over HID. Static scene → engine drops to
   4 fps ticks; nothing changed → zero bus traffic.
4. Every 2 s: re-assert brightness, zone power and aux state (firmware
   resets them on lid/power events). Every ≥12 s: repaint the rear strip via
   built-in static if its quantized color drifted.
5. Preview subscribers get the exact wire bytes over WebSocket.

## IPC

JSON over WebSocket, `127.0.0.1:53971` only. Ops in
[core/src/ipc.rs](../core/src/ipc.rs): status, effects, palettes, layout,
masks, set_effect, set_params, patch_settings, next, subscribe_preview
(binary frames), guard_running/fix/restore, quit. The UI generates every
param editor from the `specs` the core reports — no per-effect UI code.

## Settings

`%APPDATA%\Keyscape\config.json`, engine is the single writer, saves are
debounced and atomic (temp + rename). The UI's own prefs ride along in an
opaque `ui` blob.
