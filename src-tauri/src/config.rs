//! Configuração do usuário, persistida em `~/Library/Application Support/Jbuddy/config.json`.
//!
//! Carregada uma vez no boot. Se o arquivo não existe, grava os defaults.
//! (A tela de configurações que edita isso vem como passo seguinte da Fase 2.)

use serde::{Deserialize, Serialize};

use crate::store::data_dir;

/// Config de um lembrete individual.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReminderConfig {
    pub enabled: bool,
    pub interval_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub water: ReminderConfig,
    pub stretch: ReminderConfig,
    pub eyes: ReminderConfig,
    /// Idle (s) a partir do qual passa de "trabalhando" pra "ocioso".
    pub idle_secs: f64,
    /// Idle (s) a partir do qual passa de "ocioso" pra "ausente".
    pub away_secs: f64,
    /// Quanto tempo a soneca adia um lembrete.
    pub snooze_secs: u64,
    pub work_start_hour: u32,
    pub work_end_hour: u32,
    /// JANELA de almoço (ex: 12–14h): a ausência DENTRO dela, até `lunch_minutes`
    /// no total, é entendida como o almoço e NÃO conta como ausência. O excedente
    /// volta a contar. `start == end` desliga.
    pub lunch_start_hour: u32,
    pub lunch_end_hour: u32,
    pub lunch_minutes: u64,
    /// Meta de horas focadas por dia — alimenta a guarda anti-excesso.
    pub target_work_hours: f64,
    /// "Impede o notebook de desligar a tela" — keep-alive de sessão (scroll-zero)
    /// só em horário de trabalho e quando você está fora. Opt-in. Default off.
    pub keep_screen_awake: bool,
    /// Cor do mascote: "green" ou "pink".
    pub mascot_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            water: ReminderConfig { enabled: true, interval_secs: 45 * 60 },
            stretch: ReminderConfig { enabled: true, interval_secs: 60 * 60 },
            eyes: ReminderConfig { enabled: true, interval_secs: 20 * 60 },
            idle_secs: 90.0,
            away_secs: 300.0,
            snooze_secs: 10 * 60,
            work_start_hour: 9,
            work_end_hour: 18,
            lunch_start_hour: 12,
            lunch_end_hour: 14,
            lunch_minutes: 60,
            target_work_hours: 8.0,
            keep_screen_awake: false,
            mascot_color: "green".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = data_dir().join("config.json");
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => {
                let cfg = Config::default();
                cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) {
        let dir = data_dir();
        std::fs::create_dir_all(&dir).ok();
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(dir.join("config.json"), s).ok();
        }
    }
}
