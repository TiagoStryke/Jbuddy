//! Motor de lembretes: tipos, mensagens rotativas e o agendador.
//!
//! Os intervalos contam APENAS tempo de trabalho ATIVO — ocioso/ausente/almoço
//! pausam o contador (você não volta do almoço e leva um lembrete na cara). Também
//! adia em deep focus (digitando sem parar) até um teto, e dispara um por vez.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

use crate::config::{Config, ReminderConfig};
use crate::tracker::ActivityState;

/// Idle (s) abaixo do qual consideramos "digitando agora" (deep focus).
const DEEP_FOCUS_IDLE: f64 = 8.0;
/// Teto de adiamento por deep focus — depois disso, dispara mesmo assim.
const MAX_DEFER: Duration = Duration::from_secs(10 * 60);

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
    /// Tempo de trabalho ATIVO acumulado desde o último disparo.
    elapsed: Duration,
    rotation: usize,
}

pub struct Schedule {
    reminders: Vec<ReminderState>,
    snooze: Duration,
}

pub type SharedSchedule = Arc<Mutex<Schedule>>;

impl Schedule {
    pub fn from_config(cfg: &Config) -> Self {
        let mk = |kind: ReminderKind, rc: &ReminderConfig| ReminderState {
            kind,
            enabled: rc.enabled,
            interval: Duration::from_secs(rc.interval_secs.max(1)),
            elapsed: Duration::ZERO,
            rotation: 0,
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

    /// Soneca: dispara de novo após `snooze` de trabalho ATIVO.
    pub fn snooze(&mut self, kind: ReminderKind) {
        let snooze = self.snooze;
        if let Some(r) = self.find_mut(kind) {
            r.elapsed = r.interval.saturating_sub(snooze);
        }
    }

    /// "Feito" ou "pular": zera o contador (intervalo cheio de novo).
    pub fn reschedule(&mut self, kind: ReminderKind) {
        if let Some(r) = self.find_mut(kind) {
            r.elapsed = Duration::ZERO;
        }
    }

    /// Acumula `elapsed` de trabalho ATIVO e dispara no máximo um lembrete.
    /// Fora de "trabalhando" o contador NÃO anda (ocioso/ausente/almoço pausam),
    /// então voltar de uma ausência não estoura lembretes.
    pub fn tick(
        &mut self,
        state: ActivityState,
        idle_seconds: f64,
        in_meeting: bool,
        elapsed: Duration,
    ) -> Option<ReminderPayload> {
        if state != ActivityState::Working {
            return None;
        }
        for r in self.reminders.iter_mut() {
            if r.enabled {
                r.elapsed += elapsed;
            }
        }
        for r in self.reminders.iter_mut() {
            if !r.enabled || r.elapsed < r.interval {
                continue;
            }
            // Em reunião: segura (mantém o tempo acumulado pra disparar depois).
            if in_meeting {
                continue;
            }
            // Deep focus: digitando sem parar → adia, até o teto.
            if idle_seconds < DEEP_FOCUS_IDLE && r.elapsed < r.interval + MAX_DEFER {
                continue;
            }
            let payload = ReminderPayload {
                kind: r.kind.id().to_string(),
                rotation: r.rotation,
            };
            r.rotation = r.rotation.wrapping_add(1);
            r.elapsed = Duration::ZERO;
            return Some(payload);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched() -> Schedule {
        Schedule::from_config(&Config::default())
    }
    const TICK: Duration = Duration::from_secs(5);

    #[test]
    fn nao_dispara_antes_do_intervalo() {
        let mut s = sched();
        assert!(s.tick(ActivityState::Working, 20.0, false, TICK).is_none());
    }

    #[test]
    fn dispara_quando_acumula_o_intervalo() {
        let mut s = sched();
        // 21min de trabalho de uma vez → eyes (20min) vence
        assert!(s
            .tick(ActivityState::Working, 20.0, false, Duration::from_secs(21 * 60))
            .is_some());
    }

    #[test]
    fn nao_acumula_quando_ausente() {
        let mut s = sched();
        // 2h ausente NÃO fazem o contador andar...
        assert!(s
            .tick(ActivityState::Away, 5000.0, false, Duration::from_secs(2 * 3600))
            .is_none());
        // ...e ao voltar, ainda não está devido (nada acumulou na ausência).
        assert!(s.tick(ActivityState::Working, 20.0, false, TICK).is_none());
    }

    #[test]
    fn nao_dispara_em_reuniao() {
        let mut s = sched();
        assert!(s
            .tick(ActivityState::Working, 20.0, true, Duration::from_secs(21 * 60))
            .is_none());
    }

    #[test]
    fn adia_durante_deep_focus() {
        let mut s = sched();
        // devido, mas digitando sem parar (idle baixo) → adia
        assert!(s
            .tick(ActivityState::Working, 2.0, false, Duration::from_secs(21 * 60))
            .is_none());
    }

    #[test]
    fn dispara_apos_teto_de_adiamento() {
        let mut s = sched();
        assert!(s
            .tick(ActivityState::Working, 2.0, false, Duration::from_secs(21 * 60))
            .is_none());
        // passou do teto de adiamento, ainda em deep focus → dispara
        assert!(s
            .tick(ActivityState::Working, 2.0, false, MAX_DEFER)
            .is_some());
    }
}
