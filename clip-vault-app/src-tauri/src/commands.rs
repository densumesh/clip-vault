#![allow(clippy::used_underscore_binding)]

use arboard::ImageData;
use base64::engine::general_purpose;
use base64::Engine;
use clip_vault_core::{ClipboardItem, SqliteVault, Vault};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tracing::info;

use crate::modules::clipboard_monitor::{start_clipboard_monitoring, stop_clipboard_monitoring};
use crate::modules::window_manager::show_settings_window;
use crate::state::{current_timestamp, is_session_expired, AppSettings, AppState, SessionInfo};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub timestamp: u64,
    pub content_type: String,
}

#[tauri::command]
pub async fn list_clipboard(
    limit: Option<usize>,
    after_timestamp: Option<u64>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let vault_guard = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Default limit to 20 if not specified
    let effective_limit = limit.or(Some(20));

    let items = vault
        .list(effective_limit, after_timestamp)
        .map_err(|e| e.to_string())?;

    let results: Vec<SearchResult> = items
        .into_iter()
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
pub async fn search_clipboard(
    query: String,
    limit: Option<usize>,
    after_timestamp: Option<u64>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let vault_guard = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    // Default limit to 20 if not specified
    let effective_limit = limit.or(Some(20));

    let items = vault
        .search(&query, effective_limit, after_timestamp)
        .map_err(|e| e.to_string())?;

    let results: Vec<SearchResult> = items
        .into_iter()
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
pub async fn copy_to_clipboard(content: String, content_type: String) -> Result<(), String> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    if content_type == "text/plain" {
        clipboard.set_text(content).map_err(|e| e.to_string())?;
    } else if content_type == "image/png" {
        let image_data = general_purpose::STANDARD
            .decode(content)
            .map_err(|e| e.to_string())?;
        let image = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;

        let data = ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: Cow::from(image.to_rgba8().into_raw()),
        };
        clipboard.set_image(data).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_item(
    content: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // compute hash of content (text only)
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash);

    let vault_guard = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    vault.delete(arr).map_err(|e| e.to_string())?;

    // Emit event to refresh search results
    app.emit("clipboard-updated", ()).ok();

    info!("Item deleted successfully");
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned")?;
    Ok(settings.clone())
}

#[tauri::command]
pub async fn save_settings(
    new_settings: AppSettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Check if global shortcut changed
    let old_shortcut = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        settings.global_shortcut.clone()
    };

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned")?;

    if old_shortcut != new_settings.global_shortcut {
        // Update the global shortcut
        let gs = app.global_shortcut();

        // Unregister old shortcut
        if let Err(e) = gs.unregister(
            old_shortcut
                .parse::<Shortcut>()
                .map_err(|e| format!("Invalid old shortcut: {e}"))?,
        ) {
            eprintln!("Failed to unregister old shortcut: {e}");
        }

        // Register new shortcut
        let app_handle = app.app_handle().clone();
        let new_shortcut: Shortcut = new_settings
            .global_shortcut
            .parse()
            .map_err(|e| format!("Invalid shortcut: {e}"))?;
        gs.on_shortcut(new_shortcut, move |_, _, _| {
            crate::modules::window_manager::show_search_window(&app_handle);
        })
        .map_err(|e| format!("Failed to register new shortcut: {e}"))?;
    }

    *settings = new_settings;
    // TODO: Persist settings to file or config
    Ok(())
}

#[tauri::command]
pub async fn vault_exists(state: State<'_, AppState>) -> Result<bool, String> {
    let vault_path = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        PathBuf::from(&settings.vault_path)
    };

    Ok(vault_path.exists())
}

