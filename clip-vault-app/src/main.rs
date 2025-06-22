// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clip_vault_core::{default_db_path, SqliteVault, Vault};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    id: String,
    content: String,
    timestamp: u64,
    content_type: String,
}

struct AppState {
    /// Wrapped in a mutex so it can be shared safely across tauri threads.
    vault: Arc<Mutex<SqliteVault>>,
}

#[tauri::command]
async fn search_clipboard(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let items = state
        .vault
        .lock()
        .map_err(|_| "Vault lock poisoned")?
        .list(None)
        .map_err(|e| e.to_string())?;

    let results: Vec<SearchResult> = items
        .into_iter()
        .filter(|item| {
            if query.is_empty() {
                true
            } else {
                match &item.item {
                    clip_vault_core::ClipboardItem::Text(text) => {
                        text.to_lowercase().contains(&query.to_lowercase())
                    }
                }
            }
        })
        .map(|item| {
            let (content, content_type) = item.item.clone().into_parts();
            SearchResult {
                id: format!("{}", item.timestamp),
                content,
                timestamp: item.timestamp,
                content_type,
            }
        })
        .collect();

    Ok(results)
}

#[tauri::command]
async fn copy_to_clipboard(content: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn delete_item(_timestamp: u64, _state: State<'_, AppState>) -> Result<(), String> {
    // Note: This would require adding a delete method to the Vault trait
    // For now, return an error indicating it's not implemented
    Err("Delete functionality not yet implemented in vault".to_string())
}

fn show_search_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("search") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _window =
            WebviewWindowBuilder::new(app, "search", WebviewUrl::App("index.html".into()))
                .title("Clip Vault Search")
                .inner_size(600.0, 400.0)
                .center()
                .resizable(true)
                .decorations(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .build()
                .expect("Failed to create search window");
    }
}

fn main() {
    // Initialize vault
    let db_path = default_db_path();

    // Ensure the database directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    // For now, use a default password - in production this should be handled properly
    let vault =
        SqliteVault::open(&db_path, "default_password").expect("Failed to initialize vault");

    let app_state = AppState {
        vault: Arc::new(Mutex::new(vault)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            // Register global shortcut
            let app_handle = app.handle().clone();

            let gs = app.global_shortcut();
            gs.register("Shift+Cmd+C")?;
            gs.on_shortcut("Shift+Cmd+C", move |_, _, _| {
                show_search_window(&app_handle);
            })?;

            // Hide from dock on macOS
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_clipboard,
            copy_to_clipboard,
            delete_item
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
