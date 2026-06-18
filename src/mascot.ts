// Mapeia estado/lembrete → imagem do mascote, por COR (green | pink).
// PNGs transparentes em assets/mascot/<cor>/ (geradas pelos scripts em tools/).

const imgs = import.meta.glob("./assets/mascot/*/*.png", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;

function url(color: string, name: string): string {
  return (
    imgs[`./assets/mascot/${color}/${name}.png`] ??
    imgs[`./assets/mascot/${color}/base.png`] ??
    imgs["./assets/mascot/green/base.png"]
  );
}

/** Humor do mascote no dashboard a partir do estado de atividade. */
export function moodForState(state: string, color = "green"): string {
  if (state === "working") return url(color, "happy");
  if (state === "away") return url(color, "sleeping");
  return url(color, "base"); // ocioso / desconhecido
}

const REMINDER_MOOD: Record<string, string> = {
  water: "water",
  stretch: "stretch",
  eyes: "focused",
  nobreak: "tired",
  overtime: "tired",
  endday: "sleeping",
};

/** Humor do mascote na janela de lembrete a partir do tipo. */
export function moodForReminder(kind: string, color = "green"): string {
  return url(color, REMINDER_MOOD[kind] ?? "base");
}
