use arboard::Clipboard;
use clip_vault_core::{ClipboardItem, SqliteVault, Vault};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if env::var("CLIP_VAULT_FOREGROUND").is_err() {
            let stdout = fs::File::create("/tmp/clip-vault.out")?;
            let stderr = fs::File::create("/tmp/clip-vault.err")?;
            Daemonize::new()
                .pid_file("/tmp/clip-vault.pid")
                .chown_pid_file(true)
                .working_directory("/")
                .stdout(stdout)
                .stderr(stderr)
                .start()?;
        }
    }

    let mut clipboard = Clipboard::new()?;
    let key = env::var("CLIP_VAULT_KEY").unwrap_or_default();
    let db_path = clip_vault_core::default_db_path();
    std::fs::create_dir_all(db_path.parent().unwrap())?;
    let store = SqliteVault::open(db_path, &key)?;
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
