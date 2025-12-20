//! Content-addressed storage system
//!
//! Files are stored by their BLAKE3 hash, enabling:
//! - Automatic deduplication
//! - Integrity verification
//! - Efficient delta sync

use crate::{config::Config, BridgeError, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Content hash type (BLAKE3)
pub type ContentHash = String;

/// File entry in the content store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Content hash (BLAKE3)
    pub hash: ContentHash,

    /// Original file path (relative to project)
    pub path: PathBuf,

    /// File size in bytes
    pub size: u64,

    /// MIME type
    pub mime_type: Option<String>,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Modification timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,

    /// Is directory
    pub is_directory: bool,

    /// Chunk hashes for large files
    pub chunks: Vec<ContentHash>,
}

/// Content-addressed storage
pub struct ContentStore {
    /// Root directory for object storage
    objects_dir: PathBuf,

    /// SQLite database for metadata
    db: Connection,

    /// Configuration
    config: Config,

    /// In-memory cache of recent files
    cache: HashMap<ContentHash, Vec<u8>>,

    /// Maximum cache size
    max_cache_size: usize,

    /// Current cache size
    current_cache_size: usize,
}

impl ContentStore {
    /// Create a new content store
    pub fn new(config: &Config) -> Result<Self> {
        let objects_dir = config.content_store_path();
        fs::create_dir_all(&objects_dir)?;

        let db_path = config.database_path();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let db = Connection::open(&db_path)?;

        // Initialize database schema
        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                hash TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                size INTEGER NOT NULL,
                mime_type TEXT,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                is_directory INTEGER NOT NULL DEFAULT 0,
                chunks TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sync_state (
                project_id TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                synced_at TEXT NOT NULL,
                peer_id TEXT,
                PRIMARY KEY (project_id, file_hash)
            );
            "#,
        )?;

        info!("Content store initialized at {:?}", objects_dir);

