# Settings reference

All persisted at `%APPDATA%\Keyscape\config.json`; every entry is editable
from the UI (Settings view) and over IPC (`patch_settings`).

## General

| Setting | Default | Meaning |
| --- | --- | --- |
| Hardware brightness | High (3) | The keyboard's own LED level (1-3; 0 would blank per-key color) |
| Master intensity | 1.0 | Software dimmer over every effect |
| Effect transition | 0.4 s | Crossfade length when switching effects |
| Pause lighting | off | Blanks the board without stopping the core |
| Aux glow | on | Mirror the scene onto the lid logo and front bar |
| Rear bar | off | Rear lid strip: off / fixed color / follow. Experimental — see [troubleshooting](troubleshooting.md); the strip can't hold a color under a live effect |
| Typing effects | on | Allow the key-position hook for typing-reactive effects (scan codes only, never characters, never leaves the process) |
| Start with Windows | on | Register the core for login autostart |

## ASUS lighting service

| Setting | Default | Meaning |
| --- | --- | --- |
| Manage while running | on | Try to stop LightingService at core start (needs elevation), restore on exit |
| Disable service (button) | — | One UAC prompt: stop + disable permanently. Reversible with Re-enable |

While the service runs, the core counters it with 2 s full re-sends
(keepalive mode) — lighting works but ASUS may flash through briefly.

## Audio (music mode)

**Everything off by default; capture only ever starts after you enable it.**

| Setting | Default | Meaning |
| --- | --- | --- |
| Enable music mode | **off** | WASAPI loopback of what's playing (never the microphone) |
| Sensitivity | 1.0 | Input gain into the analysis |
| Amount | 0.7 | How strongly music bends the active effect |
| Brightness / Speed / Palette drift | on / on / off | Which aspects the music modulates |

## Appearance & sound (UI-only)

| Setting | Default | Meaning |
| --- | --- | --- |
| Accent color | Aurora | One of 8 presets; recolors every gradient/highlight live |
| Font | Segoe | Interface typeface (default / classic serif / monospace) |
| Interface size | Normal | Scales the whole window (compact → extra large) |
| Interface sounds | on | Synthesized ticks/chimes (WebAudio, no assets) |
| Sound theme | Soft | Character of those sounds (soft / crisp / chime / retro) |
| Sound volume | 0.4 | Master gain for those |
| Motion | on | Background drift + view transitions (honors OS reduced-motion) |
| Preview glow | on | Bloom pass in the live preview |
| Welcome tour | — | Replay the first-run feature walkthrough |

## Performance

| Setting | Default | Meaning |
| --- | --- | --- |
| Frame rate cap | 30 | 15-60; a full HID write is ~16 ms so 30 is the sweet spot |
| Gamma | 1.8 | Perceptual→LED response curve |

## Playlist

Enable, shuffle vs in-order, interval (30 s – 30 min), and the subset of
effects to rotate (nothing checked = whole library).
