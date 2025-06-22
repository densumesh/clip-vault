use clap::{Parser, Subcommand};
use clip_vault_core::{Error, Result, SqliteVault, Vault};
use dialoguer::Password;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

mod tui;

#[derive(Parser)]
#[command(name = "clip-vault")] // binary name
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

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
    /// List clipboard entries (optionally specify how many)
    List {
        /// Number of entries to show (default: all)
        #[arg(short, long)]
        count: Option<usize>,
    },
    /// Search clipboard entries for a text pattern
    Search {
        /// Text pattern to search for
        query: String,
        /// Maximum number of results to show (default: all matches)
        #[arg(short, long)]
        count: Option<usize>,
    },
    /// Launch interactive TUI (Terminal User Interface)
    Tui,
    /// Set up LaunchAgent/Service and vault password
    Setup,
    /// Gracefully stop the running daemon
    Stop,
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
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;

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

    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Latest => {
            let key = obtain_key(cli.remember, cli.forget)?;
            cmd_latest(&key)?;
        }
        Commands::List { count } => {
            let key = obtain_key(cli.remember, cli.forget)?;
            cmd_list(&key, count)?;
        }
        Commands::Search { query, count } => {
            let key = obtain_key(cli.remember, cli.forget)?;
            cmd_search(&key, &query, count)?;
        }
        Commands::Tui => {
            let key = obtain_key(cli.remember, cli.forget)?;
            cmd_tui(&key)?;
        }
        Commands::Setup => cmd_setup()?,
        Commands::Stop => cmd_stop()?,
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

fn cmd_list(key: &str, count: Option<usize>) -> Result<()> {
    let store = open_store_with_key(key)?;
    let items = store.list(count)?;

    if items.is_empty() {
        println!("No clipboard entries found.");
        return Ok(());
    }

    match count {
        Some(n) => println!("Last {} clipboard entries:", n.min(items.len())),
        None => println!("All {} clipboard entries:", items.len()),
    }

    for (i, item) in items.iter().enumerate() {
        println!("{}. {:?}", i + 1, item);
    }

    Ok(())
}

fn cmd_search(key: &str, query: &str, count: Option<usize>) -> Result<()> {
    let store = open_store_with_key(key)?;
    let items = store.search(query, count)?;

    if items.is_empty() {
        println!("No clipboard entries found matching '{query}'.");
        return Ok(());
    }

    match count {
        Some(n) => println!(
            "Found {} matches for '{}' (showing up to {}):",
            items.len(),
            query,
            n
        ),
        None => println!("Found {} matches for '{}':", items.len(), query),
    }

    for (i, item) in items.iter().enumerate() {
        println!("{}. {:?}", i + 1, item);
    }

    Ok(())
}

fn cmd_tui(key: &str) -> Result<()> {
    let store = open_store_with_key(key)?;
    let mut app = tui::App::new(store)?;
    tui::ui::run_tui(&mut app)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn cmd_setup() -> Result<()> {
    let first = rpassword::prompt_password("Set vault password: ")?;
    let second = rpassword::prompt_password("Confirm password: ")?;
    if first != second {
        eprintln!("Passwords do not match.");
        std::process::exit(1);
    }

    let exe = std::env::current_exe()?.with_file_name("clip-vault-daemon");
    println!("exe: {}", exe.display());

    let label = "com.clip-vault.daemon";

    // Build the LaunchAgent dictionary
    let plist = serde_json::json!({
        "Label": label,
        "ProgramArguments": [exe.to_string_lossy().into_owned()],
        "EnvironmentVariables": {
            "CLIP_VAULT_KEY": second,
            "CLIP_VAULT_FOREGROUND": "1",
        },
        "RunAtLoad": true,
        "KeepAlive": true,
        "StandardOutPath": "/tmp/clip-vault.out",
        "StandardErrorPath": "/tmp/clip-vault.err",
    });

    let plist_path: PathBuf = dirs::home_dir()
        .unwrap()
        .join("Library/LaunchAgents")
        .join(format!("{label}.plist"));
    std::fs::create_dir_all(plist_path.parent().unwrap())?;
    plist::to_file_xml(&plist_path, &plist)
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;

    // load & start
    let service = launchctl::Service::builder()
        .plist_path(plist_path.to_string_lossy().into_owned())
        .name(label)
        .build();
    service.start()?; // launchctl start

    println!("LaunchAgent installed & started ✅");
    Ok(())
}

fn open_store_with_key(key: &str) -> Result<SqliteVault> {
    let path = clip_vault_core::default_db_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    match SqliteVault::open(path, key) {
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

fn cmd_stop() -> Result<()> {
    let label = "com.clip-vault.daemon";
    let plist_path: PathBuf = dirs::home_dir()
        .unwrap()
        .join("Library/LaunchAgents")
        .join(format!("{label}.plist"));
    let service = launchctl::Service::builder()
        .plist_path(plist_path.to_string_lossy().into_owned())
        .name(label)
        .build();
    service.stop()?;
    Ok(())
}
