# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building
```bash
# Check all workspace crates
cargo check --workspace

# Build all workspace crates  
cargo build --workspace

# Build for release
cargo build --workspace --release

# Build specific crate
cargo build -p clip-vault-core
cargo build -p clip-vault-cli  
cargo build -p clip-vault-daemon
```

### Running
```bash
# Run CLI (requires password setup first)
cargo run -p clip-vault-cli -- setup
cargo run -p clip-vault-cli -- latest
cargo run -p clip-vault-cli -- list
cargo run -p clip-vault-cli -- stop

# Run daemon directly (for testing)
CLIP_VAULT_KEY="test" cargo run -p clip-vault-daemon
```

### Testing
No test framework currently exists in this codebase. Tests should be added using standard Rust `#[cfg(test)]` modules with `cargo test`.

## Architecture

This is a Rust workspace with three crates:

### clip-vault-core
Core library containing:
- `ClipboardItem` enum (currently Text only, Image planned)
- `Vault` trait for storage abstraction  
- `SqliteVault` implementation using encrypted SQLite with SQLCipher
- `Error` type with conversions from io::Error, bincode::Error, rusqlite::Error
- Default database path: `{data_dir}/clip-vault/clip_vault.db`

### clip-vault-cli
Command-line interface with:
- `clap` CLI parsing with subcommands: `latest`, `list`, `setup`, `stop`
- Password management with caching (default 15min) in `{cache_dir}/clip-vault/session.json`
- macOS LaunchAgent setup via `setup` command
- Environment variable `CLIP_VAULT_KEY` bypasses password prompt

### clip-vault-daemon  
Background service that:
- Monitors clipboard using `arboard` crate
- Polls every 100ms for changes
- Stores new clipboard text in encrypted SQLite vault
- Uses SHA256 hashing for duplicate detection
- Runs as daemon on Unix (except macOS), uses LaunchAgent on macOS

## Key Implementation Details

- **Encryption**: Uses SQLCipher via rusqlite's "bundled-sqlcipher" feature
- **Duplicate Detection**: SHA256 hash of clipboard content prevents duplicates
- **Password Caching**: Session tokens with configurable expiration (CLI `--remember` flag)
- **Cross-platform**: Different daemon strategies for macOS vs Unix
- **Data Serialization**: Uses `bincode` for efficient binary serialization of clipboard items

## Environment Variables

- `CLIP_VAULT_KEY`: Vault password (bypasses interactive prompt)
- `CLIP_VAULT_FOREGROUND`: Prevents daemonization on Unix systems (for debugging)

## File Locations

- Database: `{data_dir}/clip-vault/clip_vault.db` 
- Session cache: `{cache_dir}/clip-vault/session.json`
- macOS LaunchAgent: `~/Library/LaunchAgents/com.clip-vault.daemon.plist`
- Daemon logs: `/tmp/clip-vault.out`, `/tmp/clip-vault.err`

## Development Best Practices

- Use yarn always
- Whenever making a big change always commit and push to ensure that the code is always safe