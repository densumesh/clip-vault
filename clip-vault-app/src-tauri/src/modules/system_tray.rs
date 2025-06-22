use crate::commands::{start_daemon, stop_daemon};
use crate::modules::window_manager::{show_search_window, show_settings_window};
use crate::state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub fn create_system_tray(app: &AppHandle) -> tauri::Result<()> {
    let search_item = MenuItem::with_id(app, "search", "Search Clipboard", true, None::<&str>)?;
    let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let daemon_start_item =
        MenuItem::with_id(app, "daemon_start", "Start Daemon", true, None::<&str>)?;
    let daemon_stop_item =
        MenuItem::with_id(app, "daemon_stop", "Stop Daemon", true, None::<&str>)?;
    let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Clip Vault", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &search_item,
            &separator1,
            &daemon_start_item,
            &daemon_stop_item,
            &separator2,
            &settings_item,
            &quit_item,
        ],
    )?;

    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Clip Vault")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "search" => {
                show_search_window(app);
            }
            "daemon_start" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if let Err(e) = start_daemon(state, app_handle.clone()).await {
                            eprintln!("Failed to start daemon: {e}");
                        }
                    }
                });
            }
            "daemon_stop" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if let Err(e) = stop_daemon(state).await {
                            eprintln!("Failed to stop daemon: {e}");
                        }
                    }
                });
            }
            "settings" => {
                show_settings_window(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                // Handle tray icon click - could open search window
            }
        })
        .build(app)?;

    Ok(())
}