        Ok(Self {
            objects_dir,
            db,
            config: config.clone(),
            cache: HashMap::new(),
            max_cache_size: 100 * 1024 * 1024, // 100 MB cache
            current_cache_size: 0,
        })
    }

    /// Calculate BLAKE3 hash of data
    pub fn hash(data: &[u8]) -> ContentHash {
        let hash = blake3::hash(data);
        hash.to_hex().to_string()
    }

    /// Calculate hash of a file
    pub fn hash_file(path: &Path) -> Result<ContentHash> {
        let mut file = fs::File::open(path)?;
        let mut hasher = blake3::Hasher::new();

        let mut buffer = [0u8; 65536]; // 64 KB buffer
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Store content and return its hash
    pub fn store(&mut self, data: &[u8]) -> Result<ContentHash> {
        let hash = Self::hash(data);
        let object_path = self.object_path(&hash);

        if !object_path.exists() {
            // Create parent directories
            if let Some(parent) = object_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write content
            let mut file = fs::File::create(&object_path)?;
            file.write_all(data)?;

            debug!("Stored object: {} ({} bytes)", hash, data.len());
        } else {
            debug!("Object already exists: {}", hash);
        }

        // Update cache
        self.cache_put(&hash, data.to_vec());

        Ok(hash)
    }

    /// Store a file from disk
    pub fn store_file(&mut self, path: &Path, relative_path: &Path) -> Result<FileEntry> {
        let metadata = fs::metadata(path)?;
        let data = fs::read(path)?;
        let hash = self.store(&data)?;

        let entry = FileEntry {
            hash: hash.clone(),
            path: relative_path.to_path_buf(),
            size: metadata.len(),
            mime_type: Self::guess_mime_type(path),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            is_directory: false,
            chunks: vec![],
        };

        // Store metadata in database
        self.db.execute(
            "INSERT OR REPLACE INTO files (hash, path, size, mime_type, created_at, modified_at, is_directory, chunks)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.hash,
                entry.path.to_string_lossy().to_string(),
                entry.size as i64,
                entry.mime_type,
                entry.created_at.to_rfc3339(),
                entry.modified_at.to_rfc3339(),
                entry.is_directory as i32,
                serde_json::to_string(&entry.chunks).unwrap_or_default(),
            ],
        )?;

        info!("Stored file: {:?} -> {}", relative_path, hash);

        Ok(entry)
    }

    /// Retrieve content by hash
    pub fn get(&mut self, hash: &str) -> Result<Vec<u8>> {
        // Check cache first
        if let Some(data) = self.cache.get(hash) {
            return Ok(data.clone());
        }

        // Read from disk
        let object_path = self.object_path(hash);
        if !object_path.exists() {
            return Err(BridgeError::Storage(format!("Object not found: {}", hash)));
        }

        let data = fs::read(&object_path)?;

        // Update cache
        self.cache_put(hash, data.clone());

        Ok(data)
    }

    /// Check if content exists
    pub fn exists(&self, hash: &str) -> bool {
        self.object_path(hash).exists()
    }

    /// Get file entry metadata
    pub fn get_entry(&self, hash: &str) -> Result<Option<FileEntry>> {
        let mut stmt = self.db.prepare(
            "SELECT hash, path, size, mime_type, created_at, modified_at, is_directory, chunks
             FROM files WHERE hash = ?1",
        )?;

        let result = stmt.query_row(params![hash], |row| {
            let chunks_str: String = row.get(7)?;
            let chunks: Vec<String> = serde_json::from_str(&chunks_str).unwrap_or_default();

            Ok(FileEntry {
                hash: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                size: row.get::<_, i64>(2)? as u64,
                mime_type: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                is_directory: row.get::<_, i32>(6)? != 0,
                chunks,
            })
        });

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all stored files
    pub fn list_files(&self) -> Result<Vec<FileEntry>> {
        let mut stmt = self.db.prepare(
            "SELECT hash, path, size, mime_type, created_at, modified_at, is_directory, chunks
             FROM files ORDER BY path",
        )?;

        let entries = stmt
            .query_map([], |row| {
                let chunks_str: String = row.get(7)?;
                let chunks: Vec<String> = serde_json::from_str(&chunks_str).unwrap_or_default();

                Ok(FileEntry {
                    hash: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    size: row.get::<_, i64>(2)? as u64,
                    mime_type: row.get(3)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    modified_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    is_directory: row.get::<_, i32>(6)? != 0,
                    chunks,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Delete content by hash
    pub fn delete(&mut self, hash: &str) -> Result<()> {
        let object_path = self.object_path(hash);
        if object_path.exists() {
            fs::remove_file(&object_path)?;
        }

        self.db.execute("DELETE FROM files WHERE hash = ?1", params![hash])?;
        self.cache.remove(hash);

        debug!("Deleted object: {}", hash);
        Ok(())
    }

    /// Get the object storage path for a hash
    fn object_path(&self, hash: &str) -> PathBuf {
        // Use first 2 characters as subdirectory for better filesystem performance
        let prefix = &hash[..2.min(hash.len())];
        self.objects_dir.join(prefix).join(hash)
    }

    /// Add to cache with LRU eviction
    fn cache_put(&mut self, hash: &str, data: Vec<u8>) {
        let size = data.len();

        // Evict if necessary
        while self.current_cache_size + size > self.max_cache_size && !self.cache.is_empty() {
            if let Some(key) = self.cache.keys().next().cloned() {
                if let Some(removed) = self.cache.remove(&key) {
                    self.current_cache_size -= removed.len();
                }
            }
        }

        // Add to cache
        if size <= self.max_cache_size {
            self.cache.insert(hash.to_string(), data);
            self.current_cache_size += size;
        }
    }

    /// Guess MIME type from file extension
    fn guess_mime_type(path: &Path) -> Option<String> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        let mime = match ext.as_str() {
            // Images
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "avif" => "image/avif",
            "svg" => "image/svg+xml",

            // Code
            "rs" => "text/x-rust",
            "js" => "text/javascript",
            "ts" => "text/typescript",
            "py" => "text/x-python",
            "swift" => "text/x-swift",
            "kt" => "text/x-kotlin",
            "java" => "text/x-java",
            "go" => "text/x-go",
            "c" | "h" => "text/x-c",
            "cpp" | "hpp" => "text/x-c++",

            // Web
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "json" => "application/json",
            "xml" => "application/xml",

            // Documents
            "md" => "text/markdown",
            "txt" => "text/plain",
            "pdf" => "application/pdf",

            _ => return None,
        };
        Some(mime.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hash() {
        let data = b"hello world";
        let hash = ContentStore::hash(data);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // BLAKE3 produces 256-bit hash
    }

    #[test]
    fn test_store_and_retrieve() {
        let dir = tempdir().unwrap();
        let mut config = Config::default();
        config.data_dir = dir.path().to_path_buf();

        let mut store = ContentStore::new(&config).unwrap();

        let data = b"test content";
        let hash = store.store(data).unwrap();

        let retrieved = store.get(&hash).unwrap();
        assert_eq!(retrieved, data);
    }
}
