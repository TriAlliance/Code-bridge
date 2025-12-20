//! Clipboard sync across devices

use crate::{ClipboardEntry, ClipboardError, ClipboardSyncMessage, Result, SyncAction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Sync manager for clipboard content across P2P network
pub struct ClipboardSync {
    device_id: String,
    synced_hashes: HashSet<String>,
    sync_enabled: bool,
    sync_images: bool,
    max_sync_size: usize,
}

impl ClipboardSync {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            synced_hashes: HashSet::new(),
            sync_enabled: true,
            sync_images: true,
            max_sync_size: 10 * 1024 * 1024, // 10MB max
        }
    }

    /// Enable/disable sync
    pub fn set_enabled(&mut self, enabled: bool) {
        self.sync_enabled = enabled;
    }

    /// Enable/disable image sync
    pub fn set_sync_images(&mut self, enabled: bool) {
        self.sync_images = enabled;
    }

    /// Set maximum sync size
    pub fn set_max_size(&mut self, size: usize) {
        self.max_sync_size = size;
    }

    /// Check if entry should be synced
    pub fn should_sync(&self, entry: &ClipboardEntry) -> bool {
        if !self.sync_enabled {
            return false;
        }

        // Don't sync from same device
        if entry.source_device == self.device_id {
            return false;
        }

        // Check size limit
        if entry.data.len() > self.max_sync_size {
            return false;
        }

        // Check image sync setting
        if matches!(entry.content_type, crate::ContentType::Image) && !self.sync_images {
            return false;
        }

        // Don't sync sensitive content
        if entry.is_potentially_sensitive() {
            return false;
        }

        // Check if already synced
        let hash = blake3::hash(&entry.data).to_hex().to_string();
        if self.synced_hashes.contains(&hash) {
            return false;
        }

        true
    }

    /// Mark entry as synced
    pub fn mark_synced(&mut self, entry: &ClipboardEntry) {
        let hash = blake3::hash(&entry.data).to_hex().to_string();
        self.synced_hashes.insert(hash);

        // Keep cache bounded
        if self.synced_hashes.len() > 10000 {
            // Clear oldest half
            let to_remove: Vec<_> = self.synced_hashes.iter().take(5000).cloned().collect();
            for hash in to_remove {
                self.synced_hashes.remove(&hash);
            }
        }
    }

    /// Prepare entry for sync (compress if needed)
    pub fn prepare_for_sync(&self, entry: &ClipboardEntry) -> Result<ClipboardSyncMessage> {
        Ok(ClipboardSyncMessage {
            entry: entry.clone(),
            action: SyncAction::Add,
        })
    }

    /// Process incoming sync message
    pub fn process_incoming(&mut self, message: ClipboardSyncMessage) -> Result<Option<ClipboardEntry>> {
        match message.action {
            SyncAction::Add => {
                if self.should_sync(&message.entry) {
                    self.mark_synced(&message.entry);
                    Ok(Some(message.entry))
                } else {
                    Ok(None)
                }
            }
            SyncAction::Delete => {
                // Handle delete sync
                Ok(None)
            }
            SyncAction::ToggleFavorite | SyncAction::TogglePin => {
                // Handle state updates
                Ok(None)
            }
        }
    }
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enable clipboard sync
    pub enabled: bool,
    /// Sync images
    pub sync_images: bool,
    /// Maximum content size to sync (bytes)
    pub max_size: usize,
    /// Devices to sync with (empty = all)
    pub allowed_devices: Vec<String>,
    /// Content types to sync
    pub allowed_types: Vec<String>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sync_images: true,
            max_size: 10 * 1024 * 1024,
            allowed_devices: Vec::new(),
            allowed_types: vec![
                "text".to_string(),
                "code".to_string(),
                "url".to_string(),
                "image".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_sync() {
        let sync = ClipboardSync::new("device1".to_string());

        // Entry from another device should sync
        let entry = ClipboardEntry::new_text("Hello".to_string(), "device2".to_string());
        assert!(sync.should_sync(&entry));

        // Entry from same device should not sync
        let entry = ClipboardEntry::new_text("Hello".to_string(), "device1".to_string());
        assert!(!sync.should_sync(&entry));
    }

    #[test]
    fn test_mark_synced() {
        let mut sync = ClipboardSync::new("device1".to_string());
        let entry = ClipboardEntry::new_text("Hello".to_string(), "device2".to_string());

        assert!(sync.should_sync(&entry));
        sync.mark_synced(&entry);
        assert!(!sync.should_sync(&entry));
    }
}
