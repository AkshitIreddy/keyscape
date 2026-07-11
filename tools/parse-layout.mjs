// Parse ASUS ROG Live Service per-key layout CSV into Keyscape's layout JSON.
//
// Usage: node tools/parse-layout.mjs [path-to-csv] [out-json]
// Default input: C:\ProgramData\ASUS\ROG Live Service\DeviceContent\G634\G634_US_PERKEY.csv
//
// The CSV is the authoritative key->LED map for the G634 (it corrects OpenRGB's
// generic table: Backspace=56, RShift=118, arrows 120/140/141/142). Keyboard
// keys are LED indices 0-166; aux zone 167-177 holds the lid logo (168) and
// light bar segments (169/170/172/173).

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

const IN =
  process.argv[2] ??
  "C:\\ProgramData\\ASUS\\ROG Live Service\\DeviceContent\\G634\\G634_US_PERKEY.csv";
const OUT = process.argv[3] ?? "core/assets/layout_g634_us.json";

// Minimal quoted-CSV line parser (the quote key is encoded as """" in the CSV).
function parseLine(line) {
  const cells = [];
  let cur = "";
  let inQ = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (inQ) {
      if (c === '"' && line[i + 1] === '"') {
        cur += '"';
        i++;
      } else if (c === '"') inQ = false;
      else cur += c;
    } else if (c === '"') inQ = true;
    else if (c === ",") {
      cells.push(cur);
      cur = "";
    } else cur += c;
  }
  cells.push(cur);
  return cells;
}

const RENAME = {
  VOL_DN: "VolDn", VOL_UP: "VolUp", "Mic On/Off": "Mic", HyperFan: "Fan",
  "Armoury Crate": "Rog", Delete: "Del", "~": "`", Minus: "-", Equal: "=",
  Back: "Backspace", PLAY: "Play", STOP: "Stop", Cap: "Caps", '"': "'",
  ENTER: "Enter", PREV: "Prev", L_Shift: "LShift", "?": "/", R_Shift: "RShift",
  UP_ARROW: "Up", NEXT: "Next", L_Ctrl: "LCtrl", L_Fn: "Fn", L_Alt: "LAlt",
  R_Alt: "RAlt", PRTSC: "PrtSc", R_Ctrl: "RCtrl", L_ARROW: "Left",
  DN_ARROW: "Down", R_ARROW: "Right",
};

const AUX_NAMES = {
  168: "Logo",
  169: "LightbarR1",
  170: "LightbarR2",
  172: "LightbarL2",
  173: "LightbarR3",
};

const lines = readFileSync(IN, "utf8").split(/\r?\n/).filter(Boolean);
const keys = [];
const aux = [];

for (const line of lines) {
  const c = parseLine(line);
  if (!c[0]?.startsWith("LED ")) continue;
  const led = parseInt(c[0].slice(4), 10);
  const [gx, gy, exist, x0, y0, x1, y1] = c.slice(1, 8).map(Number);
  const note = c[9] ?? "";
  const keyCode = c[11] ?? "";
  if (exist !== 1) continue;

  if (led <= 166) {
    const name = RENAME[note] ?? note;
    let scan =
      keyCode && keyCode !== "NULL" ? parseInt(keyCode, 16) : null;
    // The vendor CSV swaps LShift/LAlt scan codes; real PS/2 set-1 is
    // LShift=0x2A, LAlt=0x38 (verified live against the keyboard hook).
    if (name === "LShift") scan = 0x2a;
    if (name === "LAlt") scan = 0x38;
    keys.push({ led, name, row: gy, col: gx, px: [x0, y0, x1, y1], scan });
  } else if (AUX_NAMES[led]) {
    aux.push({ led, name: AUX_NAMES[led] });
  }
  // "Rear_N" entries (177+) are a different SKU's rear bar; the proven aux
  // packet on this machine covers 167-177 only, so they are ignored.
}

// Normalize key rects to the keyboard's own bounding box.
const minX = Math.min(...keys.map((k) => k.px[0]));
const minY = Math.min(...keys.map((k) => k.px[1]));
const maxX = Math.max(...keys.map((k) => k.px[2]));
const maxY = Math.max(...keys.map((k) => k.px[3]));
const W = maxX - minX;
const H = maxY - minY;

for (const k of keys) {
  const [x0, y0, x1, y1] = k.px;
  k.x = +((x0 - minX) / W).toFixed(4);
  k.y = +((y0 - minY) / H).toFixed(4);
  k.w = +((x1 - x0) / W).toFixed(4);
  k.h = +((y1 - y0) / H).toFixed(4);
  delete k.px;
}

keys.sort((a, b) => a.led - b.led);
aux.sort((a, b) => a.led - b.led);

const layout = {
  model: "ASUS ROG Strix SCAR 16 G634JZ",
  source: "ASUS ROG Live Service DeviceContent/G634/G634_US_PERKEY.csv",
  led_count: 178, // frame indices 0..177 (keyboard 0..166 + aux 167..177)
  grid: { cols: 21, rows: 7 },
  keys,
  aux,
};

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, JSON.stringify(layout, null, 1) + "\n");
console.log(
  `wrote ${OUT}: ${keys.length} keys, ${aux.length} aux LEDs, bounds ${W}x${H}`
);
