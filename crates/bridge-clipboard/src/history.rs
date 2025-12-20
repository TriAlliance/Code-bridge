//! Clipboard history storage and management

use crate::{ClipboardEntry, ClipboardError, ContentType, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Clipboard history manager with SQLite storage
pub struct ClipboardHistory {
    conn: Connection,
    max_entries: usize,
}

impl ClipboardHistory {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        let db_path = storage_path.join("clipboard_history.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        // Create tables
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_entries (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                data BLOB NOT NULL,
                preview TEXT,
                source_device TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                pinned INTEGER DEFAULT 0,
                favorite INTEGER DEFAULT 0,
                tags TEXT DEFAULT '[]',
                app_source TEXT,
                content_hash TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_timestamp ON clipboard_entries(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_content_hash ON clipboard_entries(content_hash);
            CREATE INDEX IF NOT EXISTS idx_favorite ON clipboard_entries(favorite);
            CREATE INDEX IF NOT EXISTS idx_pinned ON clipboard_entries(pinned);

            CREATE VIRTUAL TABLE IF NOT EXISTS clipboard_fts USING fts5(
                id,
                preview,
                content=clipboard_entries,
                content_rowid=rowid
            );
            "#,
        )
        .map_err(|e| ClipboardError::Database(e.to_string()))?;

        Ok(Self {
            conn,
            max_entries: 1000,
        })
    }

    /// Add entry to history
    pub fn add(&mut self, entry: ClipboardEntry) -> Result<()> {
        let content_hash = blake3::hash(&entry.data).to_hex().to_string();
        let content_type = serde_json::to_string(&entry.content_type)
            .map_err(|e| ClipboardError::Database(e.to_string()))?;
        let tags = serde_json::to_string(&entry.tags)
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        // Check for duplicate content
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM clipboard_entries WHERE content_hash = ?1",
                params![content_hash],
                |row| row.get(0),
            )
            .ok();

        if let Some(existing_id) = existing {
            // Update timestamp of existing entry
            self.conn
                .execute(
                    "UPDATE clipboard_entries SET timestamp = ?1 WHERE id = ?2",
                    params![entry.timestamp.to_rfc3339(), existing_id],
                )
                .map_err(|e| ClipboardError::Database(e.to_string()))?;
        } else {
            // Insert new entry
            self.conn
                .execute(
                    r#"
                    INSERT INTO clipboard_entries
                    (id, content_type, data, preview, source_device, timestamp, pinned, favorite, tags, app_source, content_hash)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                    "#,
                    params![
                        entry.id,
                        content_type,
                        entry.data,
                        entry.preview,
                        entry.source_device,
                        entry.timestamp.to_rfc3339(),
                        entry.pinned as i32,
                        entry.favorite as i32,
                        tags,
                        entry.app_source,
                        content_hash,
                    ],
                )
                .map_err(|e| ClipboardError::Database(e.to_string()))?;

            // Update FTS index
            if let Some(preview) = &entry.preview {
                self.conn
                    .execute(
                        "INSERT INTO clipboard_fts (id, preview) VALUES (?1, ?2)",
                        params![entry.id, preview],
                    )
                    .ok();
            }
        }

        // Cleanup old entries (keep pinned and favorited)
        self.cleanup()?;

        Ok(())
    }

    /// Get recent entries
    pub fn get_recent(&self, limit: usize) -> Result<Vec<ClipboardEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, content_type, data, preview, source_device, timestamp, pinned, favorite, tags, app_source
                FROM clipboard_entries
                ORDER BY pinned DESC, timestamp DESC
                LIMIT ?1
                "#,
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(params![limit], |row| {
                self.row_to_entry(row)
            })
            .map_err(|e| ClipboardError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Search entries
    pub fn search(&self, query: &str) -> Result<Vec<ClipboardEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT e.id, e.content_type, e.data, e.preview, e.source_device,
                       e.timestamp, e.pinned, e.favorite, e.tags, e.app_source
                FROM clipboard_entries e
                JOIN clipboard_fts f ON e.id = f.id
                WHERE clipboard_fts MATCH ?1
                ORDER BY e.timestamp DESC
                LIMIT 100
                "#,
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(params![query], |row| {
                self.row_to_entry(row)
            })
            .map_err(|e| ClipboardError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Get favorites
    pub fn get_favorites(&self) -> Result<Vec<ClipboardEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, content_type, data, preview, source_device, timestamp, pinned, favorite, tags, app_source
                FROM clipboard_entries
                WHERE favorite = 1
                ORDER BY timestamp DESC
                "#,
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        let entries = stmt
            .query_map([], |row| {
                self.row_to_entry(row)
            })
            .map_err(|e| ClipboardError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Toggle favorite
    pub fn toggle_favorite(&mut self, entry_id: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE clipboard_entries SET favorite = NOT favorite WHERE id = ?1",
                params![entry_id],
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;
        Ok(())
    }

    /// Toggle pin
    pub fn toggle_pin(&mut self, entry_id: &str) -> Result<()> {
        self.conn
            .execute(
                "UPDATE clipboard_entries SET pinned = NOT pinned WHERE id = ?1",
                params![entry_id],
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete entry
    pub fn delete(&mut self, entry_id: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM clipboard_entries WHERE id = ?1",
                params![entry_id],
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        self.conn
            .execute(
                "DELETE FROM clipboard_fts WHERE id = ?1",
                params![entry_id],
            )
            .ok();

        Ok(())
    }

    /// Clear all history (except pinned)
    pub fn clear(&mut self) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM clipboard_entries WHERE pinned = 0",
                [],
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;
        Ok(())
    }

    /// Cleanup old entries
    fn cleanup(&mut self) -> Result<()> {
        // Count total entries
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM clipboard_entries WHERE pinned = 0 AND favorite = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| ClipboardError::Database(e.to_string()))?;

        if count > self.max_entries as i64 {
            let to_delete = count - self.max_entries as i64;

            self.conn
                .execute(
                    r#"
                    DELETE FROM clipboard_entries
                    WHERE id IN (
                        SELECT id FROM clipboard_entries
                        WHERE pinned = 0 AND favorite = 0
                        ORDER BY timestamp ASC
                        LIMIT ?1
                    )
                    "#,
                    params![to_delete],
                )
                .map_err(|e| ClipboardError::Database(e.to_string()))?;
        }

        Ok(())
    }

    fn row_to_entry(&self, row: &rusqlite::Row) -> rusqlite::Result<ClipboardEntry> {
        let content_type_str: String = row.get(1)?;
        let content_type: ContentType = serde_json::from_str(&content_type_str)
            .unwrap_or(ContentType::Text);

        let tags_str: String = row.get(8)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();

        let timestamp_str: String = row.get(5)?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|t| t.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(ClipboardEntry {
            id: row.get(0)?,
            content_type,
            data: row.get(2)?,
            preview: row.get(3)?,
            source_device: row.get(4)?,
            timestamp,
            pinned: row.get::<_, i32>(6)? != 0,
            favorite: row.get::<_, i32>(7)? != 0,
            tags,
            app_source: row.get(9)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_history_add_and_get() {
        let temp = tempdir().unwrap();
        let mut history = ClipboardHistory::new(temp.path().to_path_buf()).unwrap();

        let entry = ClipboardEntry::new_text("Test content".to_string(), "device1".to_string());
        history.add(entry.clone()).unwrap();

        let recent = history.get_recent(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, entry.id);
    }
}
