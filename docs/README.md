# Keyscape documentation

| Page | What's in it |
| --- | --- |
| [architecture.md](architecture.md) | The core/UI split, threads, render loop, IPC |
| [effects.md](effects.md) | Every effect with its parameters (auto-generated) |
| [js-effects.md](js-effects.md) | Writing your own effects in JavaScript |
| [protocol.md](protocol.md) | The G634 HID protocol: packets, zones, quirks |
| [settings.md](settings.md) | Every setting and where it lives |
| [troubleshooting.md](troubleshooting.md) | Fixes for the sharp edges |

Regenerate the effects reference after adding effects:

```powershell
cargo run -p keyscape-core -- --dump-docs > docs/effects.md
```
