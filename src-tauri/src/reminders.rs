//! Motor de lembretes: tipos, mensagens rotativas e o agendador.
//!
//! O agendador decide, a cada tick do [`crate::tracker`], se algum lembrete deve
//! disparar — respeitando: ausência (não avisa tela vazia), deep focus (adia se
//! você está digitando sem parar, até um teto), e um por vez (não empilha).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::config::{Config, ReminderConfig};
use crate::tracker::ActivityState;

/// Idle (s) abaixo do qual consideramos "digitando agora" (deep focus).
const DEEP_FOCUS_IDLE: f64 = 8.0;
/// Teto de adiamento por deep focus — depois disso, dispara mesmo assim.
const MAX_DEFER: Duration = Duration::from_secs(10 * 60);
/// Reagendamento curto quando adiamos (ausente ou deep focus).
const RECHECK: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReminderKind {
    Water,
    Stretch,
    Eyes,
}

impl ReminderKind {
    pub fn id(self) -> &'static str {
        match self {
            ReminderKind::Water => "water",
            ReminderKind::Stretch => "stretch",
            ReminderKind::Eyes => "eyes",
        }
    }

    pub fn from_id(s: &str) -> Option<Self> {
        match s {
            "water" => Some(ReminderKind::Water),
            "stretch" => Some(ReminderKind::Stretch),
            "eyes" => Some(ReminderKind::Eyes),
            _ => None,
        }
    }

}

/// Payload enviado pra janela de lembrete. Só a CHAVE e o índice de rotação —
/// emoji/título/mensagem (e tradução) ficam no `src/i18n.ts` do frontend.
#[derive(Clone, Debug, Serialize)]
pub struct ReminderPayload {
    pub kind: String,
    pub rotation: usize,
}

struct ReminderState {
    kind: ReminderKind,
    enabled: bool,
    interval: Duration,
    next_due: Instant,
    rotation: usize,
    deferred_since: Option<Instant>,
}

pub struct Schedule {
    reminders: Vec<ReminderState>,
    snooze: Duration,
}

pub type SharedSchedule = Arc<Mutex<Schedule>>;

impl Schedule {
    pub fn from_config(cfg: &Config, now: Instant) -> Self {
        let mk = |kind: ReminderKind, rc: &ReminderConfig| {
            let interval = Duration::from_secs(rc.interval_secs.max(1));
            ReminderState {
                kind,
                enabled: rc.enabled,
                interval,
                next_due: now + interval,
                rotation: 0,
                deferred_since: None,
            }
        };
        Schedule {
            reminders: vec![
                mk(ReminderKind::Water, &cfg.water),
                mk(ReminderKind::Stretch, &cfg.stretch),
                mk(ReminderKind::Eyes, &cfg.eyes),
            ],
            snooze: Duration::from_secs(cfg.snooze_secs.max(1)),
        }
    }

    fn find_mut(&mut self, kind: ReminderKind) -> Option<&mut ReminderState> {
        self.reminders.iter_mut().find(|r| r.kind == kind)
    }

    /// Soneca: dispara de novo daqui a `snooze`.
    pub fn snooze(&mut self, kind: ReminderKind, now: Instant) {
        let snooze = self.snooze;
        if let Some(r) = self.find_mut(kind) {
            r.next_due = now + snooze;
            r.deferred_since = None;
        }
    }

    /// "Feito" ou "pular": reagenda pro próximo intervalo cheio.
    pub fn reschedule(&mut self, kind: ReminderKind, now: Instant) {
        if let Some(r) = self.find_mut(kind) {
            r.next_due = now + r.interval;
            r.deferred_since = None;
        }
    }

    /// Avalia se algum lembrete deve disparar agora. No máximo um por chamada.
    pub fn tick(
        &mut self,
        now: Instant,
        state: ActivityState,
        idle_seconds: f64,
    ) -> Option<ReminderPayload> {
        for r in self.reminders.iter_mut() {
            if !r.enabled || now < r.next_due {
                continue;
            }

            // Ausente: não avisa tela vazia. Reavalia em breve.
            if state == ActivityState::Away {
                r.next_due = now + RECHECK;
                continue;
            }

            // Deep focus: digitando sem parar → adia, até o teto.
            if idle_seconds < DEEP_FOCUS_IDLE {
                let since = *r.deferred_since.get_or_insert(now);
                if now.duration_since(since) < MAX_DEFER {
                    r.next_due = now + RECHECK;
                    continue;
                }
                // passou do teto de adiamento: dispara mesmo assim.
            }

            // Dispara.
            let payload = ReminderPayload {
                kind: r.kind.id().to_string(),
                rotation: r.rotation,
            };
            r.rotation = r.rotation.wrapping_add(1);
            r.next_due = now + r.interval;
            r.deferred_since = None;
            return Some(payload);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule_now() -> (Schedule, Instant) {
        let now = Instant::now();
        (Schedule::from_config(&Config::default(), now), now)
    }

    #[test]
    fn nao_dispara_antes_do_intervalo() {
        let (mut s, now) = schedule_now();
        assert!(s
            .tick(now + Duration::from_secs(10), ActivityState::Working, 20.0)
            .is_none());
    }

    #[test]
    fn dispara_quando_devido_e_trabalhando() {
        let (mut s, now) = schedule_now();
        // eyes vence em 20min; aos 21min está devido
        let due = now + Duration::from_secs(21 * 60);
        assert!(s.tick(due, ActivityState::Working, 20.0).is_some());
    }

    #[test]
    fn nao_dispara_quando_ausente() {
        let (mut s, now) = schedule_now();
        let due = now + Duration::from_secs(21 * 60);
        assert!(s.tick(due, ActivityState::Away, 5000.0).is_none());
    }

    #[test]
    fn adia_durante_deep_focus() {
        let (mut s, now) = schedule_now();
        let due = now + Duration::from_secs(21 * 60);
        // idle baixíssimo = digitando agora → adia (primeira vez)
        assert!(s.tick(due, ActivityState::Working, 2.0).is_none());
    }

    #[test]
    fn dispara_apos_teto_de_adiamento() {
        let (mut s, now) = schedule_now();
        let due = now + Duration::from_secs(21 * 60);
        // primeira avaliação em deep focus marca deferred_since
        assert!(s.tick(due, ActivityState::Working, 2.0).is_none());
        // muito depois do teto, ainda em deep focus → dispara mesmo assim
        let later = due + MAX_DEFER + Duration::from_secs(1);
        assert!(s.tick(later, ActivityState::Working, 2.0).is_some());
    }
}
