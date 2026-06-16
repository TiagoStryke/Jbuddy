//! Jbuddy — companheiro de trabalho saudável (app de bandeja).
//!
//! Fase 1: tracking real de atividade. Sobe um ícone na bandeja com o tempo
//! efetivo de hoje (timer vivo) e uma thread que mede atividade real e persiste.

mod idle;
mod store;
mod tracker;

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State};

use tracker::{SharedSnapshot, Snapshot};

/// Retorna o snapshot do dia (estado atual + segundos por categoria).
#[tauri::command]
fn get_today_stats(snap: State<'_, SharedSnapshot>) -> Snapshot {
    snap.lock().map(|s| s.clone()).unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let snapshot: SharedSnapshot = Arc::new(Mutex::new(Snapshot::default()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(snapshot.clone())
        .invoke_handler(tauri::generate_handler![get_today_stats])
        .setup(move |app| {
            // App de bandeja puro: sem ícone na Dock (macOS).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();

            // Menu do tray (itens de status são atualizados pelo tracker).
            let today_item =
                MenuItem::with_id(&handle, "today", "Hoje: calculando…", false, None::<&str>)?;
            let status_item =
                MenuItem::with_id(&handle, "status", "Estado: —", false, None::<&str>)?;
            let open_item =
                MenuItem::with_id(&handle, "open", "Abrir painel", true, None::<&str>)?;
            let quit_item = PredefinedMenuItem::quit(&handle, Some("Sair do Jbuddy"))?;
            let sep = PredefinedMenuItem::separator(&handle)?;
            let menu = Menu::with_items(
                &handle,
                &[&today_item, &status_item, &sep, &open_item, &quit_item],
            )?;

            let mut tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .title("Jbuddy")
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "open" {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(&handle)?;

            // Thread de monitoramento (vive enquanto o app viver).
            let snap = snapshot.clone();
            std::thread::spawn(move || {
                tracker::run_loop(handle, snap, today_item, status_item);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
