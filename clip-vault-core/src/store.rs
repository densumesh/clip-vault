use crate::{ClipboardItem, Result};

pub trait Vault {
    fn insert(&self, hash: [u8; 32], item: &ClipboardItem) -> Result<()>;
    fn latest(&self) -> Result<Option<ClipboardItem>>;

    fn len(&self) -> Result<usize>;

    fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
}

use rusqlite::{params, Connection};

pub struct SqliteVault {
    conn: Connection,
}

impl SqliteVault {
    pub fn open<P: AsRef<std::path::Path>>(path: P, key: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", key)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS items (
                 hash BLOB PRIMARY KEY,
                 data BLOB NOT NULL,
                 ts   INTEGER NOT NULL DEFAULT (strftime('%s','now'))
             );",
            [],
        )?;
        Ok(Self { conn })
    }
}

impl Vault for SqliteVault {
    fn insert(&self, hash: [u8; 32], item: &ClipboardItem) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO items (hash, data) VALUES (?1, ?2);",
            params![&hash[..], bincode::serialize(item)?],
        )?;
        Ok(())
    }

    fn latest(&self) -> Result<Option<ClipboardItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM items ORDER BY ts DESC LIMIT 1;")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let blob: Vec<u8> = row.get(0)?;
            Ok(Some(bincode::deserialize(&blob)?))
        } else {
            Ok(None)
        }
    }

    fn len(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM items;", [], |row| row.get(0))?;
        Ok(usize::try_from(count).unwrap())
    }
}
