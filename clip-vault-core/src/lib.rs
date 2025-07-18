//! Core data types shared by daemon & CLI.

use base64::{engine::general_purpose, Engine as _};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Encode, Decode)]
pub enum ClipboardItem {
    Text(String),
    Image(Vec<u8>),
}

impl ClipboardItem {
    /// Deterministic hash (duplicate detection).
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match self {
            ClipboardItem::Text(t) => hasher.update(t.as_bytes()),
            ClipboardItem::Image(data) => {
                hasher.update(data);
            }
        }
        hasher.finalize().into()
    }

    #[must_use]
    pub fn into_parts(self) -> (String, String) {
        match self {
            ClipboardItem::Text(t) => (t, "text/plain".to_string()),
            ClipboardItem::Image(data) => {
                // Convert image data to base64 for transport
                (
                    general_purpose::STANDARD.encode(&data),
                    "image/png".to_string(),
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardItemWithTimestamp {
    pub item: ClipboardItem,
    pub timestamp: u64,
}

mod error;
mod store;

pub use error::{Error, Result};
pub use store::{SqliteVault, Vault};

#[must_use]
pub fn default_db_path() -> PathBuf {
    if let Ok(path) = std::env::var("CLIP_VAULT_DB_PATH") {
        PathBuf::from(path)
    } else {
        let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
        base.join("clip-vault").join("clip_vault.db")
    }
}
