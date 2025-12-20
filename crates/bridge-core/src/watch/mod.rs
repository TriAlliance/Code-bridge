//! File system watching using the notify crate
//!
//! Monitors directories for changes and emits events
//! for file creation, modification, and deletion.

use crate::{BridgeError, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tracing::{debug, info, warn};

/// File change event
#[derive(Debug, Clone)]
pub enum FileChange {
    /// File was created
    Created(PathBuf),

    /// File was modified
    Modified(PathBuf),

    /// File was deleted
    Deleted(PathBuf),

    /// File was renamed (from, to)
    Renamed(PathBuf, PathBuf),
}

/// File watcher using notify crate
pub struct FileWatcher {
    /// The debouncer (wraps the watcher)
    debouncer: Debouncer<RecommendedWatcher>,

    /// Receiver for debounced events
    rx: Receiver<std::result::Result<Vec<DebouncedEvent>, notify::Error>>,

    /// Watched paths
    watched_paths: HashSet<PathBuf>,

    /// Ignore patterns (glob patterns)
    ignore_patterns: Vec<glob::Pattern>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();

        let debouncer = new_debouncer(Duration::from_millis(500), tx)
            .map_err(|e| BridgeError::Watch(e.to_string()))?;

        info!("File watcher initialized");

        Ok(Self {
            debouncer,
            rx,
            watched_paths: HashSet::new(),
            ignore_patterns: vec![],
        })
    }

    /// Set ignore patterns
    pub fn set_ignore_patterns(&mut self, patterns: &[String]) -> Result<()> {
        self.ignore_patterns = patterns
            .iter()
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();

        debug!("Set {} ignore patterns", self.ignore_patterns.len());
        Ok(())
    }

    /// Watch a directory
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        let canonical = path
            .canonicalize()
            .map_err(|e| BridgeError::Watch(format!("Failed to canonicalize path: {}", e)))?;

        if self.watched_paths.contains(&canonical) {
            debug!("Already watching: {:?}", canonical);
            return Ok(());
        }

        self.debouncer
            .watcher()
            .watch(&canonical, RecursiveMode::Recursive)
            .map_err(|e| BridgeError::Watch(e.to_string()))?;

        self.watched_paths.insert(canonical.clone());
        info!("Watching: {:?}", canonical);

        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        let canonical = path
            .canonicalize()
            .map_err(|e| BridgeError::Watch(format!("Failed to canonicalize path: {}", e)))?;

        if !self.watched_paths.contains(&canonical) {
            return Ok(());
        }

        self.debouncer
            .watcher()
            .unwatch(&canonical)
            .map_err(|e| BridgeError::Watch(e.to_string()))?;

        self.watched_paths.remove(&canonical);
        info!("Stopped watching: {:?}", canonical);

        Ok(())
    }

    /// Get watched paths
    pub fn watched_paths(&self) -> &HashSet<PathBuf> {
        &self.watched_paths
    }

    /// Poll for file changes (non-blocking)
    pub fn poll(&self) -> Vec<FileChange> {
        let mut changes = Vec::new();

        while let Ok(result) = self.rx.try_recv() {
            match result {
                Ok(events) => {
                    for event in events {
                        if !self.should_ignore(&event.path) {
                            // The debouncer gives us simplified events
                            if event.path.exists() {
                                changes.push(FileChange::Modified(event.path));
                            } else {
                                changes.push(FileChange::Deleted(event.path));
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Watch error: {:?}", e);
                }
            }
        }

        changes
    }

    /// Wait for file changes (blocking)
    pub fn wait(&self) -> Result<Vec<FileChange>> {
        match self.rx.recv() {
            Ok(result) => {
                let mut changes = Vec::new();

                match result {
                    Ok(events) => {
                        for event in events {
                            if !self.should_ignore(&event.path) {
                                if event.path.exists() {
                                    changes.push(FileChange::Modified(event.path));
                                } else {
                                    changes.push(FileChange::Deleted(event.path));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Watch error: {:?}", e);
                    }
                }

                Ok(changes)
            }
            Err(e) => Err(BridgeError::Watch(format!("Channel closed: {}", e))),
        }
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check ignore patterns
        for pattern in &self.ignore_patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }

        // Always ignore hidden files and common build artifacts
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                return true;
            }

            // Common ignore patterns
            let ignore_names = [
                "node_modules",
                "target",
                "__pycache__",
                ".git",
                ".idea",
                ".vscode",
                "build",
                "dist",
            ];

            for component in path.components() {
                if let std::path::Component::Normal(name) = component {
                    if ignore_names.contains(&name.to_str().unwrap_or("")) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// Watch a directory for screenshots
pub struct ScreenshotWatcher {
    watcher: FileWatcher,
    screenshot_extensions: Vec<String>,
}

impl ScreenshotWatcher {
    /// Create a new screenshot watcher
    pub fn new() -> Result<Self> {
        let watcher = FileWatcher::new()?;

        Ok(Self {
            watcher,
            screenshot_extensions: vec![
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
                "webp".to_string(),
                "avif".to_string(),
            ],
        })
    }

    /// Watch a directory for screenshots
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        self.watcher.watch(path)
    }

    /// Poll for new screenshots
    pub fn poll(&self) -> Vec<PathBuf> {
        self.watcher
            .poll()
            .into_iter()
            .filter_map(|change| match change {
                FileChange::Created(path) | FileChange::Modified(path) => {
                    if self.is_screenshot(&path) {
                        Some(path)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect()
    }

    /// Check if a file is a screenshot
    fn is_screenshot(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.screenshot_extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_watcher() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_directory() {
        let dir = tempdir().unwrap();
        let mut watcher = FileWatcher::new().unwrap();

        let result = watcher.watch(dir.path());
        assert!(result.is_ok());
        assert!(watcher.watched_paths().contains(&dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_should_ignore() {
        let watcher = FileWatcher::new().unwrap();

        assert!(watcher.should_ignore(Path::new("/project/.git/config")));
        assert!(watcher.should_ignore(Path::new("/project/node_modules/package.json")));
        assert!(watcher.should_ignore(Path::new("/project/.hidden")));
        assert!(!watcher.should_ignore(Path::new("/project/src/main.rs")));
    }
}
