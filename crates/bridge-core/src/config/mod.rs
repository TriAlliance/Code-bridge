//! Configuration management for Code Bridge

use crate::{BridgeError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for Code Bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Unique device identifier
    pub device_id: String,

    /// Human-readable device name
    pub device_name: String,

    /// Data directory for storing files and database
    pub data_dir: PathBuf,

    /// Network configuration
    pub network: NetworkConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Sync configuration
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Port for P2P connections (0 = random)
    pub listen_port: u16,

    /// Enable mDNS for local network discovery
    pub enable_mdns: bool,

    /// Enable DHT for global discovery
    pub enable_dht: bool,

    /// Bootstrap peers for DHT
    pub bootstrap_peers: Vec<String>,

    /// Enable QUIC transport
    pub enable_quic: bool,

    /// Enable TCP transport (fallback)
    pub enable_tcp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum cache size in bytes
    pub max_cache_size: u64,

    /// Enable compression for stored files
    pub enable_compression: bool,

    /// Chunk size for content-addressed storage (bytes)
    pub chunk_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Directories to watch for changes
    pub watch_paths: Vec<PathBuf>,

    /// Patterns to ignore (glob patterns)
    pub ignore_patterns: Vec<String>,

    /// Enable real-time sync
    pub realtime_sync: bool,

    /// Sync interval in seconds (for periodic sync)
    pub sync_interval_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = directories::ProjectDirs::from("io", "coachly", "codebridge")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".codebridge"));

        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            data_dir,
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            sync: SyncConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 0, // Random port
            enable_mdns: true,
            enable_dht: true,
            bootstrap_peers: vec![],
            enable_quic: true,
            enable_tcp: true,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 10 * 1024 * 1024 * 1024, // 10 GB
            enable_compression: true,
            chunk_size: 256 * 1024, // 256 KB chunks
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![],
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".DS_Store".to_string(),
                "*.pyc".to_string(),
                "__pycache__".to_string(),
                ".idea".to_string(),
                ".vscode".to_string(),
                "*.swp".to_string(),
                "*.swo".to_string(),
            ],
            realtime_sync: true,
            sync_interval_secs: 30,
        }
    }
}

impl Config {
    /// Load configuration from disk, or create default if not exists
    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        tracing::info!("Configuration saved to {:?}", config_path);
        Ok(())
    }

    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        directories::ProjectDirs::from("io", "coachly", "codebridge")
            .map(|dirs| dirs.config_dir().join("config.json"))
            .ok_or_else(|| BridgeError::Config("Could not determine config directory".to_string()))
    }

    /// Get the data directory, creating it if necessary
    pub fn ensure_data_dir(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.data_dir)?;
        Ok(self.data_dir.clone())
    }

    /// Get the database path
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("bridge.db")
    }

    /// Get the content store path
    pub fn content_store_path(&self) -> PathBuf {
        self.data_dir.join("objects")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.device_id.is_empty());
        assert!(config.network.enable_mdns);
        assert!(config.network.enable_quic);
    }
}
