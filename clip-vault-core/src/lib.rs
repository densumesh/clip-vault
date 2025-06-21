//! Core data types shared by daemon & CLI.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardItem {
    Text(String),
    // Image(Vec<u8>)   // add later
}

impl ClipboardItem {
    /// Deterministic hash (duplicate detection).
    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match self {
            ClipboardItem::Text(t) => hasher.update(t.as_bytes()),
        }
        hasher.finalize().into()
    }
}

mod error;
mod store;

pub use error::{Error, Result};
pub use store::{SqliteVault, Vault};

#[must_use]
pub fn default_db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    base.join("clip-vault").join("clip_vault.db")
}
