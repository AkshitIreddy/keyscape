# Changelog

All notable changes to Keyscape. This file is the source of the release notes
published to GitHub — the release workflow pastes the matching version's
section into each release.

## [0.5.1]

### Added
- Playlist → **Shuffle palettes**: optionally give each effect a random color
  palette when the playlist switches to it, so the same effect looks different
  every time it comes up. The effect's own saved palette isn't changed.

### Fixed
- Settings layout: the two-column panels now pack tightly (masonry) instead of
  leaving awkward vertical gaps where a short panel sat next to a taller one.
- Settings → Global shortcuts: the recorder/clear controls no longer overflow
  the panel on narrow widths — the label yields and the control wraps underneath.

## [0.5.0]

### Added
- **Master lights switch in the sidebar.** One click blanks the keyboard (or
  brings it back) without stopping the background core — the same on/off you
  get from Settings, now always one click away.
- **Global keyboard shortcuts.** Bind system-wide hotkeys — in Settings →
  Global shortcuts — for Toggle lights, Next effect, Toggle playlist, and
  Brightness up/down. They work in any app, even with the Keyscape window
  closed, and only listen for the exact chords you assign (no key logging).
- **Playlist mood presets.** One-click Calm / Energetic / Cosmic / Nature /
  Retro buttons fill the rotation with effects that share a feel, so you can
  set a vibe instead of ticking effects one by one.

### Changed
- The playlist's default "switch every" is now 2 minutes, with a warning below
  1 minute (effects that develop slowly barely appear before the next switch)
  and a 30-second floor.

## [0.4.4]

### Added
- **Automatic keyboard-layout detection.** On startup the core reads your
  laptop's own ASUS per-key data files and builds the key→LED map for your
  exact model, so keys light up in the right places on any ASUS ROG N-KEY
  laptop with no setup. Falls back to the bundled layout if those files aren't
  present.
- A live GitHub-release version badge in the README (never goes stale).

### Changed
- Generalized throughout from a single model to the accurate device family:
  the ASUS "N-KEY" per-key keyboard (`0B05:19B6`) used in 2021+ ROG laptops.

### Fixed
- CI and fresh-checkout source installs failed because the Tauri shell's
  bundled resources weren't staged before compiling; both now build in the
  correct order.

## [0.4.3]

### Changed
- README rewritten: added a tech-stack table and a fuller installation guide,
  removed the roadmap.
- Documentation and UI wording de-branded to the generic device family.

## [0.4.2]

### Added
- **Custom effects manager** (new *Custom* tab): upload, validate, try and
  delete your own JavaScript effects, with an AI prompt file you can hand to
  any chatbot to generate effects without coding.
- **Onboarding tour** on first launch, replayable any time.
- **Deep customization** in Settings: accent-color themes, fonts, interface
  scale, sound themes, effect-transition length — all searchable.
- **Guide** redesigned into grouped, searchable sections.
- Rear-bar mode setting (off / fixed color / follow).

### Changed
- The rear lid strip is **off by default**: it's a firmware-effect-only zone
  that can't hold a color while the keyboard streams per-key effects (a
  hardware limitation, now documented). The lid logo and front bar are
  unaffected.

### Fixed
- Custom effects switched from Python to an embedded JavaScript engine — no
  interpreter to install.

## [0.4.0]

### Added
- The full app: 50 hand-built effects across 7 categories, per-effect
  parameters, 22 palettes, playlist/shuffle, opt-in music-reactive mode, a
  live keyboard preview, and a system-tray lighting core that keeps running
  when the window is closed.
- Windows installer, Start Menu entry, and login autostart.
