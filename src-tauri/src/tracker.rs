//! Máquina de estados de atividade + loop de monitoramento.
//!
//! A cada `POLL_SECS`, lê o idle do sistema, classifica o estado e credita o
//! tempo no [`Store`]. Tempo "efetivo" = soma do tempo em `Working`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, Timelike};
use serde::Serialize;
use tauri::menu::MenuItem;
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::config::Config;
use crate::idle;
use crate::keepalive;
use crate::locale;
use crate::meeting;
use crate::reminders::SharedSchedule;
use crate::store::Store;

/// Intervalo entre leituras. Crédito por tick é fixo (robusto a sleep do sistema:
/// se a máquina dorme, a thread também dorme e não infla as horas).
const POLL_SECS: u64 = 5;

/// Intervalo mínimo entre nudges de keep-alive (Teams marca ausente ~5min).
const KEEP_ALIVE_SECS: u64 = 150;

/// Tempo contínuo trabalhando sem pausa que dispara o aviso anti-excesso.
const NO_BREAK_SECS: u64 = 90 * 60;

/// Última tecla há menos que isso (e trabalhando) ⇒ "digitando".
const TYPING_SECS: f64 = 4.0;

/// Dia útil e dentro do horário de trabalho configurado.
fn is_work_hours(start: u32, end: u32) -> bool {
    let now = Local::now();
    if now.weekday().num_days_from_monday() >= 5 {
        return false; // fim de semana
    }
    let h = now.hour();
    if start <= end {
        h >= start && h < end
    } else {
        h >= start || h < end
    }
}

/// Dia útil e já passou do fim do expediente.
fn is_after_work_end(end: u32) -> bool {
    let now = Local::now();
    now.weekday().num_days_from_monday() < 5 && now.hour() >= end
}

/// Mostra um lembrete (qualquer tipo) na janela do canto.
fn show_reminder(app: &AppHandle, kind: &str, rotation: usize) {
    let payload = serde_json::json!({ "kind": kind, "rotation": rotation });
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = app2.emit_to("reminder", "show-reminder", payload);
        if let Some(w) = app2.get_webview_window("reminder") {
            crate::place_reminder(&w);
            // mostra SEM roubar o foco; o clique funciona via acceptFirstMouse.
            let _ = w.show();
        }
    });
}

/// Estado de atividade derivado do tempo de idle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActivityState {
    /// Atividade recente — conta como hora efetiva.
    Working,
    /// Pausa curta (parou, mas ainda está por perto). Não conta.
    Idle,
    /// Longe do teclado (almoço, reunião fora). Não conta; "fecha" o bloco.
    Away,
}

impl ActivityState {
    /// Chave estável (não traduzida) usada no snapshot e no CSS `data-state`.
    pub fn id(self) -> &'static str {
        match self {
            ActivityState::Working => "working",
            ActivityState::Idle => "idle",
            ActivityState::Away => "away",
        }
    }
}

/// Limites (em segundos) que separam os estados. Configurável depois (Fase 2).
#[derive(Clone, Copy, Debug)]
pub struct Thresholds {
    pub idle_secs: f64,
    pub away_secs: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            idle_secs: 90.0,
            away_secs: 300.0,
        }
    }
}

/// Classifica o estado a partir dos segundos de idle.
pub fn classify(idle_seconds: f64, t: &Thresholds) -> ActivityState {
    if idle_seconds < t.idle_secs {
        ActivityState::Working
    } else if idle_seconds < t.away_secs {
        ActivityState::Idle
    } else {
        ActivityState::Away
    }
}

/// Snapshot do dia, exposto ao frontend via comando Tauri.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Snapshot {
    pub state: String,
    pub working_secs: i64,
    pub idle_secs: i64,
    pub away_secs: i64,
    pub typing: bool,
}

pub type SharedSnapshot = Arc<Mutex<Snapshot>>;

