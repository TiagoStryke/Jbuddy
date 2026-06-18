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

// "Fundo/sombra" por COR (medido nas imagens): a sombra verde é quase cinza
// (sat baixo), a rosa é tingida (sat médio) — os corpos têm sat bem mais alto.
// O brilho protege o topo claro/highlight (sobretudo no rosa).
const TH = {
  green: { sat: 22, bright: 205 },
  pink: { sat: 52, bright: 150 },
};
function isBg(r, g, b, th) {
  const sat = Math.max(r, g, b) - Math.min(r, g, b);
  const bright = (r + g + b) / 3;
  return sat < th.sat && bright < th.bright;
}

function process_(color, file) {
  // imagens do Pollinations são JPEG (apesar da extensão); decodifica pra RGBA.
  const { width: w, height: h, data } = jpeg.decode(
    readFileSync(join(SRC, color, file)),
    { formatAsRGBA: true },
  );
  const th = TH[color] || TH.green;
  const idx = (x, y) => (y * w + x) * 4;
  const removed = new Uint8Array(w * h); // 1 = virou fundo (alpha 0)
  const stack = [];
  const push = (x, y) => {
    if (x < 0 || y < 0 || x >= w || y >= h) return;
    const p = y * w + x;
    if (removed[p]) return;
    const i = p * 4;
    if (!isBg(data[i], data[i + 1], data[i + 2], th)) return;
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
  // mantém só o MAIOR componente conectado opaco (o mascote) e zera respingos
  // soltos (sombra de chão, manchas) — agnóstico de cor.
  const opaque = (p) => data[p * 4 + 3] > 40;
  const comp = new Int32Array(w * h).fill(-1);
  let best = -1;
  let bestSize = 0;
  let cid = 0;
  const q = [];
  for (let s = 0; s < w * h; s++) {
    if (comp[s] !== -1 || !opaque(s)) continue;
    comp[s] = cid;
    let size = 0;
    q.length = 0;
    q.push(s);
    while (q.length) {
      const p = q.pop();
      size++;
      const x = p % w;
      if (x > 0 && comp[p - 1] === -1 && opaque(p - 1)) {
        comp[p - 1] = cid;
        q.push(p - 1);
      }
      if (x < w - 1 && comp[p + 1] === -1 && opaque(p + 1)) {
        comp[p + 1] = cid;
        q.push(p + 1);
      }
      if (p - w >= 0 && comp[p - w] === -1 && opaque(p - w)) {
        comp[p - w] = cid;
        q.push(p - w);
      }
      if (p + w < w * h && comp[p + w] === -1 && opaque(p + w)) {
        comp[p + w] = cid;
        q.push(p + w);
      }
    }
    if (size > bestSize) {
      bestSize = size;
      best = cid;
    }
    cid++;
  }
  for (let p = 0; p < w * h; p++) {
    if (comp[p] !== best) data[p * 4 + 3] = 0;
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
