import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyStatic, getReminder } from "./i18n";
import { moodForReminder } from "./mascot";

interface ReminderPayload {
  kind: string;
  rotation: number;
}

let currentKind = "";
let mascotColor = "green";

function setText(id: string, value: string) {
  const el = document.getElementById(id);
  if (el) el.textContent = value;
}

// O backend manda a CHAVE + rotação; o i18n vira emoji/título/mensagem.
// Tematizamos pelo tipo e re-disparamos a animação a cada vez que aparece.
listen<ReminderPayload>("show-reminder", (event) => {
  const { kind, rotation } = event.payload;
  currentKind = kind;
  document.body.dataset.kind = kind;

  const c = getReminder(kind, rotation);
  setText("title", c.title);
  setText("message", c.message);
  const mascot = document.getElementById("mascot") as HTMLImageElement | null;
  if (mascot) mascot.src = moodForReminder(kind, mascotColor);

  const card = document.querySelector(".card");
  if (card) {
    card.classList.remove("enter");
    void (card as HTMLElement).offsetWidth; // força reflow pra reiniciar a animação
    card.classList.add("enter");
  }
});

function act(action: string) {
  invoke("reminder_action", { kind: currentKind, action }).catch((e) =>
    console.error("reminder_action falhou:", e),
  );
}

window.addEventListener("DOMContentLoaded", () => {
  applyStatic();
  const applyCfg = () =>
    invoke<{ mascot_color: string; theme: string }>("get_config")
      .then((c) => {
        mascotColor = c.mascot_color || "green";
        document.documentElement.dataset.theme = c.theme || "default";
      })
      .catch(() => {});
  applyCfg();
  listen("config-changed", applyCfg);
  document.getElementById("done")?.addEventListener("click", () => act("done"));
  document
    .getElementById("snooze")
    ?.addEventListener("click", () => act("snooze"));
  document.getElementById("skip")?.addEventListener("click", () => act("skip"));
});
