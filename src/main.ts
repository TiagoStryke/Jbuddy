import { invoke } from "@tauri-apps/api/core";

interface Snapshot {
  state: string;
  working_secs: number;
  idle_secs: number;
  away_secs: number;
}

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

async function refresh() {
  try {
    const s = await invoke<Snapshot>("get_today_stats");
    setText("working", fmt(s.working_secs));
    setText("idle", fmt(s.idle_secs));
    setText("away", fmt(s.away_secs));
    setText("state", s.state || "—");
  } catch (e) {
    console.error("get_today_stats falhou:", e);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  refresh();
  setInterval(refresh, 5000);
});
