use clip_vault_core::{default_db_path, SqliteVault};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub poll_interval_ms: u64,
    pub vault_path: String,
    pub auto_lock_minutes: u32,
    pub global_shortcut: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100,
            vault_path: default_db_path().to_string_lossy().to_string(),
            auto_lock_minutes: 60, // Default to 1 hour
            global_shortcut: if cfg!(target_os = "macos") {
                "Shift+Cmd+C".to_string()
            } else {
                "Shift+Ctrl+C".to_string()
            },
        }
    }
}

#[derive(Debug)]
pub struct SessionInfo {
    pub last_activity: u64,
}

#[derive(Debug, Default)]
pub struct DaemonState {
    pub is_running: bool,
    pub shutdown_sender: Option<mpsc::UnboundedSender<()>>,
    pub last_hash: Option<[u8; 32]>,
}

pub struct AppState {
    /// Vault is optional - only initialized after successful unlock
    pub vault: Arc<Mutex<Option<SqliteVault>>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub session: Arc<Mutex<Option<SessionInfo>>>,
    pub daemon: Arc<Mutex<DaemonState>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            vault: Arc::new(Mutex::new(None)), // No vault initialized
            settings: Arc::new(Mutex::new(AppSettings::default())),
            session: Arc::new(Mutex::new(None)), // No session active
            daemon: Arc::new(Mutex::new(DaemonState::default())), // No daemon running
        }
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn is_session_expired(session: &SessionInfo, auto_lock_minutes: u32) -> bool {
    let now = current_timestamp();
    let session_duration_secs = u64::from(auto_lock_minutes) * 60;
    now > session.last_activity + session_duration_secs
}
