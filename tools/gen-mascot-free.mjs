// Gerador GRÁTIS do mascote via Pollinations.ai (Flux). Sem chave.
// Seed fixo + descrição idêntica → consistência. Gera por COR em assets-gen/<cor>/.
//
// Uso: node tools/gen-mascot-free.mjs <green|pink> [seed]

import { writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";

const COLOR = process.argv[2] || "green";
const SEED = process.argv[3] || "4242";
const OUT = join(process.cwd(), "assets-gen", COLOR);
mkdirSync(OUT, { recursive: true });

const BODY = {
  green: "smooth matte mint-green body",
  pink: "smooth matte soft pastel-pink body",
}[COLOR];

// Personagem TRAVADO (só muda a cor do corpo entre as versões):
const CHAR =
  `a small round chubby blob mascot character, ${BODY}, simple shiny black ` +
  "dot eyes with a tiny white highlight, soft rosy cheek blush, tiny short " +
  "stubby arms, cute kawaii soft-3D toy render, soft studio lighting, " +
  "centered, plain dark charcoal background, no text, no watermark";

const MOODS = {
  base: "with a calm friendly closed-mouth smile, looking forward",
  happy: "very happy and cheerful, big open joyful smile, eyes curved upward with joy",
  tired: "tired and sleepy, half-closed droopy eyes, slightly slumped, one small sweat drop",
  focused: "calm and focused, determined eyes, a subtle soft glow around the body",
  sleeping: "sleeping peacefully, eyes closed as gentle curves, a small 'z' floating above its head",
  water: "happily holding a tiny glass of water with both stubby arms, taking a sip",
  stretch: "doing a cheerful morning stretch, both short arms raised high above its head, stretching upward, happy face",
};

async function gen(name, mood) {
  const file = join(OUT, `${name}.png`);
  if (existsSync(file)) {
    console.log(`${COLOR}/${name}... já existe (pulando)`);
    return;
  }
  const prompt = `${CHAR}, ${mood}`;
  const url =
    `https://image.pollinations.ai/prompt/${encodeURIComponent(prompt)}` +
    `?width=768&height=768&model=flux&nologo=true&seed=${SEED}`;
  process.stdout.write(`${COLOR}/${name}... `);
  const res = await fetch(url);
  if (!res.ok) {
    console.log("✗ HTTP", res.status);
    return;
  }
  writeFileSync(file, Buffer.from(await res.arrayBuffer()));
  console.log("✓");
}

for (const [name, mood] of Object.entries(MOODS)) {
  await gen(name, mood);
}
console.log("\nprontos em:", OUT);
