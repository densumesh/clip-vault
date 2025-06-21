use arboard::Clipboard;
use clip_vault_core::ClipboardItem;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let mut clipboard = Clipboard::new()?;
    let mut last_hash: Option<[u8; 32]> = None;

    loop {
        if let Ok(text) = clipboard.get_text() {
            let item = ClipboardItem::Text(text);
            let hash = item.hash();

            if last_hash.map(|h| h != hash).unwrap_or(true) {
                info!(
                    "New clipboard text: {}…",
                    match &item {
                        ClipboardItem::Text(t) => t.chars().take(40).collect::<String>(),
                    }
                );

                // TODO: persist to sled DB
                // db.insert(hash, bincode::serialize(&item)?)?;
                last_hash = Some(hash);
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}