#[tauri::command]
pub async fn create_vault(
    password: String,
    settings: AppSettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<bool, String> {
    // Check if global shortcut changed
    let old_shortcut = {
        let app_settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        app_settings.global_shortcut.clone()
    };

    // Update settings first
    {
        let mut app_settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;

        if old_shortcut != settings.global_shortcut {
            // Update the global shortcut
            let gs = app.global_shortcut();

            // Unregister old shortcut
            if let Err(e) = gs.unregister(
                old_shortcut
                    .parse::<Shortcut>()
                    .map_err(|e| format!("Invalid old shortcut: {e}"))?,
            ) {
                eprintln!("Failed to unregister old shortcut: {e}");
            }

            // Register new shortcut
            let app_handle = app.app_handle().clone();
            let new_shortcut: Shortcut = settings
                .global_shortcut
                .parse()
                .map_err(|e| format!("Invalid shortcut: {e}"))?;
            gs.on_shortcut(new_shortcut, move |_, _, _| {
                crate::modules::window_manager::show_search_window(&app_handle);
            })
            .map_err(|e| format!("Failed to register new shortcut: {e}"))?;
        }

        *app_settings = settings;
    }

    // Create the vault by attempting to open it (this creates it if it doesn't exist)
    let vault_path = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        PathBuf::from(&settings.vault_path)
    };

    match SqliteVault::open(&vault_path, &password) {
        Ok(new_vault) => {
            let mut vault = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
            *vault = Some(new_vault);

            // Create new session
            let now = current_timestamp();
            let mut session = state.session.lock().map_err(|_| "Session lock poisoned")?;
            *session = Some(SessionInfo { last_activity: now });
            drop(session);
            drop(vault);

            // Start clipboard monitoring
            let poll_interval = {
                let settings = state
                    .settings
                    .lock()
                    .map_err(|_| "Settings lock poisoned")?;
                settings.poll_interval_ms
            };

            start_clipboard_monitoring(&state.vault, &state.daemon, poll_interval, app)?;

            Ok(true)
        }
        Err(e) => {
            eprintln!("Failed to create vault: {e}");
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn unlock_vault(
    password: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<bool, String> {
    let vault_path = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        PathBuf::from(&settings.vault_path)
    };

    match SqliteVault::open(&vault_path, &password) {
        Ok(new_vault) => {
            let mut vault = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
            *vault = Some(new_vault);

            // Create new session
            let now = current_timestamp();
            let mut session = state.session.lock().map_err(|_| "Session lock poisoned")?;
            *session = Some(SessionInfo { last_activity: now });
            drop(session);
            drop(vault);

            // Start clipboard monitoring
            let poll_interval = {
                let settings = state
                    .settings
                    .lock()
                    .map_err(|_| "Settings lock poisoned")?;
                settings.poll_interval_ms
            };

            start_clipboard_monitoring(&state.vault, &state.daemon, poll_interval, app)?;

            Ok(true)
        }
        Err(e) => {
            eprintln!("Failed to unlock vault: {e}");
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn check_vault_status(state: State<'_, AppState>) -> Result<bool, String> {
    // Check if vault is unlocked and session is valid
    let auto_lock_minutes = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        settings.auto_lock_minutes
    };

    let mut session_guard = state.session.lock().map_err(|_| "Session lock poisoned")?;

    if let Some(session) = session_guard.as_ref() {
        if is_session_expired(session, auto_lock_minutes) {
            // Session expired, clear vault and session
            *session_guard = None;
            drop(session_guard);

            let mut vault_guard = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
            *vault_guard = None;
            drop(vault_guard);

            Ok(false) // Vault is locked due to expired session
        } else {
            // Update last activity
            if let Some(session) = session_guard.as_mut() {
                session.last_activity = current_timestamp();
            }
            Ok(true) // Vault is unlocked and session is valid
        }
    } else {
        Ok(false) // No active session
    }
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    show_settings_window(&app);
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn start_daemon(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let poll_interval = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        settings.poll_interval_ms
    };

    start_clipboard_monitoring(&state.vault, &state.daemon, poll_interval, app)?;
    Ok(())
}

#[tauri::command]
pub async fn stop_daemon(state: State<'_, AppState>) -> Result<(), String> {
    stop_clipboard_monitoring(&state.daemon)
}

#[tauri::command]
pub async fn daemon_status(state: State<'_, AppState>) -> Result<bool, String> {
    let daemon_guard = state.daemon.lock().map_err(|_| "Daemon lock poisoned")?;
    Ok(daemon_guard.is_running)
}

#[tauri::command]
pub async fn update_item(
    old_content: String,
    new_content: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // compute hash of old content (text only)
    let mut hasher = Sha256::new();
    hasher.update(old_content.as_bytes());
    let old_hash = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&old_hash);

    let new_item = ClipboardItem::Text(new_content.clone());

    let vault_guard = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;

    vault.update(arr, &new_item).map_err(|e| e.to_string())?;

    // Emit event to refresh search results
    app.emit("clipboard-updated", ()).ok();

    info!("Item updated successfully");
    Ok(())
}

#[tauri::command]
pub async fn get_platform() -> Result<String, String> {
    Ok(std::env::consts::OS.to_string())
}
