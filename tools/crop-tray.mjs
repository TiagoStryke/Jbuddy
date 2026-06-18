// Recorta JUSTO (bounding box do alpha) os humores usados no tray, deixando o
// blob preenchendo o quadro — assim o ícone aparece maior na barra de menu.
// Lê src/assets/mascot/<cor>/<mood>.png (transparentes) → <cor>/tray/<mood>.png.
//
// Uso: node tools/crop-tray.mjs

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { PNG } from "pngjs";

const ROOT = join(process.cwd(), "src/assets/mascot");
const COLORS = ["green", "pink"];
const MOODS = ["base", "happy", "sleeping", "typing"]; // os que o tray usa

function cropSquare(color, mood) {
  const src = join(ROOT, color, `${mood}.png`);
  if (!existsSync(src)) {
    console.log(`${color}/${mood}... (faltando)`);
    return;
  }
  const img = PNG.sync.read(readFileSync(src));
  const { width: w, height: h, data } = img;
  // bounding box dos pixels com alpha
  let minX = w,
    minY = h,
    maxX = 0,
    maxY = 0;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (data[(y * w + x) * 4 + 3] > 24) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
    }
  }
  const bw = maxX - minX + 1;
  const bh = maxY - minY + 1;
  const side = Math.max(bw, bh);
  const pad = Math.round(side * 0.08);
  const out = side + pad * 2;
  const dst = new PNG({ width: out, height: out });
  dst.data.fill(0);
  const offX = pad + Math.floor((side - bw) / 2);
  const offY = pad + Math.floor((side - bh) / 2);
  for (let y = 0; y < bh; y++) {
    for (let x = 0; x < bw; x++) {
      const si = ((minY + y) * w + (minX + x)) * 4;
      const di = ((offY + y) * out + (offX + x)) * 4;
      dst.data[di] = data[si];
      dst.data[di + 1] = data[si + 1];
      dst.data[di + 2] = data[si + 2];
      dst.data[di + 3] = data[si + 3];
    }
  }
  mkdirSync(join(ROOT, color, "tray"), { recursive: true });
  writeFileSync(join(ROOT, color, "tray", `${mood}.png`), PNG.sync.write(dst));
  console.log(`${color}/tray/${mood}... ✓ ${out}×${out}`);
}

for (const c of COLORS) for (const m of MOODS) cropSquare(c, m);
