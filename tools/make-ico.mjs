// Pack the generated PNGs into a Vista-style ICO (PNG-compressed entries).
// Run after tools/gen-icon.ps1:  node tools/make-ico.mjs

import { readFileSync, writeFileSync } from "node:fs";

const dir = "ui/src-tauri/icons";
const sizes = [16, 32, 48, 64, 128, 256];
const pngs = sizes.map((s) => ({ s, data: readFileSync(`${dir}/icon-${s}.png`) }));

const header = Buffer.alloc(6);
header.writeUInt16LE(0, 0); // reserved
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(pngs.length, 4);

const entries = [];
let offset = 6 + pngs.length * 16;
for (const { s, data } of pngs) {
  const e = Buffer.alloc(16);
  e.writeUInt8(s >= 256 ? 0 : s, 0); // width (0 = 256)
  e.writeUInt8(s >= 256 ? 0 : s, 1); // height
  e.writeUInt8(0, 2); // palette
  e.writeUInt8(0, 3); // reserved
  e.writeUInt16LE(1, 4); // planes
  e.writeUInt16LE(32, 6); // bpp
  e.writeUInt32LE(data.length, 8);
  e.writeUInt32LE(offset, 12);
  offset += data.length;
  entries.push(e);
}

writeFileSync(`${dir}/icon.ico`, Buffer.concat([header, ...entries, ...pngs.map((p) => p.data)]));
console.log(`wrote ${dir}/icon.ico (${offset} bytes)`);
