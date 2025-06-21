use arboard::Clipboard;
use clip_vault_core::{ClipboardItem, SqliteVault, Vault};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let mut clipboard = Clipboard::new()?;
    let key = env::var("CLIP_VAULT_KEY").unwrap_or_default();
    let store = SqliteVault::open("clip_vault_db", &key)?;
    let mut last_hash: Option<[u8; 32]> = None;

    loop {
        if let Ok(text) = clipboard.get_text() {
            let item = ClipboardItem::Text(text);
            let hash = item.hash();

            if last_hash.map_or(true, |h| h != hash) {
                info!(
                    "New clipboard text: {}…",
                    match &item {
                        ClipboardItem::Text(t) => t.chars().take(40).collect::<String>(),
                    }
                );

                store.insert(hash, &item)?;
                last_hash = Some(hash);
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}
