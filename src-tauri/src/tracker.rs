//! Máquina de estados de atividade + loop de monitoramento.
//!
//! A cada `POLL_SECS`, lê o idle do sistema, classifica o estado e credita o
//! tempo no [`Store`]. Tempo "efetivo" = soma do tempo em `Working`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{Local, Timelike};
use serde::Serialize;
use tauri::menu::MenuItem;
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::config::Config;
use crate::idle;
use crate::locale;
use crate::reminders::SharedSchedule;
use crate::store::Store;

/// Intervalo entre leituras. Crédito por tick é fixo (robusto a sleep do sistema:
/// se a máquina dorme, a thread também dorme e não infla as horas).
const POLL_SECS: u64 = 5;

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
    let config = app.state::<Arc<Config>>().inner().clone();
    let schedule = app.state::<SharedSchedule>().inner().clone();
    let reminder_active = app.state::<Arc<AtomicBool>>().inner().clone();
    let th = Thresholds {
        idle_secs: config.idle_secs,
        away_secs: config.away_secs,
    };
    let poll = Duration::from_secs(POLL_SECS);

    loop {
        let idle_seconds = idle::seconds_since_last_input();
        let state = classify(idle_seconds, &th);

        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let hour = now.hour() as i64;

        let (working, idle_t, away) = if let Some(store) = &store {
            if let Err(e) = store.credit(&date, hour, state, POLL_SECS as i64) {
                eprintln!("[jbuddy] falha ao gravar: {e}");
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
        }

        // Atualiza a UI (tray) sempre na main thread.
        // Só o tempo no título (compacto); o ícone é o "símbolo".
        let t = locale::tray();
        let tray_title = fmt_short(working);
        let today_text = format!("{}: {}", t.today, fmt_short(working));
        let status_text = format!("{}: {}", t.status, locale::state_label(state.id()));
        let tray2 = tray.clone();
        let ti = today_item.clone();
        let si = status_item.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = tray2.set_title(Some(tray_title));
            let _ = ti.set_text(today_text);
            let _ = si.set_text(status_text);
        });

        // Lembretes: dispara no máximo um, e só se nenhum estiver aberto.
        if !reminder_active.load(Ordering::Relaxed) {
            let due = {
                let mut sched = schedule.lock().unwrap();
                sched.tick(Instant::now(), state, idle_seconds)
            };
            if let Some(payload) = due {
                reminder_active.store(true, Ordering::Relaxed);
                let app3 = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let _ = app3.emit_to("reminder", "show-reminder", payload);
                    if let Some(w) = app3.get_webview_window("reminder") {
                        crate::place_reminder(&w);
                        let _ = w.show();
                    }
                });
            }
        }

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
