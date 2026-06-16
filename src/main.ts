import { invoke } from "@tauri-apps/api/core";
import { applyStatic, stateLabel } from "./i18n";

interface Snapshot {
  state: string;
  working_secs: number;
  idle_secs: number;
  away_secs: number;
}

// Meta diária de foco (provisória; vira configurável depois).
const GOAL_SECS = 6 * 3600;
const RING_R = 86;
const RING_C = 2 * Math.PI * RING_R;

function fmt(secs: number): string {
  const min = Math.floor(secs / 60);
  const h = Math.floor(min / 60);
  const m = min % 60;
  return `${h}h${String(m).padStart(2, "0")}`;
}

function setText(id: string, value: string) {
  const el = document.getElementById(id);
  if (el) el.textContent = value;
}

function setRing(workingSecs: number) {
  const ring = document.getElementById("ring") as SVGCircleElement | null;
  if (!ring) return;
  const pct = Math.max(0, Math.min(1, workingSecs / GOAL_SECS));
  ring.style.strokeDasharray = `${RING_C}`;
  ring.style.strokeDashoffset = `${RING_C * (1 - pct)}`;
}

async function refresh() {
  try {
    const s = await invoke<Snapshot>("get_today_stats");
    setText("working", fmt(s.working_secs));
    setText("idle", fmt(s.idle_secs));
    setText("away", fmt(s.away_secs));
    setText("state", stateLabel(s.state));

    const pill = document.getElementById("state-pill");
    if (pill) pill.dataset.state = s.state || "";

    const tracked = s.working_secs + s.idle_secs + s.away_secs;
    setText(
      "focus",
      tracked > 0 ? `${Math.round((s.working_secs / tracked) * 100)}%` : "—",
    );

    setRing(s.working_secs);
  } catch (e) {
    console.error("get_today_stats falhou:", e);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  applyStatic();
  refresh();
  setInterval(refresh, 5000);
});
