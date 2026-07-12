// Pack the generated images into an ICO. Sizes <= 64 are stored as classic
// BMP entries (BITMAPINFOHEADER + bottom-up BGRA + AND mask) because parts
// of the Windows shell refuse PNG-encoded entries below 256px; 256 stays
// PNG-compressed. Run after tools/gen-icon.ps1:  node tools/make-ico.mjs

import { readFileSync, writeFileSync } from "node:fs";

const dir = "ui/src-tauri/icons";
const bmpSizes = [16, 32, 48, 64];
const pngSizes = [256];

function bmpEntry(size) {
  // top-down BGRA dump from gen-icon.ps1
  const bgra = readFileSync(`${dir}/icon-${size}.bgra`);
  const header = Buffer.alloc(40);
  header.writeUInt32LE(40, 0); // biSize
  header.writeInt32LE(size, 4); // biWidth
  header.writeInt32LE(size * 2, 8); // biHeight: XOR + AND masks
  header.writeUInt16LE(1, 12); // biPlanes
  header.writeUInt16LE(32, 14); // biBitCount
  // XOR data, bottom-up
  const xor = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    bgra.copy(xor, y * size * 4, (size - 1 - y) * size * 4, (size - y) * size * 4);
  }
  // AND mask: all zeros (alpha channel governs), rows padded to 32 bits
  const maskStride = Math.ceil(size / 32) * 4;
  const and = Buffer.alloc(maskStride * size);
  return Buffer.concat([header, xor, and]);
}

const entries = [
  ...bmpSizes.map((s) => ({ s, data: bmpEntry(s) })),
  ...pngSizes.map((s) => ({ s, data: readFileSync(`${dir}/icon-${s}.png`) })),
];

const header = Buffer.alloc(6);
header.writeUInt16LE(0, 0);
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(entries.length, 4);

const dirEntries = [];
let offset = 6 + entries.length * 16;
for (const { s, data } of entries) {
  const e = Buffer.alloc(16);
  e.writeUInt8(s >= 256 ? 0 : s, 0);
  e.writeUInt8(s >= 256 ? 0 : s, 1);
  e.writeUInt16LE(1, 4); // planes
  e.writeUInt16LE(32, 6); // bpp
  e.writeUInt32LE(data.length, 8);
  e.writeUInt32LE(offset, 12);
  offset += data.length;
  dirEntries.push(e);
}

writeFileSync(`${dir}/icon.ico`, Buffer.concat([header, ...dirEntries, ...entries.map((e) => e.data)]));
console.log(`wrote ${dir}/icon.ico (${offset} bytes, ${bmpSizes.length} BMP + ${pngSizes.length} PNG entries)`);
