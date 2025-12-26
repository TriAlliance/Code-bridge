//! Sync engine for Code Bridge
//!
//! Handles bidirectional synchronization between peers
//! using CRDTs for conflict-free merging.

use crate::{storage::ContentHash, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, info};

/// Sync state for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Project ID
    pub project_id: String,

    /// Map of file paths to their current hashes
    pub files: HashMap<PathBuf, FileState>,

    /// Vector clock for ordering
    pub vector_clock: VectorClock,

    /// Last sync timestamp
    pub last_sync: chrono::DateTime<chrono::Utc>,
}

/// State of a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// Content hash
    pub hash: ContentHash,

    /// Modification timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,

    /// Vector clock version
    pub version: u64,

    /// Is deleted
    pub deleted: bool,
}

/// Vector clock for distributed ordering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorClock {
    /// Map of device ID to logical timestamp
    clocks: HashMap<String, u64>,
}

impl VectorClock {
    /// Create a new vector clock
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment the clock for a device
    pub fn increment(&mut self, device_id: &str) {
        let counter = self.clocks.entry(device_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Get the clock value for a device
    pub fn get(&self, device_id: &str) -> u64 {
        self.clocks.get(device_id).copied().unwrap_or(0)
    }

    /// Merge two vector clocks (take maximum of each)
    pub fn merge(&mut self, other: &VectorClock) {
        for (device_id, &clock) in &other.clocks {
            let current = self.clocks.entry(device_id.clone()).or_insert(0);
            *current = (*current).max(clock);
        }
    }

    /// Check if this clock is concurrent with another
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Check if this clock happens before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut dominated = false;

        // All our clocks must be <= other's clocks
        for (device_id, &clock) in &self.clocks {
            let other_clock = other.get(device_id);
            if clock > other_clock {
                return false;
            }
            if clock < other_clock {
                dominated = true;
            }
        }

        // Other might have devices we don't have
        for (device_id, &clock) in &other.clocks {
            if !self.clocks.contains_key(device_id) && clock > 0 {
                dominated = true;
            }
        }

        dominated
    }
}

/// Diff between two sync states
#[derive(Debug, Clone)]
pub struct SyncDiff {
    /// Files that need to be sent to the peer
    pub to_send: Vec<PathBuf>,

    /// Files that need to be received from the peer
    pub to_receive: Vec<PathBuf>,

    /// Files with conflicts (concurrent modifications)
    pub conflicts: Vec<PathBuf>,

    /// Files to delete locally
    pub to_delete: Vec<PathBuf>,
}

/// Sync engine
pub struct SyncEngine {
    /// Local device ID
    device_id: String,

