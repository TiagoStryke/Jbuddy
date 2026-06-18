// Gerador do mascote do Jbuddy via Gemini 2.5 Flash Image ("Nano Banana").
// Lê as chaves de ~/.config/jbuddy/gemini.keys (uma por linha) e ROTACIONA
// entre elas pra não bater limite. Gera PNGs em src/assets/mascot/.
//
// Uso:
//   node tools/gen-mascot.mjs base          -> gera o mascote base
//   node tools/gen-mascot.mjs moods         -> gera os humores (usa base como referência)
//
// As chaves NÃO ficam no repo. Nada de segredo aqui.

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const MODEL = "gemini-2.5-flash-image";
const KEYS_FILE = join(homedir(), ".config/jbuddy/gemini.keys");
const OUT_DIR = join(process.cwd(), "src/assets/mascot");

const keys = readFileSync(KEYS_FILE, "utf8")
  .split("\n")
  .map((k) => k.trim())
  .filter(Boolean);
if (keys.length === 0) throw new Error("nenhuma chave em " + KEYS_FILE);
let keyIdx = 0;
const nextKey = () => keys[keyIdx++ % keys.length];

mkdirSync(OUT_DIR, { recursive: true });

// Estética: mascote do Jbuddy — encaixa no orbe/anel verde do app.
const STYLE =
  "Mascot character for a friendly desktop wellbeing app. A small, soft, " +
  "rounded blob/bean creature with a smooth matte body in a mint-to-teal " +
  "gradient (fresh green #30d977 to teal #34c8c0), simple expressive dark " +
  "dot eyes and a tiny friendly mouth. Cute but clean and modern, soft 3D / " +
  "vector toy look, gentle soft shading, bold and readable even when small. " +
  "Centered, facing forward, full body, generous margin, plain transparent " +
  "background, no text, no shadow on the ground.";

const MOODS = {
  happy: "The mascot looks happy and energetic, bright cheerful smile, eyes curved with joy.",
  neutral: "The mascot looks calm and content, neutral relaxed expression.",
  tired: "The mascot looks tired and a bit droopy, half-closed sleepy eyes, slightly slumped, one tiny sweat drop.",
  focused: "The mascot looks focused and determined, calm confident eyes, a subtle soft glow around it.",
  sleeping: "The mascot is sleeping peacefully, eyes closed, tiny 'z' floating above.",
  water: "The mascot happily holds a tiny glass of water, taking a sip, cheerful.",
};

async function generate(prompt, refPngPath) {
  const parts = [{ text: prompt }];
  if (refPngPath && existsSync(refPngPath)) {
    const b64 = readFileSync(refPngPath).toString("base64");
    parts.push({ inlineData: { mimeType: "image/png", data: b64 } });
  }
  const body = JSON.stringify({
    contents: [{ parts }],
    generationConfig: { responseModalities: ["IMAGE"] },
  });

  let lastErr = "";
  for (let attempt = 0; attempt < Math.min(keys.length, 6); attempt++) {
    const key = nextKey();
    const url = `https://generativelanguage.googleapis.com/v1beta/models/${MODEL}:generateContent?key=${key}`;
    let res;
    try {
      res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body,
      });
    } catch (e) {
      lastErr = "fetch falhou: " + e.message;
      continue;
    }
    if (res.status === 429 || res.status === 403) {
      lastErr = `HTTP ${res.status} (limite/permissão) — tentando próxima chave`;
      continue;
    }
    const json = await res.json();
    if (!res.ok) {
      lastErr = `HTTP ${res.status}: ${JSON.stringify(json).slice(0, 300)}`;
      // erro de formato/modelo não melhora trocando chave: aborta
      if (res.status === 400 || res.status === 404) break;
      continue;
    }
    const imgPart = json?.candidates?.[0]?.content?.parts?.find(
      (p) => p.inlineData?.data,
    );
    if (!imgPart) {
      lastErr = "resposta sem imagem: " + JSON.stringify(json).slice(0, 300);
      break;
    }
    return Buffer.from(imgPart.inlineData.data, "base64");
  }
  throw new Error(lastErr || "falhou");
}

const mode = process.argv[2] || "base";

if (mode === "base") {
  console.log("Gerando mascote base...");
  const img = await generate(
    `${STYLE} The mascot has a neutral friendly expression. This is the canonical reference pose.`,
  );
  const out = join(OUT_DIR, "base.png");
  writeFileSync(out, img);
  console.log("✓ salvo:", out, `(${img.length} bytes)`);
} else if (mode === "moods") {
  const base = join(OUT_DIR, "base.png");
  if (!existsSync(base)) throw new Error("rode 'base' primeiro (falta base.png)");
  for (const [name, desc] of Object.entries(MOODS)) {
    process.stdout.write(`Gerando '${name}'... `);
    try {
      const img = await generate(
        `Keep EXACTLY the same character, style, colors and proportions as the reference image. ${desc} Plain transparent background, centered, no text.`,
        base,
      );
      const out = join(OUT_DIR, `${name}.png`);
      writeFileSync(out, img);
      console.log("✓", `${img.length} bytes`);
    } catch (e) {
      console.log("✗", e.message);
    }
  }
} else {
  console.log("uso: node tools/gen-mascot.mjs [base|moods]");
}
