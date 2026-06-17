//! Localização do lado nativo (tray + rótulo de estado).
//!
//! Toda a cópia visível do WEBVIEW vive no `src/i18n.ts`. Aqui ficam só as
//! poucas strings nativas (menu do tray) que não dá pra traduzir no frontend.
//! Default: inglês; PT quando o locale do sistema começa com "pt".

use std::sync::OnceLock;

use sys_locale::get_locale;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Pt,
}

/// Idioma resolvido uma vez por execução.
pub fn lang() -> Lang {
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| match get_locale() {
        Some(l) if l.to_lowercase().starts_with("pt") => Lang::Pt,
        _ => Lang::En,
    })
}

/// Rótulo do estado de atividade a partir da chave estável.
pub fn state_label(state_id: &str) -> &'static str {
    match (lang(), state_id) {
        (Lang::Pt, "working") => "trabalhando",
        (Lang::Pt, "idle") => "ocioso",
        (Lang::Pt, "away") => "ausente",
        (Lang::En, "working") => "working",
        (Lang::En, "idle") => "idle",
        (Lang::En, "away") => "away",
        _ => "—",
    }
}

/// Strings do menu do tray.
pub struct TrayStrings {
    pub today: &'static str,
    pub status: &'static str,
    pub open: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
}

pub fn tray() -> TrayStrings {
    match lang() {
        Lang::Pt => TrayStrings {
            today: "Hoje",
            status: "Estado",
            open: "Abrir painel",
            settings: "Configurações",
            quit: "Sair do Jbuddy",
        },
        Lang::En => TrayStrings {
            today: "Today",
            status: "Status",
            open: "Open panel",
            settings: "Settings",
            quit: "Quit Jbuddy",
        },
    }
}