    /// Current sync state per project
    states: HashMap<String, SyncState>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            states: HashMap::new(),
        }
    }

    /// Set the device ID
    pub fn set_device_id(&mut self, device_id: String) {
        self.device_id = device_id;
    }

    /// Initialize sync state for a project
    pub fn init_project(&mut self, project_id: &str) -> &SyncState {
        self.states.entry(project_id.to_string()).or_insert_with(|| {
            SyncState {
                project_id: project_id.to_string(),
                files: HashMap::new(),
                vector_clock: VectorClock::new(),
                last_sync: chrono::Utc::now(),
            }
        })
    }

    /// Get sync state for a project
    pub fn get_state(&self, project_id: &str) -> Option<&SyncState> {
        self.states.get(project_id)
    }

    /// Update a file in the sync state
    pub fn update_file(
        &mut self,
        project_id: &str,
        path: PathBuf,
        hash: ContentHash,
    ) -> Result<()> {
        let state = self.states
            .entry(project_id.to_string())
            .or_insert_with(|| SyncState {
                project_id: project_id.to_string(),
                files: HashMap::new(),
                vector_clock: VectorClock::new(),
                last_sync: chrono::Utc::now(),
            });

        // Increment vector clock
        state.vector_clock.increment(&self.device_id);
        let version = state.vector_clock.get(&self.device_id);

        // Update file state
        state.files.insert(path.clone(), FileState {
            hash,
            modified_at: chrono::Utc::now(),
            version,
            deleted: false,
        });

        debug!("Updated file: {:?} (version {})", path, version);
        Ok(())
    }

    /// Mark a file as deleted
    pub fn delete_file(&mut self, project_id: &str, path: &PathBuf) -> Result<()> {
        if let Some(state) = self.states.get_mut(project_id) {
            if let Some(file_state) = state.files.get_mut(path) {
                state.vector_clock.increment(&self.device_id);
                file_state.deleted = true;
                file_state.version = state.vector_clock.get(&self.device_id);
                debug!("Marked file as deleted: {:?}", path);
            }
        }
        Ok(())
    }

    /// Calculate diff between local and remote state
    pub fn diff(&self, project_id: &str, remote: &SyncState) -> SyncDiff {
        let mut diff = SyncDiff {
            to_send: Vec::new(),
            to_receive: Vec::new(),
            conflicts: Vec::new(),
            to_delete: Vec::new(),
        };

        let local = match self.states.get(project_id) {
            Some(s) => s,
            None => return diff,
        };

        // Get all unique file paths
        let all_paths: HashSet<_> = local.files.keys()
            .chain(remote.files.keys())
            .cloned()
            .collect();

        for path in all_paths {
            let local_file = local.files.get(&path);
            let remote_file = remote.files.get(&path);

            match (local_file, remote_file) {
                // Both have the file
                (Some(local_f), Some(remote_f)) => {
                    if local_f.hash == remote_f.hash {
                        // Same content, no action needed
                        continue;
                    }

                    // Different content - check versions
                    if local_f.version > remote_f.version {
                        // Local is newer
                        if local_f.deleted {
                            diff.to_delete.push(path);
                        } else {
                            diff.to_send.push(path);
                        }
                    } else if remote_f.version > local_f.version {
                        // Remote is newer
                        if remote_f.deleted {
                            diff.to_delete.push(path);
                        } else {
                            diff.to_receive.push(path);
                        }
                    } else {
                        // Same version but different content = conflict
                        diff.conflicts.push(path);
                    }
                }

                // Only local has the file
                (Some(local_f), None) => {
                    if !local_f.deleted {
                        diff.to_send.push(path);
                    }
                }

                // Only remote has the file
                (None, Some(remote_f)) => {
                    if !remote_f.deleted {
                        diff.to_receive.push(path);
                    }
                }

                // Neither has the file (shouldn't happen)
                (None, None) => {}
            }
        }

        info!(
            "Sync diff: {} to send, {} to receive, {} conflicts",
            diff.to_send.len(),
            diff.to_receive.len(),
            diff.conflicts.len()
        );

        diff
    }

    /// Merge remote state into local
    pub fn merge(&mut self, project_id: &str, remote: &SyncState) -> Result<()> {
        let local = self.states
            .entry(project_id.to_string())
            .or_insert_with(|| SyncState {
                project_id: project_id.to_string(),
                files: HashMap::new(),
                vector_clock: VectorClock::new(),
                last_sync: chrono::Utc::now(),
            });

        // Merge vector clocks
        local.vector_clock.merge(&remote.vector_clock);

        // Merge file states (remote wins for files with higher version)
        for (path, remote_file) in &remote.files {
            let should_update = match local.files.get(path) {
                Some(local_file) => remote_file.version > local_file.version,
                None => true,
            };

            if should_update {
                local.files.insert(path.clone(), remote_file.clone());
            }
        }

        local.last_sync = chrono::Utc::now();

        info!("Merged sync state for project: {}", project_id);
        Ok(())
    }
}

impl Default for SyncEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        clock1.increment("device1");
        clock1.increment("device1");
        clock2.increment("device2");

        assert_eq!(clock1.get("device1"), 2);
        assert_eq!(clock2.get("device2"), 1);

        // Clocks are concurrent (neither happens-before the other)
        assert!(clock1.is_concurrent(&clock2));

        // Merge clocks
        clock1.merge(&clock2);
        assert_eq!(clock1.get("device1"), 2);
        assert_eq!(clock1.get("device2"), 1);
    }

    #[test]
    fn test_sync_diff() {
        let mut engine = SyncEngine::new();
        engine.init_project("test");

        // Add a local file
        engine.update_file("test", PathBuf::from("file1.txt"), "hash1".to_string()).unwrap();

        // Create remote state with different file
        let remote = SyncState {
            project_id: "test".to_string(),
            files: [(
                PathBuf::from("file2.txt"),
                FileState {
                    hash: "hash2".to_string(),
                    modified_at: chrono::Utc::now(),
                    version: 1,
                    deleted: false,
                },
            )]
            .into_iter()
            .collect(),
            vector_clock: VectorClock::new(),
            last_sync: chrono::Utc::now(),
        };

        let diff = engine.diff("test", &remote);

        assert_eq!(diff.to_send.len(), 1);
        assert_eq!(diff.to_receive.len(), 1);
        assert!(diff.to_send.contains(&PathBuf::from("file1.txt")));
        assert!(diff.to_receive.contains(&PathBuf::from("file2.txt")));
    }
}