/// Loop de monitoramento. Roda numa thread dedicada por toda a vida do app.
pub fn run_loop(
    app: AppHandle,
    snap: SharedSnapshot,
    tray: TrayIcon<Wry>,
    today_item: MenuItem<Wry>,
    status_item: MenuItem<Wry>,
) {
    // Se o store falhar, NÃO retornamos: isso descartaria o `tray`/menus movidos
    // pra cá e o ícone sumiria do menu bar. Seguimos sem persistência (UI viva).
    let store = match Store::open() {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("[jbuddy] store indisponível, seguindo sem persistência: {e}");
            None
        }
    };
    // Estado compartilhado (gerenciado pelo Tauri).
    let config = app.state::<Arc<Mutex<Config>>>().inner().clone();
    let schedule = app.state::<SharedSchedule>().inner().clone();
    let reminder_active = app.state::<Arc<AtomicBool>>().inner().clone();
    let poll = Duration::from_secs(POLL_SECS);
    let keep_alive_gap = Duration::from_secs(KEEP_ALIVE_SECS);
    let no_break = Duration::from_secs(NO_BREAK_SECS);

    // Atividade REAL (exclui os nudges do próprio keep-alive).
    let mut last_real_activity = Instant::now();
    let mut last_nudge: Option<Instant> = None;

    // Guarda anti-excesso + ritual de fim de expediente.
    let mut continuous_work = Duration::ZERO;
    let mut nobreak_warned = false;
    let mut overtime_fired = false;
    let mut endday_fired = false;
    let mut flag_date = String::new();
    // "endday" só dispara se vimos o expediente ANTES do fim nesta sessão (evita
    // disparar no boot tardio). "prev_working" detecta o cruzamento da meta.
    let mut seen_before_end = false;
    let mut prev_working: Option<i64> = None;
    // Cota de almoço já consumida hoje (ausência dentro da janela).
    let mut lunch_used = Duration::ZERO;
    // Último ícone do tray (cor, humor) — só troca quando muda.
    let mut last_icon: Option<(String, String)> = None;

    loop {
        // Snapshot da config deste tick (a tela de configs altera ao vivo).
        let (th, keep_awake, work_start, work_end, lunch_start, lunch_end, lunch_minutes, target_secs, mascot_color) =
            match config.lock() {
                Ok(c) => (
                    Thresholds {
                        idle_secs: c.idle_secs,
                        away_secs: c.away_secs,
                    },
                    c.keep_screen_awake,
                    c.work_start_hour,
                    c.work_end_hour,
                    c.lunch_start_hour,
                    c.lunch_end_hour,
                    c.lunch_minutes,
                    (c.target_work_hours * 3600.0) as i64,
                    c.mascot_color.clone(),
                ),
                Err(_) => (
                    Thresholds::default(),
                    false,
                    9,
                    24,
                    12,
                    12,
                    60,
                    8 * 3600,
                    "green".to_string(),
                ),
            };

        let now_inst = Instant::now();
        let hid_idle = idle::seconds_since_last_input();
        let last_event = now_inst
            .checked_sub(Duration::from_secs_f64(hid_idle.max(0.0)))
            .unwrap_or(now_inst);
        // Se o último evento foi um nudge NOSSO, não conta como atividade real.
        let from_own_nudge = last_nudge.is_some_and(|n| {
            let diff = if last_event >= n {
                last_event - n
            } else {
                n - last_event
            };
            diff < Duration::from_secs(3)
        });
        if !from_own_nudge {
            last_real_activity = last_event;
        }
        let real_idle = now_inst.duration_since(last_real_activity).as_secs_f64();

        let screen_off = idle::display_asleep();
        // Tela dormindo ⇒ ausente. Senão, classifica pelo idle REAL.
        let state = if screen_off {
            ActivityState::Away
        } else {
            classify(real_idle, &th)
        };
        // Digitando = trabalhando + tecla há pouquíssimo tempo.
        let typing = state == ActivityState::Working
            && idle::seconds_since_last_key() < TYPING_SECS;

        // Keep-alive opt-in: só em horário de trabalho, tela acesa, e quando você
        // está fora (idle real alto). No máximo um nudge a cada KEEP_ALIVE_SECS.
        if keep_awake
            && !screen_off
            && real_idle > th.idle_secs
            && is_work_hours(work_start, work_end)
            && last_nudge.is_none_or(|n| now_inst.duration_since(n) >= keep_alive_gap)
        {
            keepalive::nudge();
            last_nudge = Some(now_inst);
        }

        // Trabalho contínuo (pra guarda anti-excesso). Qualquer pausa zera.
        if state == ActivityState::Working {
            continuous_work += poll;
        } else {
            continuous_work = Duration::ZERO;
            nobreak_warned = false;
        }

        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let hour = now.hour() as i64;

        // Reset diário dos avisos "uma vez por dia".
        if date != flag_date {
            flag_date = date.clone();
            overtime_fired = false;
            endday_fired = false;
            seen_before_end = false;
            lunch_used = Duration::ZERO;
        }
        // Marca que estávamos no expediente antes do fim (pro ritual de fim de dia).
        if now.weekday().num_days_from_monday() < 5 && (hour as u32) < work_end {
            seen_before_end = true;
        }

        let within_work_hours = (hour as u32) >= work_start && (hour as u32) < work_end;
        let is_break = state != ActivityState::Working;

        // Almoço: enquanto AUSENTE dentro da janela, consome a cota (é o almoço).
        let consuming_lunch = lunch_start != lunch_end
            && (hour as u32) >= lunch_start
            && (hour as u32) < lunch_end
            && state == ActivityState::Away
            && lunch_used < Duration::from_secs(lunch_minutes * 60);
        if consuming_lunch {
            lunch_used += poll;
        }

        // NÃO credita: almoço, OU pausa (ocioso/ausente) FORA do expediente — a noite
        // dormindo / fora do horário não vira "ausência". Foco (trabalho) conta sempre.
        let skip_credit = consuming_lunch || (is_break && !within_work_hours);

        let (working, idle_t, away) = if let Some(store) = &store {
            if !skip_credit {
                if let Err(e) = store.credit(&date, hour, state, POLL_SECS as i64) {
                    eprintln!("[jbuddy] falha ao gravar: {e}");
                }
            }
            store.today_totals(&date).unwrap_or((0, 0, 0))
        } else {
            (0, 0, 0)
        };

        if let Ok(mut s) = snap.lock() {
            s.state = state.id().to_string();
            s.working_secs = working;
            s.idle_secs = idle_t;
            s.away_secs = away;
            s.typing = typing;
        }

        // Atualiza a UI (tray) sempre na main thread.
        // Só o tempo no título (compacto); o ícone é o "símbolo".
        let t = locale::tray();
        let tray_title = fmt_short(working);
        let today_text = format!("{}: {}", t.today, fmt_short(working));
        let status_text = format!("{}: {}", t.status, locale::state_label(state.id()));
        // Ícone do tray = mascote no humor atual; só troca quando muda.
        let mood = if typing {
            "typing"
        } else {
            crate::tray_mood(state.id())
        };
        let icon = if last_icon.as_ref().map(|(c, m)| (c.as_str(), m.as_str()))
            != Some((mascot_color.as_str(), mood))
        {
            last_icon = Some((mascot_color.clone(), mood.to_string()));
            Some(crate::mood_icon(&mascot_color, mood))
        } else {
            None
        };

        let tray2 = tray.clone();
        let ti = today_item.clone();
        let si = status_item.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = tray2.set_title(Some(tray_title));
            let _ = ti.set_text(today_text);
            let _ = si.set_text(status_text);
            if let Some(ic) = icon {
                let _ = tray2.set_icon(Some(ic));
            }
        });

        // Lembretes (intervalo + anti-excesso + fim de expediente): no máximo
        // um por vez, e só se nenhum estiver aberto.
        if !reminder_active.load(Ordering::Relaxed) {
            let in_meeting = meeting::microphone_in_use();
            // lock graceful: um panic aqui mataria a thread e sumiria com o tray.
            let due = match schedule.lock() {
                Ok(mut sched) => sched.tick(state, real_idle, in_meeting, poll),
                Err(_) => None,
            };
            let fire: Option<(String, usize)> = if let Some(p) = due {
                Some((p.kind, p.rotation))
            } else if !nobreak_warned
                && continuous_work >= no_break
                && state == ActivityState::Working
                && !in_meeting
            {
                nobreak_warned = true;
                Some(("nobreak".to_string(), 0))
            } else if !overtime_fired
                && prev_working.is_some_and(|p| p < target_secs)
                && working >= target_secs
                && state == ActivityState::Working
            {
                overtime_fired = true;
                Some(("overtime".to_string(), 0))
            } else if !endday_fired
                && seen_before_end
                && is_after_work_end(work_end)
                && state != ActivityState::Away
            {
                endday_fired = true;
                Some(("endday".to_string(), 0))
            } else {
                None
            };
            if let Some((kind, rotation)) = fire {
                reminder_active.store(true, Ordering::Relaxed);
                show_reminder(&app, &kind, rotation);
            }
        }
        prev_working = Some(working);

        thread::sleep(poll);
    }
}

