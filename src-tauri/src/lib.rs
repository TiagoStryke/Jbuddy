//! Jbuddy — companheiro de trabalho saudável (app de bandeja).
//!
//! Fase 1: tracking real de atividade. Sobe um ícone na bandeja com o tempo
//! efetivo de hoje (timer vivo) e uma thread que mede atividade real e persiste.

mod config;
mod idle;
mod keepalive;
mod locale;
mod meeting;
mod reminders;
mod store;
mod tracker;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State};

use config::Config;
use reminders::{ReminderKind, Schedule, SharedSchedule};
use tracker::{SharedSnapshot, Snapshot};

/// Retorna o snapshot do dia (estado atual + segundos por categoria).
#[tauri::command]
fn get_today_stats(snap: State<'_, SharedSnapshot>) -> Snapshot {
    snap.lock().map(|s| s.clone()).unwrap_or_default()
}

type SharedConfig = Arc<Mutex<Config>>;

/// Config atual (pra tela de configurações ler).
#[tauri::command]
fn get_config(config: State<'_, SharedConfig>) -> Config {
    config.lock().map(|c| c.clone()).unwrap_or_default()
}

/// Buckets por hora de hoje (pro gráfico "hoje por hora").
#[tauri::command]
fn get_today_hourly() -> Vec<store::HourStat> {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    store::Store::open()
        .and_then(|s| s.day_hourly(&date))
        .unwrap_or_default()
}

/// Totais dos últimos 7 dias (pro gráfico da semana).
#[tauri::command]
fn get_week() -> Vec<store::DayStat> {
    let today = chrono::Local::now().date_naive();
    let from = (today - chrono::Duration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    let to = today.format("%Y-%m-%d").to_string();
    store::Store::open()
        .and_then(|s| s.range_daily(&from, &to))
        .unwrap_or_default()
}

/// Heatmap de foco por hora-do-dia em todo o histórico (pico × ocioso).
#[tauri::command]
fn get_heatmap() -> Vec<store::HourStat> {
    store::Store::open()
        .and_then(|s| s.heatmap())
        .unwrap_or_default()
}

/// Salva a config (da tela de configurações), persiste e reaplica ao vivo:
/// reconstrói o agendador com os novos intervalos. Thresholds são lidos da
/// config a cada tick, então já valem na hora.
#[tauri::command]
fn save_config(
    config: State<'_, SharedConfig>,
    schedule: State<'_, SharedSchedule>,
    new_config: Config,
) {
    if let Ok(mut c) = config.lock() {
        *c = new_config;
        c.save();
        if let Ok(mut s) = schedule.lock() {
            *s = Schedule::from_config(&c);
        }
    }
}

/// Ação do usuário num lembrete: "done" / "skip" (reagenda) ou "snooze" (adia curto).
/// Sempre esconde a janela de lembrete e libera o disparo do próximo.
#[tauri::command]
fn reminder_action(
    app: AppHandle,
    schedule: State<'_, SharedSchedule>,
    active: State<'_, Arc<AtomicBool>>,
    kind: String,
    action: String,
) {
    if let Some(k) = ReminderKind::from_id(&kind) {
        if let Ok(mut s) = schedule.lock() {
            match action.as_str() {
                "snooze" => s.snooze(k),
                _ => s.reschedule(k),
            }
        }
    }
    if let Some(win) = app.get_webview_window("reminder") {
        let _ = win.hide();
    }
    active.store(false, Ordering::Relaxed);
}

/// Posiciona a janela de lembrete no canto superior direito do monitor atual
/// (discreto, perto das notificações do macOS), com margem da barra de menu.
pub(crate) fn place_reminder(win: &tauri::WebviewWindow) {
    if let (Ok(Some(monitor)), Ok(size)) = (win.current_monitor(), win.outer_size()) {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        let scale = monitor.scale_factor();
        let margin = (16.0 * scale) as i32;
        let top = (44.0 * scale) as i32;
        let x = m_pos.x + m_size.width as i32 - size.width as i32 - margin;
        let y = m_pos.y + top;
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let snapshot: SharedSnapshot = Arc::new(Mutex::new(Snapshot::default()));
    let config: SharedConfig = Arc::new(Mutex::new(Config::load()));
    let schedule: SharedSchedule = {
        let c = config.lock().expect("config lock");
        Arc::new(Mutex::new(Schedule::from_config(&c)))
    };
    // true enquanto uma janela de lembrete está aberta (evita empilhar).
    let reminder_active = Arc::new(AtomicBool::new(false));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(snapshot.clone())
        .manage(config.clone())
        .manage(schedule.clone())
        .manage(reminder_active.clone())
        .invoke_handler(tauri::generate_handler![
            get_today_stats,
            get_config,
            save_config,
            reminder_action,
            get_today_hourly,
            get_week,
            get_heatmap
        ])
        .setup(move |app| {
            // App de bandeja puro: sem ícone na Dock (macOS).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();

            // Menu do tray (itens de status são atualizados pelo tracker).
            let t = locale::tray();
            let today_item = MenuItem::with_id(
                &handle,
                "today",
                format!("{}: …", t.today),
                false,
                None::<&str>,
            )?;
            let status_item = MenuItem::with_id(
                &handle,
                "status",
                format!("{}: —", t.status),
                false,
                None::<&str>,
            )?;
            let open_item = MenuItem::with_id(&handle, "open", t.open, true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(&handle, "settings", t.settings, true, None::<&str>)?;
            let quit_item = PredefinedMenuItem::quit(&handle, Some(t.quit))?;
            let sep = PredefinedMenuItem::separator(&handle)?;
            let menu = Menu::with_items(
                &handle,
                &[
                    &today_item,
                    &status_item,
                    &sep,
                    &open_item,
                    &settings_item,
                    &quit_item,
                ],
            )?;

            // Ícone embutido em tempo de compilação — garante que o tray sempre
            // tem imagem (não depende de default_window_icon vir Some).
            let tray_icon = tauri::include_image!("icons/32x32.png");
            // IMPORTANTE: guardar o handle. Se o `TrayIcon` for descartado, o macOS
            // remove o item do menu bar. Ele vai morar na thread de monitoramento.
            let tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("Jbuddy")
                .title("Jbuddy")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(&handle)?;

            // Fechar a janela (X) só ESCONDE — o app continua vivo na bandeja.
            // Sair de verdade é só pelo menu "Sair do Jbuddy".
            for label in ["main", "settings"] {
                if let Some(win) = handle.get_webview_window(label) {
                    let win_for_event = win.clone();
                    win.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = win_for_event.hide();
                        }
                    });
                }
            }

            // Thread de monitoramento (vive enquanto o app viver). Leva o `tray`
            // junto pra mantê-lo vivo e atualizar o título por ele.
            let snap = snapshot.clone();
            std::thread::spawn(move || {
                tracker::run_loop(handle, snap, tray, today_item, status_item);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
