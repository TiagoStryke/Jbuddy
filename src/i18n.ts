// i18n do webview. Fonte única de toda a cópia visível (dashboard + lembrete).
// Default inglês; PT quando o locale do sistema começa com "pt".
// O backend manda CHAVES estáveis (state id, reminder kind+rotation); aqui vira texto.

type Lang = "en" | "pt";

export const lang: Lang = navigator.language?.toLowerCase().startsWith("pt")
  ? "pt"
  : "en";

const ui: Record<Lang, Record<string, string>> = {
  en: {
    "focus.today": "focus today",
    "stat.idle": "idle",
    "stat.away": "away",
    "stat.focus": "focus",
    "state.working": "working",
    "state.idle": "idle",
    "state.away": "away",
    "action.done": "Done",
    "action.snooze": "Snooze",
    "action.skip": "Skip",
    "tagline": "your healthy work companion",
    "settings.title": "Settings",
    "settings.reminders": "Reminders",
    "settings.water": "Water",
    "settings.stretch": "Stand & stretch",
    "settings.eyes": "Eyes (20-20-20)",
    "settings.every": "every (min)",
    "settings.activity": "Activity",
    "settings.idle": "idle after (s)",
    "settings.away": "away after (s)",
    "settings.snooze": "snooze (min)",
    "settings.work": "Work hours",
    "settings.start": "start",
    "settings.end": "end",
    "settings.screen": "Screen",
    "settings.keepAwake": "Keep the screen from turning off",
    "settings.save": "Save",
    "settings.saved": "Saved ✓",
  },
  pt: {
    "focus.today": "foco hoje",
    "stat.idle": "ocioso",
    "stat.away": "ausente",
    "stat.focus": "foco",
    "state.working": "trabalhando",
    "state.idle": "ocioso",
    "state.away": "ausente",
    "action.done": "Feito",
    "action.snooze": "Soneca",
    "action.skip": "Pular",
    "tagline": "teu companheiro de trabalho saudável",
    "settings.title": "Configurações",
    "settings.reminders": "Lembretes",
    "settings.water": "Água",
    "settings.stretch": "Levantar e alongar",
    "settings.eyes": "Olhos (20-20-20)",
    "settings.every": "a cada (min)",
    "settings.activity": "Atividade",
    "settings.idle": "ocioso após (s)",
    "settings.away": "ausente após (s)",
    "settings.snooze": "soneca (min)",
    "settings.work": "Jornada",
    "settings.start": "início",
    "settings.end": "fim",
    "settings.screen": "Tela",
    "settings.keepAwake": "Impede o notebook de desligar a tela",
    "settings.save": "Salvar",
    "settings.saved": "Salvo ✓",
  },
};

interface ReminderCopy {
  emoji: string;
  title: string;
  messages: string[];
}

const reminders: Record<Lang, Record<string, ReminderCopy>> = {
  en: {
    water: {
      emoji: "💧",
      title: "Time for water",
      messages: [
        "Take a few sips of water. Your brain will thank you. 💧",
        "Quick hydration break — grab a glass of water?",
        "Small sips, big difference. Time for water.",
      ],
    },
    stretch: {
      emoji: "🧍",
      title: "Stand up & stretch",
      messages: [
        "Stand up, reach for the sky and roll your shoulders.",
        "On your feet! A quick lap around the room loosens you up.",
        "Slowly stretch your neck to each side. No rush.",
      ],
    },
    eyes: {
      emoji: "👀",
      title: "Rest your eyes",
      messages: [
        "20-20-20: look at something ~6 m away for 20 seconds.",
        "Look away from the screen and focus on something far.",
        "Blink a few times and relax your gaze on the horizon.",
      ],
    },
  },
  pt: {
    water: {
      emoji: "💧",
      title: "Hora de beber água",
      messages: [
        "Toma uns goles d'água. Teu cérebro agradece. 💧",
        "Hidratação rápida — vai um copo d'água?",
        "Pausa pra água. Pequenos goles, grande diferença.",
      ],
    },
    stretch: {
      emoji: "🧍",
      title: "Levanta e alonga",
      messages: [
        "Levanta, estica os braços pro alto e gira os ombros.",
        "De pé! Uma volta rápida pela sala destrava o corpo.",
        "Alonga o pescoço devagar pra cada lado. Sem pressa.",
      ],
    },
    eyes: {
      emoji: "👀",
      title: "Descansa os olhos",
      messages: [
        "20-20-20: olha por 20s algo a uns 6 metros de distância.",
        "Tira os olhos da tela e foca em algo longe por uns segundos.",
        "Pisca algumas vezes e relaxa o olhar no horizonte.",
      ],
    },
  },
};

export function t(key: string): string {
  return ui[lang][key] ?? ui.en[key] ?? key;
}

/** Rótulo de um estado de atividade a partir da chave (working/idle/away). */
export function stateLabel(stateId: string): string {
  return stateId ? t(`state.${stateId}`) : "—";
}

/** Conteúdo de um lembrete a partir da chave e do índice de rotação. */
export function getReminder(
  kind: string,
  rotation: number,
): { emoji: string; title: string; message: string } {
  const r = reminders[lang][kind] ?? reminders.en[kind];
  if (!r) return { emoji: "🔔", title: "", message: "" };
  return {
    emoji: r.emoji,
    title: r.title,
    message: r.messages[rotation % r.messages.length],
  };
}

/** Preenche todos os elementos com [data-i18n] usando o dicionário. */
export function applyStatic(root: ParentNode = document): void {
  root.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n;
    if (key) el.textContent = t(key);
  });
}
