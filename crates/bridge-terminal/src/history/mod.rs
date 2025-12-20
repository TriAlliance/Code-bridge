//! Command history storage and sync

use crate::{HistoryEntry, Result, TerminalError};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Command history manager
pub struct CommandHistory {
    conn: Connection,
}

impl CommandHistory {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        let db_path = storage_path.join("command_history.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS command_history (
                id TEXT PRIMARY KEY,
                command TEXT NOT NULL,
                working_dir TEXT NOT NULL,
                exit_code INTEGER,
                duration_ms INTEGER,
                timestamp TEXT NOT NULL,
                source_device TEXT NOT NULL,
                shell TEXT NOT NULL,
                session_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_timestamp ON command_history(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_command ON command_history(command);

            CREATE VIRTUAL TABLE IF NOT EXISTS history_fts USING fts5(
                command,
                content=command_history,
                content_rowid=rowid
            );
            "#,
        )
        .map_err(|e| TerminalError::Database(e.to_string()))?;

        Ok(Self { conn })
    }

    /// Add entry to history
    pub fn add(&mut self, entry: HistoryEntry) -> Result<()> {
        let duration_ms = entry.duration.map(|d| d.as_millis() as i64);

        self.conn
            .execute(
                r#"
                INSERT INTO command_history
                (id, command, working_dir, exit_code, duration_ms, timestamp, source_device, shell, session_id)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    entry.id,
                    entry.command,
                    entry.working_dir.to_string_lossy(),
                    entry.exit_code,
                    duration_ms,
                    entry.timestamp.to_rfc3339(),
                    entry.source_device,
                    entry.shell,
                    entry.session_id,
                ],
            )
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        // Update FTS
        self.conn
            .execute(
                "INSERT INTO history_fts (rowid, command) VALUES (last_insert_rowid(), ?1)",
                params![entry.command],
            )
            .ok();

        Ok(())
    }

    /// Get recent entries
    pub fn get_recent(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, command, working_dir, exit_code, duration_ms, timestamp, source_device, shell, session_id
                FROM command_history
                ORDER BY timestamp DESC
                LIMIT ?1
                "#,
            )
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(params![limit], |row| self.row_to_entry(row))
            .map_err(|e| TerminalError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Search history
    pub fn search(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        // First try FTS search
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT h.id, h.command, h.working_dir, h.exit_code, h.duration_ms,
                       h.timestamp, h.source_device, h.shell, h.session_id
                FROM command_history h
                JOIN history_fts f ON h.rowid = f.rowid
                WHERE history_fts MATCH ?1
                ORDER BY h.timestamp DESC
                LIMIT 100
                "#,
            )
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        let entries: Vec<HistoryEntry> = stmt
            .query_map(params![format!("{}*", query)], |row| self.row_to_entry(row))
            .map_err(|e| TerminalError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Fallback to LIKE search if FTS returns nothing
        if entries.is_empty() {
            let mut stmt = self
                .conn
                .prepare(
                    r#"
                    SELECT id, command, working_dir, exit_code, duration_ms,
                           timestamp, source_device, shell, session_id
                    FROM command_history
                    WHERE command LIKE ?1
                    ORDER BY timestamp DESC
                    LIMIT 100
                    "#,
                )
                .map_err(|e| TerminalError::Database(e.to_string()))?;

            return stmt
                .query_map(params![format!("%{}%", query)], |row| self.row_to_entry(row))
                .map_err(|e| TerminalError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>()
                .pipe(Ok);
        }

        Ok(entries)
    }

    /// Get entries by directory
    pub fn get_by_directory(&self, dir: &PathBuf) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT id, command, working_dir, exit_code, duration_ms,
                       timestamp, source_device, shell, session_id
                FROM command_history
                WHERE working_dir = ?1
                ORDER BY timestamp DESC
                LIMIT 100
                "#,
            )
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(params![dir.to_string_lossy()], |row| self.row_to_entry(row))
            .map_err(|e| TerminalError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Get most used commands
    pub fn get_most_used(&self, limit: usize) -> Result<Vec<(String, usize)>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT command, COUNT(*) as count
                FROM command_history
                GROUP BY command
                ORDER BY count DESC
                LIMIT ?1
                "#,
            )
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        let entries = stmt
            .query_map(params![limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
            })
            .map_err(|e| TerminalError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Delete entry
    pub fn delete(&mut self, entry_id: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM command_history WHERE id = ?1",
                params![entry_id],
            )
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        Ok(())
    }

    /// Clear all history
    pub fn clear(&mut self) -> Result<()> {
        self.conn
            .execute("DELETE FROM command_history", [])
            .map_err(|e| TerminalError::Database(e.to_string()))?;

        self.conn
            .execute("DELETE FROM history_fts", [])
            .ok();

        Ok(())
    }

    /// Import from shell history file
    pub fn import_from_file(&mut self, path: &PathBuf, shell: &str) -> Result<usize> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| TerminalError::History(e.to_string()))?;

        let mut count = 0;

        for line in content.lines() {
            // Skip empty lines and comments
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle different history formats
            let command = if shell.contains("zsh") {
                // zsh format: : timestamp:0;command
                if let Some(idx) = line.find(';') {
                    line[idx + 1..].to_string()
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            };

            let entry = HistoryEntry::new(
                command,
                std::env::current_dir().unwrap_or_default(),
                shell,
            );

            self.add(entry)?;
            count += 1;
        }

        Ok(count)
    }

    fn row_to_entry(&self, row: &rusqlite::Row) -> rusqlite::Result<HistoryEntry> {
        let timestamp_str: String = row.get(5)?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|t| t.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let duration = row.get::<_, Option<i64>>(4)?
            .map(|ms| std::time::Duration::from_millis(ms as u64));

        Ok(HistoryEntry {
            id: row.get(0)?,
            command: row.get(1)?,
            working_dir: PathBuf::from(row.get::<_, String>(2)?),
            exit_code: row.get(3)?,
            duration,
            timestamp,
            source_device: row.get(6)?,
            shell: row.get(7)?,
            session_id: row.get(8)?,
        })
    }
}

// Helper trait for pipe syntax
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_and_get() {
        let temp = tempdir().unwrap();
        let mut history = CommandHistory::new(temp.path().to_path_buf()).unwrap();

        let entry = HistoryEntry::new(
            "ls -la".to_string(),
            PathBuf::from("/home/user"),
            "bash",
        );
        history.add(entry).unwrap();

        let recent = history.get_recent(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].command, "ls -la");
    }

    #[test]
    fn test_search() {
        let temp = tempdir().unwrap();
        let mut history = CommandHistory::new(temp.path().to_path_buf()).unwrap();

        history.add(HistoryEntry::new("git status".to_string(), PathBuf::from("/"), "bash")).unwrap();
        history.add(HistoryEntry::new("git commit".to_string(), PathBuf::from("/"), "bash")).unwrap();
        history.add(HistoryEntry::new("ls -la".to_string(), PathBuf::from("/"), "bash")).unwrap();

        let results = history.search("git").unwrap();
        assert_eq!(results.len(), 2);
    }
}
