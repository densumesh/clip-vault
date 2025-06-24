// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clip_vault_core::default_db_path;
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

mod commands;
mod modules;
mod state;

use commands::{
    check_vault_status, copy_to_clipboard, daemon_status, delete_item, get_settings,
    list_clipboard, open_settings_window, quit_app, save_settings, search_clipboard, start_daemon, stop_daemon,
    unlock_vault, update_item,
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
        .manage(app_state)
        .setup(|app| {
            // Hide the main window immediately
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.hide().ok();
            }

            // Create system tray
            create_system_tray(app.handle())?;

            // Register global shortcut
            let app_handle = app.handle().clone();

            let gs = app.global_shortcut();
            gs.on_shortcut("Shift+Cmd+C", move |_, _, _| {
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
            update_item
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
