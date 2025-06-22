use clip_vault_core::{ClipboardItem, SqliteVault, Vault};
use image::{ImageBuffer, ImageFormat, RgbaImage};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::state::DaemonState;

pub fn start_clipboard_monitoring(
    vault: &Arc<Mutex<Option<SqliteVault>>>,
    daemon: &Arc<Mutex<DaemonState>>,
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
                () = tokio::time::sleep(poll_duration) => {
                    // Check if vault is still available
                    let Ok(vault_guard) = vault_clone.lock() else {
                        warn!("Vault lock poisoned, stopping daemon");
                        break;
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

                        if last_hash != Some(hash) {
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
                                    app_handle.emit("clipboard-updated", ()).ok();

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

pub fn stop_clipboard_monitoring(daemon: &Arc<Mutex<DaemonState>>) -> Result<(), String> {
    let mut daemon_guard = daemon.lock().map_err(|_| "Daemon lock poisoned")?;

    if !daemon_guard.is_running {
        return Ok(()); // Already stopped
    }

    if let Some(sender) = daemon_guard.shutdown_sender.take() {
        sender.send(()).ok();
    }

    daemon_guard.is_running = false;
    Ok(())
}
