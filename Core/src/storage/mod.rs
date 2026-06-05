//! `storage` — local database, note management, and history persistence submodule.

use std::path::Path;
use rusqlite::{params, Connection};

use crate::RecordProvider;

/// Custom error type for storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// SQLite database error.
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// Invalid path or configuration error.
    #[error("Invalid path or configuration: {0}")]
    InvalidInput(String),
}

/// Standard SQLite storage engine.
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open the SQLite database at `db_path`. Creates the database and necessary schema
    /// if it does not exist.
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let conn = Connection::open(db_path).map_err(|e| StorageError::Database(e))?;

        // Initialize standard unified key-value schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS storage_entries (
                key        TEXT PRIMARY KEY,
                value      TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Retrieve the value associated with `key`.
    /// Returns `None` if the key does not exist.
    pub fn get(&self, key: &str) -> Option<String> {
        let mut stmt = match self.conn.prepare("SELECT value FROM storage_entries WHERE key = ?") {
            Ok(s) => s,
            Err(_) => return None,
        };

        stmt.query_row(params![key], |row| row.get::<_, String>(0)).ok()
    }

    /// Insert or update a key-value pair.
    /// This method does not return any status or boolean type on success.
    pub fn insert_or_update(&mut self, key: &str, value: &str) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO storage_entries (key, value) VALUES (?, ?)",
            params![key, value],
        );
    }

    /// Delete a key.
    /// This method does not return any status or boolean type on success.
    pub fn delete(&mut self, key: &str) {
        let _ = self.conn.execute(
            "DELETE FROM storage_entries WHERE key = ?",
            params![key],
        );
    }
}

impl RecordProvider for Storage {
    fn get(&self, key: &str) -> Option<String> {
        self.get(key)
    }
}
