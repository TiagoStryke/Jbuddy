// Remove o fundo cinza dos mascotes (assets-gen/*.png) → transparentes em
// src/assets/mascot/. Pura JS (pngjs), sem dependência nativa.
//
// Método: flood-fill a partir das BORDAS removendo pixels "de fundo" (cinza,
// dessaturado, não-verde). Os olhos/highlights ficam protegidos por serem
// interiores (não conectados à borda). Borda do mascote ganha leve feather.
//
// Uso: node tools/remove-bg.mjs

import { readdirSync, readFileSync, writeFileSync, mkdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { PNG } from "pngjs";
import jpeg from "jpeg-js";

const SRC = join(process.cwd(), "assets-gen");
const OUT_ROOT = join(process.cwd(), "src/assets/mascot");

// Um pixel é "fundo" se é dessaturado (cinza) E não tem dominância de verde.
function isBg(r, g, b) {
  const mx = Math.max(r, g, b);
  const mn = Math.min(r, g, b);
  const greenDom = g - (r + b) / 2;
  return mx - mn < 32 && greenDom < 12;
}

function process_(color, file) {
  // imagens do Pollinations são JPEG (apesar da extensão); decodifica pra RGBA.
  const { width: w, height: h, data } = jpeg.decode(
    readFileSync(join(SRC, color, file)),
    { formatAsRGBA: true },
  );
  const idx = (x, y) => (y * w + x) * 4;
  const removed = new Uint8Array(w * h); // 1 = virou fundo (alpha 0)
  const stack = [];
  const push = (x, y) => {
    if (x < 0 || y < 0 || x >= w || y >= h) return;
    const p = y * w + x;
    if (removed[p]) return;
    const i = p * 4;
    if (!isBg(data[i], data[i + 1], data[i + 2])) return;
    removed[p] = 1;
    data[i + 3] = 0;
    stack.push(x, y);
  };
  // semente: todas as bordas
  for (let x = 0; x < w; x++) {
    push(x, 0);
    push(x, h - 1);
  }
  for (let y = 0; y < h; y++) {
    push(0, y);
    push(w - 1, y);
  }
  while (stack.length) {
    const y = stack.pop();
    const x = stack.pop();
    push(x + 1, y);
    push(x - 1, y);
    push(x, y + 1);
    push(x, y - 1);
  }
  // feather: pixel mantido que faz fronteira com removido fica semi-transparente
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const p = y * w + x;
      if (removed[p]) continue;
      let edge = false;
      if (x > 0 && removed[p - 1]) edge = true;
      else if (x < w - 1 && removed[p + 1]) edge = true;
      else if (y > 0 && removed[p - w]) edge = true;
      else if (y < h - 1 && removed[p + w]) edge = true;
      if (edge) data[p * 4 + 3] = 200;
    }
  }
  const out = new PNG({ width: w, height: h });
  Buffer.from(data.buffer, data.byteOffset, data.length).copy(out.data);
  mkdirSync(join(OUT_ROOT, color), { recursive: true });
  writeFileSync(join(OUT_ROOT, color, file), PNG.sync.write(out));
  const kept = removed.reduce((a, v) => a + (v ? 0 : 1), 0);
  console.log(`${color}/${file}... ✓ (${Math.round((kept / (w * h)) * 100)}% mantido)`);
}

const colors = readdirSync(SRC).filter((c) => statSync(join(SRC, c)).isDirectory());
for (const color of colors) {
  for (const f of readdirSync(join(SRC, color)).filter((f) => f.endsWith(".png"))) {
    try {
      process_(color, f);
    } catch (e) {
      console.log(`${color}/${f}... ✗`, e.message);
    }
  }
}
console.log("\ntransparentes em:", OUT_ROOT);
