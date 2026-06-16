//! Máquina de estados de atividade + loop de monitoramento.
//!
//! A cada `POLL_SECS`, lê o idle do sistema, classifica o estado e credita o
//! tempo no [`Store`]. Tempo "efetivo" = soma do tempo em `Working`.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::{Local, Timelike};
use serde::Serialize;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Wry};

use crate::idle;
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
    pub fn label_pt(self) -> &'static str {
        match self {
            ActivityState::Working => "trabalhando",
            ActivityState::Idle => "ocioso",
            ActivityState::Away => "ausente",
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
    today_item: MenuItem<Wry>,
    status_item: MenuItem<Wry>,
) {
    let store = match Store::open() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[jbuddy] falha ao abrir o store: {e}");
            return;
        }
    };
    let th = Thresholds::default();
    let poll = Duration::from_secs(POLL_SECS);

    loop {
        let idle_seconds = idle::seconds_since_last_input();
        let state = classify(idle_seconds, &th);

        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let hour = now.hour() as i64;

        if let Err(e) = store.credit(&date, hour, state, POLL_SECS as i64) {
            eprintln!("[jbuddy] falha ao gravar: {e}");
        }
        let (working, idle_t, away) = store.today_totals(&date).unwrap_or((0, 0, 0));

        if let Ok(mut s) = snap.lock() {
            s.state = state.label_pt().to_string();
            s.working_secs = working;
            s.idle_secs = idle_t;
            s.away_secs = away;
        }

        // Atualiza a UI (tray) sempre na main thread.
        let tray_title = fmt_short(working);
        let today_text = format!("Hoje: {} efetivas", fmt_short(working));
        let status_text = format!("Estado: {}", state.label_pt());
        let app2 = app.clone();
        let ti = today_item.clone();
        let si = status_item.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(tray) = app2.tray_by_id("main") {
                let _ = tray.set_title(Some(tray_title));
            }
            let _ = ti.set_text(today_text);
            let _ = si.set_text(status_text);
        });

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
