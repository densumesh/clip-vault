// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clip_vault_core::{default_db_path, SqliteVault, Vault};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::image::Image;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    id: String,
    content: String,
    timestamp: u64,
    content_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    poll_interval_ms: u64,
    vault_path: String,
    auto_lock_minutes: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100,
            vault_path: default_db_path().to_string_lossy().to_string(),
            auto_lock_minutes: 60, // Default to 1 hour
        }
    }
}

#[derive(Debug)]
struct SessionInfo {
    unlocked_at: u64,
    last_activity: u64,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn is_session_expired(session: &SessionInfo, auto_lock_minutes: u32) -> bool {
    let now = current_timestamp();
    let session_duration_secs = (auto_lock_minutes as u64) * 60;
    now > session.last_activity + session_duration_secs
}

struct AppState {
    /// Vault is optional - only initialized after successful unlock
    vault: Arc<Mutex<Option<SqliteVault>>>,
    settings: Arc<Mutex<AppSettings>>,
    session: Arc<Mutex<Option<SessionInfo>>>,
}

#[tauri::command]
async fn search_clipboard(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let vault_guard = state.vault.lock().map_err(|_| "Vault lock poisoned")?;
    let vault = vault_guard.as_ref().ok_or("Vault not unlocked")?;
    
    let items = vault.list(None).map_err(|e| e.to_string())?;

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

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned")?;
    Ok(settings.clone())
}

#[tauri::command]
async fn save_settings(
    new_settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned")?;
    *settings = new_settings;
    // TODO: Persist settings to file or config
    Ok(())
}

#[tauri::command]
async fn unlock_vault(password: String, state: State<'_, AppState>) -> Result<bool, String> {
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
            *session = Some(SessionInfo {
                unlocked_at: now,
                last_activity: now,
            });
            
            Ok(true)
        }
        Err(e) => {
            eprintln!("Failed to unlock vault: {}", e);
            Ok(false)
        }
    }
}

#[tauri::command]
async fn check_vault_status(state: State<'_, AppState>) -> Result<bool, String> {
    // Check if vault is unlocked and session is valid
    let auto_lock_minutes = {
        let settings = state.settings.lock().map_err(|_| "Settings lock poisoned")?;
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
async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    show_settings_window(&app);
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
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
                .resizable(false)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .transparent(true)
                .build()
                .expect("Failed to create search window");
    }
}

fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _window =
            WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
                .title("Clip Vault Settings")
                .inner_size(500.0, 550.0)
                .center()
                .resizable(false)
                .decorations(true)
                .always_on_top(false)
                .skip_taskbar(false)
                .build()
                .expect("Failed to create settings window");
    }
}

fn create_system_tray(app: &AppHandle) -> tauri::Result<()> {
    let search_item = MenuItem::with_id(app, "search", "Search Clipboard", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Clip Vault", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&search_item, &settings_item, &quit_item])?;

    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Clip Vault")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "search" => {
                show_search_window(app);
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

pub fn run() {
    // Initialize directory for vault but don't create vault yet
    let db_path = default_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    let app_state = AppState {
        vault: Arc::new(Mutex::new(None)), // No vault initialized
        settings: Arc::new(Mutex::new(AppSettings::default())),
        session: Arc::new(Mutex::new(None)), // No session active
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            // Hide the main window immediately
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.hide();
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
            search_clipboard,
            copy_to_clipboard,
            delete_item,
            get_settings,
            save_settings,
            unlock_vault,
            check_vault_status,
            open_settings_window,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
