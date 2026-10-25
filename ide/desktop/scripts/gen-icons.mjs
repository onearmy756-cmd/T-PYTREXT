#!/usr/bin/env node
/**
 * gen-icons.mjs — tengeneza icons za Tauri (PNG + ICO + ICNS) bila zana za nje.
 *
 * Badala ya heredoc ya Python kwenye workflow ya CI. Hutengeneza PNG chanzo
 * cha RGBA (bluu #3b82f6), kisha hufunga ICO (Windows) na ICNS (macOS).
 *
 *     node scripts/gen-icons.mjs [outDir]
 */
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = resolve(process.argv[2] || join(HERE, "..", "src-tauri", "icons"));
const RGBA = [59, 130, 246, 255];

function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return (~c) >>> 0;
}

function chunk(type, data) {
  const t = Buffer.from(type, "ascii");
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}

function png(w, h, rgba = RGBA) {
  const raw = Buffer.alloc(h * (1 + w * 4));
  for (let y = 0; y < h; y++) {
    const off = y * (1 + w * 4);
    raw[off] = 0; // filter: none
    for (let x = 0; x < w; x++) {
      const p = off + 1 + x * 4;
      raw[p] = rgba[0];
      raw[p + 1] = rgba[1];
      raw[p + 2] = rgba[2];
      raw[p + 3] = rgba[3];
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // colour type RGBA
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function ico(entries) {
  // entries: [{ size, data }]
  const n = entries.length;
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(n, 4);
  const dir = Buffer.alloc(16 * n);
  let offset = 6 + 16 * n;
  const blobs = [];
  entries.forEach((e, i) => {
    const o = i * 16;
    dir[o] = e.size % 256;
    dir[o + 1] = e.size % 256;
    dir[o + 2] = 0;
    dir[o + 3] = 0;
    dir.writeUInt16LE(1, o + 4);
    dir.writeUInt16LE(32, o + 6);
    dir.writeUInt32LE(e.data.length, o + 8);
    dir.writeUInt32LE(offset, o + 12);
    offset += e.data.length;
    blobs.push(e.data);
  });
  return Buffer.concat([header, dir, ...blobs]);
}

function icns(entries) {
  // entries: [{ type: "ic07", data }]
  const parts = [];
  for (const e of entries) {
    const t = Buffer.from(e.type, "ascii");
    const len = Buffer.alloc(4);
    len.writeUInt32BE(e.data.length + 8);
    parts.push(Buffer.concat([t, len, e.data]));
  }
  const body = Buffer.concat(parts);
  const head = Buffer.alloc(8);
  head.write("icns", 0, "ascii");
  head.writeUInt32BE(body.length + 8, 4);
  return Buffer.concat([head, body]);
}

mkdirSync(OUT, { recursive: true });
const sizes = { "32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256, "icon.png": 512 };
const data = {};
for (const [name, s] of Object.entries(sizes)) {
  data[s] = png(s, s);
  writeFileSync(join(OUT, name), data[s]);
}
writeFileSync(
  join(OUT, "icon.ico"),
  ico([
    { size: 32, data: data[32] },
    { size: 128, data: data[128] },
    { size: 256, data: data[256] },
  ]),
);
writeFileSync(
  join(OUT, "icon.icns"),
  icns([
    { type: "ic07", data: data[128] },
    { type: "ic08", data: data[256] },
    { type: "ic09", data: data[512] },
  ]),
);
console.log(`icons ok (RGBA + ico + icns) -> ${OUT}`);
