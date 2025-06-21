use base64::Engine;
use clap::{Parser, Subcommand};
use clip_vault_core::{Error, Result, SqliteVault, Vault};
use dialoguer::Password;
use serde::{Deserialize, Serialize};
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "clip-vault")] // binary name
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Remember the password for this long (e.g. 15m, 2h, 1d). Default 15m.
    #[arg(long, value_parser = humantime::parse_duration)]
    remember: Option<StdDuration>,

    /// Forget any cached password and exit.
    #[arg(long)]
    forget: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the latest clipboard entry
    Latest,
    /// Dummy list (placeholder – hook up DB later)
    List,
    /// Set up LaunchAgent/Service and vault password
    Setup,
}

#[derive(Serialize, Deserialize)]
struct Session {
    key: String,
    expires_at: u64,
}

fn cache_path() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("clip-vault")
        .join("session.json")
}

fn obtain_key(rem: Option<StdDuration>, forget: bool) -> Result<String> {
    use std::fs;
    let cache = cache_path();

    // forget flag wipes cache
    if forget && cache.exists() {
        let _ = fs::remove_file(&cache);
        println!("Password cache cleared.");
        std::process::exit(0);
    }

    // env var override
    if let Ok(ev) = std::env::var("CLIP_VAULT_KEY") {
        return Ok(ev);
    }

    // try cache
    if let Ok(text) = fs::read_to_string(&cache) {
        if let Ok(sess) = serde_json::from_str::<Session>(&text) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            if now < sess.expires_at {
                return Ok(sess.key);
            }
        }
    }

    // prompt
    let prompt = "Vault password";
    let pass = Password::new()
        .with_prompt(prompt)
        .interact()
        .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // write cache
    let duration = rem.unwrap_or_else(|| StdDuration::from_secs(15 * 60));
    let expires = SystemTime::now() + duration;
    let sess = Session {
        key: pass.clone(),
        expires_at: expires.duration_since(UNIX_EPOCH)?.as_secs(),
    };
    fs::create_dir_all(cache.parent().unwrap())?;
    let json = serde_json::to_vec(&sess)
        .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    fs::write(&cache, json)?;

    Ok(pass)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let key = obtain_key(cli.remember, cli.forget)?;

    match cli.command {
        Commands::Latest => cmd_latest(&key)?,
        Commands::List => cmd_list(&key)?,
        Commands::Setup => cmd_setup()?,
    }

    Ok(())
}

fn cmd_latest(key: &str) -> Result<()> {
    let store = open_store_with_key(key)?;
    if let Some(item) = store.latest()? {
        println!("{item:?}");
    } else {
        println!("No clipboard entries found.");
    }
    Ok(())
}

fn cmd_list(key: &str) -> Result<()> {
    let store = open_store_with_key(key)?;
    println!("Listing {} entries (unordered):", store.len()?);
    Ok(())
}

fn cmd_setup() -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::{fs, path::PathBuf};

    // 1. Prompt for password twice
    let first = rpassword::prompt_password("Set vault password: ")?;
    let second = rpassword::prompt_password("Confirm password: ")?;
    if first != second {
        eprintln!("Passwords do not match.");
        std::process::exit(1);
    }

    // 2. Derive 32-byte key via SHA-256 of UTF-8 bytes
    let mut hasher = Sha256::new();
    hasher.update(first.as_bytes());
    let key = hasher.finalize();
    let key_b64 = base64::engine::general_purpose::STANDARD.encode(key);

    // 3. Create LaunchAgent plist on macOS (~/Library/LaunchAgents/com.clip-vault.daemon.plist)
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").expect("HOME not set");
        let launch_agents = PathBuf::from(home).join("Library/LaunchAgents");
        fs::create_dir_all(&launch_agents)?;

        let plist_path = launch_agents.join("com.clip-vault.daemon.plist");

        let exe = std::env::current_exe()?;
        // assume daemon binary is alongside CLI (../clip-vault-daemon)
        let daemon_path = exe.with_file_name("clip-vault-daemon");

        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.clip-vault.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>CLIP_VAULT_KEY</key><string>{}</string>
    </dict>
    <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
            daemon_path.display(),
            key_b64
        );

        fs::write(&plist_path, plist_content)?;

        println!("LaunchAgent written to {}", plist_path.display());
        println!(
            "Run 'launchctl load {}' to start the daemon now.",
            plist_path.display()
        );
    }

    #[cfg(not(target_os = "macos"))]
    println!("Bootstrap currently implemented only for macOS (launchd). Please set CLIP_VAULT_KEY and run the daemon manually on this platform.");

    Ok(())
}

fn open_store_with_key(key: &str) -> Result<SqliteVault> {
    match SqliteVault::open("clip_vault_db", key) {
        Ok(s) => Ok(s),
        Err(err) => {
            if let Error::Sqlite(sql_err) = &err {
                if sql_err.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy) {
                    eprintln!("Database is busy (writer active). Unable to open store.");
                    std::process::exit(1);
                }
            }
            Err(err)
        }
    }
}
