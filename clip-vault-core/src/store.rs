use crate::{ClipboardItem, ClipboardItemWithTimestamp, Result};

pub trait Vault {
    fn insert(&self, hash: [u8; 32], item: &ClipboardItem) -> Result<()>;
    fn latest(&self) -> Result<Option<ClipboardItem>>;
    fn list(
        &self,
        limit: Option<usize>,
        after_timestamp: Option<u64>,
    ) -> Result<Vec<ClipboardItemWithTimestamp>>;
    fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        after_timestamp: Option<u64>,
    ) -> Result<Vec<ClipboardItemWithTimestamp>>;
    fn update(&self, old_hash: [u8; 32], new_item: &ClipboardItem) -> Result<()>;
    fn delete(&self, hash: [u8; 32]) -> Result<()>;

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
        conn.pragma_update(None, "journal_mode", "WAL")?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS items (
                id      INTEGER PRIMARY KEY,
                hash    BLOB    UNIQUE NOT NULL,
                mime    TEXT    NOT NULL,
                text    TEXT,
                data    BLOB    NOT NULL,
                ts      INTEGER NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_mime_text ON items (mime, text);
            CREATE INDEX IF NOT EXISTS idx_ts ON items (ts);
            ",
        )?;

        Ok(Self { conn })
    }
}

unsafe impl Send for SqliteVault {}
unsafe impl Sync for SqliteVault {}

impl Vault for SqliteVault {
    fn insert(&self, hash: [u8; 32], item: &ClipboardItem) -> Result<()> {
        let timestamp = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();

        let (text, mime) = item.clone().into_parts();
        if mime == "image/png" {
            self.conn.execute(
                "INSERT OR IGNORE INTO items (hash, mime, data, ts) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(hash) DO UPDATE SET ts = ?4;",
                params![&hash[..], mime, bincode::encode_to_vec(item, bincode::config::standard())?, timestamp],
            )?;
        } else {
            self.conn.execute(
                "INSERT OR IGNORE INTO items (hash, mime, text, data, ts) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(hash) DO UPDATE SET ts = ?5;",
                params![&hash[..], mime, text, bincode::encode_to_vec(item, bincode::config::standard())?, timestamp],
            )?;
        }

        Ok(())
    }

    fn latest(&self) -> Result<Option<ClipboardItem>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM items ORDER BY ts DESC LIMIT 1;")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let blob: Vec<u8> = row.get(0)?;
            let (item, _): (ClipboardItem, usize) =
                bincode::decode_from_slice(&blob, bincode::config::standard())?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    fn list(
        &self,
        limit: Option<usize>,
        after_timestamp: Option<u64>,
    ) -> Result<Vec<ClipboardItemWithTimestamp>> {
        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) =
            match (limit, after_timestamp) {
                (Some(n), Some(ts)) => (
                    format!("SELECT data, ts FROM items WHERE ts < ? ORDER BY ts DESC LIMIT {n}"),
                    vec![Box::new(ts)],
                ),
                (Some(n), None) => (
                    format!("SELECT data, ts FROM items ORDER BY ts DESC LIMIT {n}"),
                    vec![],
                ),
                (None, Some(ts)) => (
                    "SELECT data, ts FROM items WHERE ts < ? ORDER BY ts DESC".to_string(),
                    vec![Box::new(ts)],
                ),
                (None, None) => (
                    "SELECT data, ts FROM items ORDER BY ts DESC".to_string(),
                    vec![],
                ),
            };

        let mut stmt = self.conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(std::convert::AsRef::as_ref).collect();
        let rows = stmt.query_map(&param_refs[..], |row| {
            let blob: Vec<u8> = row.get(0)?;
            let timestamp: u64 = row.get(1)?;
            let (item, _): (ClipboardItem, usize) =
                bincode::decode_from_slice(&blob, bincode::config::standard()).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })?;
            Ok(ClipboardItemWithTimestamp { item, timestamp })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        after_timestamp: Option<u64>,
    ) -> Result<Vec<ClipboardItemWithTimestamp>> {
        // Add wildcards for LIKE pattern matching
        let like_pattern = format!("%{query}%");

        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match (limit, after_timestamp)
        {
            (Some(n), Some(ts)) => (
                format!(
                    "SELECT data, ts FROM items 
                    WHERE text LIKE ? AND ts < ? AND mime != 'image/png'
                    ORDER BY ts DESC LIMIT {n}"
                ),
                vec![Box::new(like_pattern), Box::new(ts)],
            ),
            (Some(n), None) => (
                format!(
                    "SELECT data, ts FROM items 
                    WHERE text LIKE ? AND mime != 'image/png'
                    ORDER BY ts DESC LIMIT {n}"
                ),
                vec![Box::new(like_pattern)],
            ),
            (None, Some(ts)) => (
                "SELECT data, ts FROM items 
                WHERE text LIKE ? AND ts < ? AND mime != 'image/png'
                ORDER BY ts DESC"
                    .to_string(),
                vec![Box::new(like_pattern), Box::new(ts)],
            ),
            (None, None) => (
                "SELECT data, ts FROM items 
                WHERE text LIKE ? AND mime != 'image/png'
                ORDER BY ts DESC"
                    .to_string(),
                vec![Box::new(like_pattern)],
            ),
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(std::convert::AsRef::as_ref).collect();
        let rows = stmt.query_map(&param_refs[..], |row| {
            let blob: Vec<u8> = row.get(0)?;
            let timestamp: u64 = row.get(1)?;
            let (item, _): (ClipboardItem, usize) =
                bincode::decode_from_slice(&blob, bincode::config::standard()).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })?;
            Ok(ClipboardItemWithTimestamp { item, timestamp })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    fn len(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM items;", [], |row| row.get(0))?;
        Ok(usize::try_from(count).unwrap())
    }

    fn update(&self, old_hash: [u8; 32], new_item: &ClipboardItem) -> Result<()> {
        let new_hash = new_item.hash();
        let (text, mime) = new_item.clone().into_parts();
        let timestamp = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();

        self.conn.execute(
            "UPDATE items SET hash = ?1, mime = ?2, text = ?3, data = ?4, ts = ?5 WHERE hash = ?6;",
            params![
                &new_hash[..],
                mime,
                text,
                bincode::encode_to_vec(new_item, bincode::config::standard())?,
                timestamp,
                &old_hash[..]
            ],
        )?;
        Ok(())
    }

    fn delete(&self, hash: [u8; 32]) -> Result<()> {
        self.conn
            .execute("DELETE FROM items WHERE hash = ?1;", params![&hash[..]])?;
        Ok(())
    }
}