/// Formata segundos como `3h05` (curto, pro título do tray).
fn fmt_short(secs: i64) -> String {
    let total_min = secs / 60;
    format!("{}h{:02}", total_min / 60, total_min % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifica_trabalhando_abaixo_do_limite_de_ocioso() {
        let t = Thresholds::default();
        assert_eq!(classify(0.0, &t), ActivityState::Working);
        assert_eq!(classify(89.9, &t), ActivityState::Working);
    }

    #[test]
    fn classifica_ocioso_entre_os_limites() {
        let t = Thresholds::default();
        // exatamente no limite de ocioso já conta como ocioso (fronteira inclusiva)
        assert_eq!(classify(90.0, &t), ActivityState::Idle);
        assert_eq!(classify(299.9, &t), ActivityState::Idle);
    }

    #[test]
    fn classifica_ausente_acima_do_limite() {
        let t = Thresholds::default();
        assert_eq!(classify(300.0, &t), ActivityState::Away);
        assert_eq!(classify(10_000.0, &t), ActivityState::Away);
    }

    #[test]
    fn fmt_short_formata_horas_e_minutos() {
        assert_eq!(fmt_short(0), "0h00");
        assert_eq!(fmt_short(60), "0h01");
        assert_eq!(fmt_short(3 * 3600 + 5 * 60), "3h05");
        assert_eq!(fmt_short(3600), "1h00");
    }
}
