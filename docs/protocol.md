# G634 HID protocol notes

Everything below was established against a real ROG Strix SCAR 16 (G634JZ),
cross-checked with OpenRGB's `AsusAuraCoreLaptop` controller, asusctl's
`rog-aura`, and g-helper. Device: ASUS N-KEY `0B05:19B6`, HID collection
usage page `0xFF31`, usage `0x0079`. All commands are 64-byte **feature**
reports whose first byte is `0x5D`.

## Commands

| Command | Bytes | Notes |
| --- | --- | --- |
| Direct-mode init | `5D BC 01` | once, ~50 ms settle; required after built-in modes |
| Per-key colors | `5D BC 00 01 01 01 <start> <count> 00` + RGB×count | keyboard indices 0-166, 16 per packet |
| Aux colors | `5D BC 00 01 04 00 00 00 00` + RGB×11 | consumed positionally as indices 167-177; start/count ignored |
| Brightness | `5D BA C5 C4 <0-3>` | 0 makes every zone invisible |
| Zone power | `5D BD 01 3F 0F 77 77 FF` | see quirks below |
| Built-in effect | `5D B3 <zone> <mode> R G B <speed> <dir> 00 R2 G2 B2` | zone 0 = whole device, mode 0 = static |
| Apply built-in | `5D B4` | required after `B3` |
| Save to flash | `5D B5` | avoid: flash wear; `B4` alone applies |

## LED index map (178 total)

| Index | Zone |
| --- | --- |
| 0-166 | keyboard keys (per-key) |
| 167 | lid ROG logo (168 mirrored as safety net) |
| 169-174 | front wrap-around bar, right→left physically |
| 176 / 177 | rear lid strip channels — **but see below** |

The vendor CSV (`ROG Live Service/DeviceContent/G634/G634_US_PERKEY.csv`) is
authoritative for keyboard keys but its aux section is **1-based** (logo
listed as "LED 168") and its 33 `Rear_N` rows are an editor canvas, not wire
reality. Two of its scan codes are swapped (LShift/LAlt).

## The rear strip is built-in-only

Verified with a per-index sweep on hardware: no direct index (167-177)
lights the rear strip; it follows **built-in firmware effects only**.
Keyscape colors it by flashing `5D B3` static + `5D B4`, then immediately
re-entering direct mode and resending the frame — keyboard/logo/front bar
snap back to per-key data, the rear strip latches the static color. The
repaint blinks the board for a frame, so it's throttled (quantized color,
≥12 s interval).

## Zone power quirks

A zone whose power bits are off ignores color data entirely. The encoding
that works on this generation is g-helper's, not asusctl's cleaner one:

```
5D BD 01 <keyb+logo> <bar> <lid> <rear> FF
          0x3F        0x0F  0x77  0x77       (boot+awake+sleep, no shutdown)
```

- awake-bar bit is doubled (bits 0 and 2 of the bar byte)
- lid/rear duplicate their low nibble into the high nibble
- the trailing `0xFF` is required

The firmware silently resets brightness/zone-power/aux state on lid and
power events (hibernate especially), so the core re-asserts all three every
2 seconds.

## Timing

A full-board write is ~16.5 ms of blocking control I/O — that's why the
default frame cap is 30 fps and why the transport diffs per 16-LED block.
