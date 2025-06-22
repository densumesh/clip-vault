// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clip_vault_core::{default_db_path, ClipboardItem, SqliteVault, Vault};
use image::{ImageBuffer, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tokio::sync::mpsc;
use tracing::{info, warn};

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

#[derive(Debug, Default)]
struct DaemonState {
    is_running: bool,
    shutdown_sender: Option<mpsc::UnboundedSender<()>>,
    last_hash: Option<[u8; 32]>,
}

struct AppState {
    /// Vault is optional - only initialized after successful unlock
    vault: Arc<Mutex<Option<SqliteVault>>>,
    settings: Arc<Mutex<AppSettings>>,
    session: Arc<Mutex<Option<SessionInfo>>>,
    daemon: Arc<Mutex<DaemonState>>,
}

fn start_clipboard_monitoring(
    vault: Arc<Mutex<Option<SqliteVault>>>,
    daemon: Arc<Mutex<DaemonState>>,
    poll_interval_ms: u64,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut daemon_guard = daemon.lock().map_err(|_| "Daemon lock poisoned")?;

    if daemon_guard.is_running {
        return Ok(()); // Already running
    }

    let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
    daemon_guard.shutdown_sender = Some(shutdown_tx);
    daemon_guard.is_running = true;
    drop(daemon_guard);

    let vault_clone = vault.clone();
    let daemon_clone = daemon.clone();

    tokio::spawn(async move {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                warn!("Failed to create clipboard: {}", e);
                return;
            }
        };

        let mut last_hash: Option<[u8; 32]> = None;
        let poll_duration = Duration::from_millis(poll_interval_ms);

        info!("Clipboard monitoring started");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Clipboard monitoring shutdown requested");
                    break;
                }
                _ = tokio::time::sleep(poll_duration) => {
                    // Check if vault is still available
                    let vault_guard = match vault_clone.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            warn!("Vault lock poisoned, stopping daemon");
                            break;
                        }
                    };

                    if vault_guard.is_none() {
                        // Vault is locked, stop monitoring
                        break;
                    }

                    let clipboard_item = if let Ok(image_data) = clipboard.get_image() {
                        let image: RgbaImage = ImageBuffer::from_raw(
                            image_data.width.try_into().unwrap(),
                            image_data.height.try_into().unwrap(),
                            image_data.bytes.into_owned(),
                        ).unwrap();
                        let mut buffer = std::io::Cursor::new(Vec::new());
                        image.write_to(&mut buffer, ImageFormat::Png).unwrap();
                        Some(ClipboardItem::Image(buffer.into_inner()))
                    } else if let Ok(text) = clipboard.get_text() {
                        Some(ClipboardItem::Text(text))
                    } else {
                        None
                    };

                    if let Some(item) = clipboard_item {
                        let hash = item.hash();

                        if last_hash.map_or(true, |h| h != hash) {
                            let item_description = match &item {
                                ClipboardItem::Text(t) => format!("text: {}…", t.chars().take(40).collect::<String>()),
                                ClipboardItem::Image(data) => format!("image: {} bytes", data.len()),
                            };

                            info!("New clipboard {}", item_description);

                            if let Some(vault) = vault_guard.as_ref() {
                                if let Err(e) = vault.insert(hash, &item) {
                                    warn!("Failed to store clipboard item: {}", e);
                                } else {
                                    last_hash = Some(hash);

                                    // Update last hash in daemon state
                                    if let Ok(mut daemon_guard) = daemon_clone.lock() {
                                        daemon_guard.last_hash = Some(hash);
                                    }

                                    // Emit event to frontend about new clipboard item
                                    let _ = app_handle.emit("clipboard-updated", ());

                                    info!("New clipboard item stored successfully");
                                }
                            }
                        }
                    }
                }
            }
        }

        // Mark daemon as stopped
        if let Ok(mut daemon_guard) = daemon_clone.lock() {
            daemon_guard.is_running = false;
            daemon_guard.shutdown_sender = None;
            daemon_guard.last_hash = last_hash;
        }

        info!("Clipboard monitoring stopped");
    });

    Ok(())
}

fn stop_clipboard_monitoring(daemon: Arc<Mutex<DaemonState>>) -> Result<(), String> {
    let mut daemon_guard = daemon.lock().map_err(|_| "Daemon lock poisoned")?;

    if !daemon_guard.is_running {
        return Ok(()); // Already stopped
    }

    if let Some(sender) = daemon_guard.shutdown_sender.take() {
        let _ = sender.send(());
    }

    daemon_guard.is_running = false;
    Ok(())
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
                    clip_vault_core::ClipboardItem::Image(_) => {
                        query.to_lowercase().contains("image")
                            || query.to_lowercase().contains("png")
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
async fn copy_to_clipboard(content: String, _content_type: String) -> Result<(), String> {
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
async fn unlock_vault(
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

            start_clipboard_monitoring(
                state.vault.clone(),
                state.daemon.clone(),
                poll_interval,
                app,
            )?;

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
async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    show_settings_window(&app);
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
async fn start_daemon(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let poll_interval = {
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned")?;
        settings.poll_interval_ms
    };

    start_clipboard_monitoring(
        state.vault.clone(),
        state.daemon.clone(),
        poll_interval,
        app,
    )
}

#[tauri::command]
async fn stop_daemon(state: State<'_, AppState>) -> Result<(), String> {
    stop_clipboard_monitoring(state.daemon.clone())
}

#[tauri::command]
async fn daemon_status(state: State<'_, AppState>) -> Result<bool, String> {
    let daemon_guard = state.daemon.lock().map_err(|_| "Daemon lock poisoned")?;
    Ok(daemon_guard.is_running)
}

#[tauri::command]
async fn update_item(
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
    let _ = app.emit("clipboard-updated", ());

    info!("Item updated successfully");
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
                .inner_size(1000.0, 600.0)
                .min_inner_size(800.0, 500.0)
                .center()
                .resizable(true)
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
                            eprintln!("Failed to start daemon: {}", e);
                        }
                    }
                });
            }
            "daemon_stop" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if let Err(e) = stop_daemon(state).await {
                            eprintln!("Failed to stop daemon: {}", e);
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
        daemon: Arc::new(Mutex::new(DaemonState::default())), // No daemon running
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
            quit_app,
            start_daemon,
            stop_daemon,
            daemon_status,
            update_item
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
