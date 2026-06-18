import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyStatic, lang, stateLabel, t } from "./i18n";
import { moodForState } from "./mascot";

interface Snapshot {
  state: string;
  working_secs: number;
  idle_secs: number;
  away_secs: number;
  typing: boolean;
}
interface HourStat {
  hour: number;
  working_secs: number;
}
interface DayStat {
  date: string;
  working_secs: number;
}

const GOAL_SECS = 6 * 3600;
const RING_R = 86;
const RING_C = 2 * Math.PI * RING_R;
const LOCALE = lang === "pt" ? "pt-BR" : "en-US";
let mascotColor = "green";

function fmt(secs: number): string {
  const min = Math.floor(secs / 60);
  return `${Math.floor(min / 60)}h${String(min % 60).padStart(2, "0")}`;
}

function setText(id: string, value: string) {
  const el = document.getElementById(id);
  if (el) el.textContent = value;
}

function dateKey(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function weekdayShort(dateKey: string): string {
  return new Date(`${dateKey}T00:00:00`).toLocaleDateString(LOCALE, {
    weekday: "short",
  });
}

async function refreshStats() {
  try {
    const s = await invoke<Snapshot>("get_today_stats");
    setText("working", fmt(s.working_secs));
    setText("idle", fmt(s.idle_secs));
    setText("away", fmt(s.away_secs));
    setText("state", stateLabel(s.state));

    const pill = document.getElementById("state-pill");
    if (pill) pill.dataset.state = s.state || "";

    const mascot = document.getElementById("mascot") as HTMLImageElement | null;
    if (mascot) {
      const next = moodForState(s.state, mascotColor, s.typing);
      if (mascot.src !== next) mascot.src = next;
    }

    const tracked = s.working_secs + s.idle_secs + s.away_secs;
    setText(
      "focus",
      tracked > 0 ? `${Math.round((s.working_secs / tracked) * 100)}%` : "—",
    );

    const ring = document.getElementById("ring") as SVGCircleElement | null;
    if (ring) {
      const pct = Math.max(0, Math.min(1, s.working_secs / GOAL_SECS));
      ring.style.strokeDasharray = `${RING_C}`;
      ring.style.strokeDashoffset = `${RING_C * (1 - pct)}`;
    }
  } catch (e) {
    console.error("get_today_stats falhou:", e);
  }
}

async function renderWeek() {
  const days = await invoke<DayStat[]>("get_week");
  const map = new Map(days.map((d) => [d.date, d.working_secs]));
  const today = new Date();
  const items = [];
  for (let i = 6; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    const key = dateKey(d);
    items.push({ key, secs: map.get(key) ?? 0, isToday: i === 0 });
  }
  const max = Math.max(1, ...items.map((x) => x.secs));
  const host = document.getElementById("week");
  if (!host) return;
  host.innerHTML = "";
  for (const it of items) {
    const col = document.createElement("div");
    col.className = `day${it.secs === 0 ? " empty" : ""}${it.isToday ? " today" : ""}`;
    const bar = document.createElement("div");
    bar.className = "day-bar";
    bar.style.height = `${it.secs === 0 ? 3 : Math.max(6, (it.secs / max) * 100)}%`;
    bar.title = fmt(it.secs);
    const label = document.createElement("span");
    label.className = "day-label";
    label.textContent = weekdayShort(it.key);
    col.append(bar, label);
    host.append(col);
  }
}

async function renderHours() {
  const stats = await invoke<HourStat[]>("get_today_hourly");
  const byHour = new Map(stats.map((s) => [s.hour, s.working_secs]));
  const vals = Array.from({ length: 24 }, (_, h) => byHour.get(h) ?? 0);
  const max = Math.max(1, ...vals);
  let peakHour = -1;
  let peakVal = 0;
  vals.forEach((v, h) => {
    if (v > peakVal) {
      peakVal = v;
      peakHour = h;
    }
  });

  const host = document.getElementById("hours");
  if (host) {
    host.innerHTML = "";
    vals.forEach((v, h) => {
      const bar = document.createElement("div");
      bar.className = `hour-bar${v > 0 ? " has" : ""}${h === peakHour && peakVal > 0 ? " peak" : ""}`;
      bar.style.height = `${v > 0 ? Math.max(8, (v / max) * 100) : 2}%`;
      bar.title = `${h}h · ${fmt(v)}`;
      host.append(bar);
    });
  }
  setText("peak", peakVal > 0 ? `${t("dash.peak")} ${peakHour}h` : "");
}

async function refreshReports() {
  try {
    await Promise.all([renderWeek(), renderHours()]);
  } catch (e) {
    console.error("relatórios falharam:", e);
  }
}

async function applyConfig() {
  try {
    const cfg = await invoke<{ mascot_color: string; theme: string }>("get_config");
    mascotColor = cfg.mascot_color || "green";
    document.documentElement.dataset.theme = cfg.theme || "default";
  } catch (e) {
    console.error("get_config falhou:", e);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  applyStatic();
  await applyConfig();
  refreshStats();
  refreshReports();
  setInterval(refreshStats, 5000);
  setInterval(refreshReports, 60000);
  // reaplica tema/cor quando as configurações mudam (sem reabrir).
  listen("config-changed", async () => {
    await applyConfig();
    refreshStats();
  });
});
