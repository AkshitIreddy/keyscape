# Troubleshooting

## Lighting fights back / flickers to ASUS colors
Armoury Crate's LightingService is running. Keyscape's keepalive keeps
winning within ~2 s, but the clean fix is Settings → ASUS lighting service →
**Disable service** (one UAC prompt, reversible in-app).

## Logo / bars / rear strip went dark after sleep or lid close
Firmware resets brightness, zone power and aux state on power events. The
core re-asserts all of it every 2 s — if a zone stays dark longer than that,
check the core is actually running (tray icon present).

## Rear strip shows a stale color
By design it updates at most every ~12 s (each repaint costs a one-frame
board blink — see docs/protocol.md). If it seems stuck for minutes, quit and
relaunch the core from the tray.

## A custom JS effect shows a red pulsing Esc key
The script died: exception, bad return shape, or it blew the 60 ms frame
budget 10 times in a row. Run `keyscape-core.exe run <effect_id>` in a
terminal to see the error output. See docs/js-effects.md.

## Which LED is which? / zones misbehaving
Run the built-in probe (it stops and restarts the core by itself):

```powershell
& "$env:LOCALAPPDATA\Keyscape\bin\keyscape-core.exe" --zone-test
```

## Start Menu icon is blank
Windows icon cache. Sign out and back in, or run `ie4uinit.exe -show`.

## Keyboard does nothing at all
1. `keyscape-core.exe --identify` — the `0xFF31/0x0079` interface must be
   listed.
2. `keyscape-core.exe --solid FF0000` — floods the board red past the whole
   engine; if this works, the transport is fine.
3. Check hardware brightness isn't 0 (Settings → General).

## The window is heavy on GPU
It shouldn't be (~5% of a 3D engine). Appearance → Motion off and Preview
glow off shave it further; minimized windows render nothing.

## Where things live

| Thing | Path |
| --- | --- |
| Binaries | `%LOCALAPPDATA%\Keyscape\bin` |
| Settings | `%APPDATA%\Keyscape\config.json` |
| Custom effects | `%APPDATA%\Keyscape\effects\*.js` |
| Autostart | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Keyscape` |
