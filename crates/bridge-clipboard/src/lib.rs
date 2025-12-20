//! Universal clipboard sync with history for Code Bridge
//!
//! This crate provides cross-platform clipboard functionality with:
//! - Text, image, and rich content sync
//! - Clipboard history with search
//! - P2P sync across devices
//! - Sensitive content detection

pub mod platform;
pub mod sync;
pub mod history;
pub mod detection;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard access failed: {0}")]
    AccessFailed(String),

    #[error("Content type not supported: {0}")]
    UnsupportedType(String),

    #[error("Sync failed: {0}")]
    SyncFailed(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ClipboardError>;

/// Content types supported by clipboard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    /// Plain text
    Text,
    /// Rich text (HTML)
    RichText,
    /// Image data
    Image,
    /// File references
    Files,
    /// Code snippet with language
    Code { language: String },
    /// URL with metadata
    Url,
}

/// A clipboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: String,
    pub content_type: ContentType,
    pub data: Vec<u8>,
    pub preview: Option<String>,
    pub source_device: String,
    pub timestamp: DateTime<Utc>,
    pub pinned: bool,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub app_source: Option<String>,
}

impl ClipboardEntry {
    pub fn new_text(text: String, source_device: String) -> Self {
        let preview = if text.len() > 100 {
            Some(format!("{}...", &text[..100]))
        } else {
            Some(text.clone())
        };

        Self {
            id: Uuid::new_v4().to_string(),
            content_type: ContentType::Text,
            data: text.into_bytes(),
            preview,
            source_device,
            timestamp: Utc::now(),
            pinned: false,
            favorite: false,
            tags: Vec::new(),
            app_source: None,
        }
    }

    pub fn new_image(data: Vec<u8>, source_device: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content_type: ContentType::Image,
            data,
            preview: Some("[Image]".to_string()),
            source_device,
            timestamp: Utc::now(),
            pinned: false,
            favorite: false,
            tags: Vec::new(),
            app_source: None,
        }
    }

    pub fn new_code(code: String, language: String, source_device: String) -> Self {
        let preview = if code.len() > 100 {
            Some(format!("{}...", &code[..100]))
        } else {
            Some(code.clone())
        };

        Self {
            id: Uuid::new_v4().to_string(),
            content_type: ContentType::Code { language },
            data: code.into_bytes(),
            preview,
            source_device,
            timestamp: Utc::now(),
            pinned: false,
            favorite: false,
            tags: Vec::new(),
            app_source: None,
        }
    }

    pub fn new_url(url: String, source_device: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content_type: ContentType::Url,
            data: url.clone().into_bytes(),
            preview: Some(url),
            source_device,
            timestamp: Utc::now(),
            pinned: false,
            favorite: false,
            tags: Vec::new(),
            app_source: None,
        }
    }

    /// Get text content if available
    pub fn as_text(&self) -> Option<String> {
        match &self.content_type {
            ContentType::Text | ContentType::RichText | ContentType::Code { .. } | ContentType::Url => {
                String::from_utf8(self.data.clone()).ok()
            }
            _ => None,
        }
    }

    /// Get image data if available
    pub fn as_image(&self) -> Option<&[u8]> {
        match &self.content_type {
            ContentType::Image => Some(&self.data),
            _ => None,
        }
    }

    /// Check if content might be sensitive
    pub fn is_potentially_sensitive(&self) -> bool {
        detection::is_sensitive(&self.data, &self.content_type)
    }
}

/// Clipboard manager for access and sync
pub struct ClipboardManager {
    device_id: String,
    history: history::ClipboardHistory,
    last_content_hash: Option<String>,
}

impl ClipboardManager {
    pub fn new(device_id: String, storage_path: PathBuf) -> Result<Self> {
        let history = history::ClipboardHistory::new(storage_path)?;

        Ok(Self {
            device_id,
            history,
            last_content_hash: None,
        })
    }

    /// Get current clipboard content
    pub fn get_current(&self) -> Result<Option<ClipboardEntry>> {
        platform::get_clipboard_content(&self.device_id)
    }

    /// Set clipboard content
    pub fn set_content(&mut self, entry: &ClipboardEntry) -> Result<()> {
        platform::set_clipboard_content(entry)?;
        self.history.add(entry.clone())?;
        Ok(())
    }

    /// Copy text to clipboard
    pub fn copy_text(&mut self, text: String) -> Result<ClipboardEntry> {
        let entry = ClipboardEntry::new_text(text, self.device_id.clone());

        if !entry.is_potentially_sensitive() {
            platform::set_clipboard_content(&entry)?;
            self.history.add(entry.clone())?;
        }

        Ok(entry)
    }

    /// Copy image to clipboard
    pub fn copy_image(&mut self, data: Vec<u8>) -> Result<ClipboardEntry> {
        let entry = ClipboardEntry::new_image(data, self.device_id.clone());
        platform::set_clipboard_content(&entry)?;
        self.history.add(entry.clone())?;
        Ok(entry)
    }

    /// Get clipboard history
    pub fn get_history(&self, limit: usize) -> Result<Vec<ClipboardEntry>> {
        self.history.get_recent(limit)
    }

    /// Search clipboard history
    pub fn search(&self, query: &str) -> Result<Vec<ClipboardEntry>> {
        self.history.search(query)
    }

    /// Get favorites
    pub fn get_favorites(&self) -> Result<Vec<ClipboardEntry>> {
        self.history.get_favorites()
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&mut self, entry_id: &str) -> Result<()> {
        self.history.toggle_favorite(entry_id)
    }

    /// Pin/unpin entry
    pub fn toggle_pin(&mut self, entry_id: &str) -> Result<()> {
        self.history.toggle_pin(entry_id)
    }

    /// Delete entry from history
    pub fn delete(&mut self, entry_id: &str) -> Result<()> {
        self.history.delete(entry_id)
    }

    /// Clear history
    pub fn clear_history(&mut self) -> Result<()> {
        self.history.clear()
    }

    /// Watch clipboard for changes
    pub async fn watch<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(ClipboardEntry) + Send + 'static,
    {
        use std::time::Duration;
        use tokio::time::interval;

        let mut ticker = interval(Duration::from_millis(500));

        loop {
            ticker.tick().await;

            if let Ok(Some(entry)) = self.get_current() {
                let hash = blake3::hash(&entry.data).to_hex().to_string();

                if self.last_content_hash.as_ref() != Some(&hash) {
                    self.last_content_hash = Some(hash);

                    if !entry.is_potentially_sensitive() {
                        self.history.add(entry.clone()).ok();
                        callback(entry);
                    }
                }
            }
        }
    }
}

/// Clipboard sync message for P2P
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardSyncMessage {
    pub entry: ClipboardEntry,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncAction {
    Add,
    Delete,
    ToggleFavorite,
    TogglePin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_entry() {
        let entry = ClipboardEntry::new_text("Hello, World!".to_string(), "device1".to_string());

        assert_eq!(entry.content_type, ContentType::Text);
        assert_eq!(entry.as_text(), Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_code_entry() {
        let entry = ClipboardEntry::new_code(
            "fn main() {}".to_string(),
            "rust".to_string(),
            "device1".to_string(),
        );

        assert!(matches!(entry.content_type, ContentType::Code { .. }));
    }

    #[test]
    fn test_url_entry() {
        let entry = ClipboardEntry::new_url(
            "https://example.com".to_string(),
            "device1".to_string(),
        );

        assert_eq!(entry.content_type, ContentType::Url);
    }
}
