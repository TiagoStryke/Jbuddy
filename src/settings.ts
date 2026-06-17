import { invoke } from "@tauri-apps/api/core";
import { applyStatic, t } from "./i18n";

interface ReminderCfg {
  enabled: boolean;
  interval_secs: number;
}
interface Config {
  water: ReminderCfg;
  stretch: ReminderCfg;
  eyes: ReminderCfg;
  idle_secs: number;
  away_secs: number;
  snooze_secs: number;
  work_start_hour: number;
  work_end_hour: number;
  target_work_hours: number;
  keep_screen_awake: boolean;
}

function el(id: string): HTMLInputElement {
  return document.getElementById(id) as HTMLInputElement;
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, isNaN(n) ? lo : n));
}

async function load() {
  const c = await invoke<Config>("get_config");
  el("water-on").checked = c.water.enabled;
  el("water-int").value = String(Math.round(c.water.interval_secs / 60));
  el("stretch-on").checked = c.stretch.enabled;
  el("stretch-int").value = String(Math.round(c.stretch.interval_secs / 60));
  el("eyes-on").checked = c.eyes.enabled;
  el("eyes-int").value = String(Math.round(c.eyes.interval_secs / 60));
  el("idle").value = String(Math.round(c.idle_secs));
  el("away").value = String(Math.round(c.away_secs));
  el("snooze").value = String(Math.round(c.snooze_secs / 60));
  el("work-start").value = String(c.work_start_hour);
  el("work-end").value = String(c.work_end_hour);
  el("target").value = String(c.target_work_hours);
  el("keep-awake").checked = c.keep_screen_awake;
}

function collect(): Config {
  const mins = (id: string) => clamp(parseInt(el(id).value, 10), 1, 240) * 60;
  return {
    water: { enabled: el("water-on").checked, interval_secs: mins("water-int") },
    stretch: {
      enabled: el("stretch-on").checked,
      interval_secs: mins("stretch-int"),
    },
    eyes: { enabled: el("eyes-on").checked, interval_secs: mins("eyes-int") },
    idle_secs: clamp(parseInt(el("idle").value, 10), 5, 3600),
    away_secs: clamp(parseInt(el("away").value, 10), 10, 7200),
    snooze_secs: clamp(parseInt(el("snooze").value, 10), 1, 120) * 60,
    work_start_hour: clamp(parseInt(el("work-start").value, 10), 0, 23),
    work_end_hour: clamp(parseInt(el("work-end").value, 10), 0, 23),
    target_work_hours: clamp(parseFloat(el("target").value), 1, 16),
    keep_screen_awake: el("keep-awake").checked,
  };
}

async function save() {
  try {
    await invoke("save_config", { newConfig: collect() });
    const btn = document.getElementById("save");
    if (btn) {
      btn.textContent = t("settings.saved");
      setTimeout(() => (btn.textContent = t("settings.save")), 1500);
    }
  } catch (e) {
    console.error("save_config falhou:", e);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  applyStatic();
  load();
  document.getElementById("save")?.addEventListener("click", save);
});
