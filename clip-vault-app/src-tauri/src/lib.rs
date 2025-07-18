// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clip_vault_core::default_db_path;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

mod commands;
mod modules;
mod state;

use commands::{
    check_for_updates, check_vault_status, copy_to_clipboard, create_vault, daemon_status,
    delete_item, get_platform, get_settings, install_update, list_clipboard, open_settings_window,
    quit_app, save_settings, search_clipboard, show_toast_notification, start_daemon, stop_daemon, unlock_vault,
    update_item, vault_exists,
};
use modules::{system_tray::create_system_tray, window_manager::show_search_window};
use state::AppState;

/// Bootstraps the Tauri application.
///
/// # Panics
/// Panics if the application context cannot be created, a required directory
/// cannot be created, or if the Tauri runtime fails to start.
#[allow(clippy::missing_panics_doc)]
pub fn run() {
    // Initialize directory for vault but don't create vault yet
    let db_path = default_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            // Hide the main window immediately
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.hide().ok();
            }

            // Create system tray
            create_system_tray(app.handle())?;
            show_search_window(app.handle());

            // Register global shortcut from settings
            let app_handle = app.handle().clone();
            let shortcut = {
                let app_state = app.state::<AppState>();
                let settings = app_state
                    .settings
                    .lock()
                    .map_err(|_| "Settings lock poisoned")?;
                settings.global_shortcut.clone()
            };

            let gs = app.global_shortcut();
            let parsed_shortcut: Shortcut = shortcut
                .parse()
                .map_err(|e| format!("Invalid shortcut: {e}"))?;
            gs.on_shortcut(parsed_shortcut, move |_, _, _| {
                show_search_window(&app_handle);
            })?;

            // Hide from dock on macOS but keep in menu bar
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_clipboard,
            search_clipboard,
            copy_to_clipboard,
            delete_item,
            get_settings,
            save_settings,
            unlock_vault,
            check_vault_status,
            open_settings_window,
            quit_app,
            start_daemon,
            stop_daemon,
            daemon_status,
            update_item,
            vault_exists,
            create_vault,
            get_platform,
            check_for_updates,
            install_update,
            show_toast_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
