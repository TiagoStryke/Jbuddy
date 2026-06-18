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

// Renderiza um lembrete: chave+rotação → emoji/título/mensagem (i18n) + mascote.
function render(kind: string, rotation: number) {
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
    void (card as HTMLElement).offsetWidth; // reflow pra reiniciar a animação
    card.classList.add("enter");
  }
}

// Push: o backend avisa quando um lembrete dispara.
listen<ReminderPayload>("show-reminder", (event) =>
  render(event.payload.kind, event.payload.rotation),
);

function act(action: string) {
  invoke("reminder_action", { kind: currentKind, action }).catch((e) =>
    console.error("reminder_action falhou:", e),
  );
}

window.addEventListener("DOMContentLoaded", async () => {
  applyStatic();
  const applyCfg = () =>
    invoke<{ mascot_color: string; theme: string }>("get_config")
      .then((c) => {
        mascotColor = c.mascot_color || "green";
        document.documentElement.dataset.theme = c.theme || "default";
      })
      .catch(() => {});
  await applyCfg();
  listen("config-changed", applyCfg);

  // Pull: ao carregar, se já há um lembrete pendente (evento perdido no 1º show),
  // renderiza ele mesmo assim.
  try {
    const p = await invoke<ReminderPayload | null>("get_pending_reminder");
    if (p) render(p.kind, p.rotation);
  } catch {
    /* sem pendente */
  }

  document.getElementById("done")?.addEventListener("click", () => act("done"));
  document
    .getElementById("snooze")
    ?.addEventListener("click", () => act("snooze"));
  document.getElementById("skip")?.addEventListener("click", () => act("skip"));
});
