//! # Coachly Code Bridge Core
//!
//! A P2P file sharing library designed for developers.
//! Enables seamless sharing of codebases and screenshots across
//! macOS and Linux environments.

pub mod config;
pub mod p2p;
pub mod storage;
pub mod sync;
pub mod watch;

pub use config::Config;
pub use p2p::PeerNetwork;
pub use storage::ContentStore;
pub use sync::SyncEngine;
pub use watch::FileWatcher;

use thiserror::Error;

/// Core error types for Code Bridge
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("P2P network error: {0}")]
    Network(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Sync error: {0}")]
    Sync(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File watch error: {0}")]
    Watch(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BridgeError>;

/// Initialize the Code Bridge runtime
pub async fn init() -> Result<Bridge> {
    Bridge::new().await
}

/// Main Code Bridge instance
pub struct Bridge {
    config: Config,
    network: PeerNetwork,
    storage: ContentStore,
    watcher: FileWatcher,
    sync_engine: SyncEngine,
}

impl Bridge {
    /// Create a new Bridge instance
    pub async fn new() -> Result<Self> {
        let config = Config::load_or_create()?;
        let storage = ContentStore::new(&config)?;
        let network = PeerNetwork::new(&config).await?;
        let watcher = FileWatcher::new()?;
        let sync_engine = SyncEngine::new();

        Ok(Self {
            config,
            network,
            storage,
            watcher,
            sync_engine,
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the peer network
    pub fn network(&self) -> &PeerNetwork {
        &self.network
    }

    /// Get mutable access to the peer network
    pub fn network_mut(&mut self) -> &mut PeerNetwork {
        &mut self.network
    }

    /// Get the content store
    pub fn storage(&self) -> &ContentStore {
        &self.storage
    }

    /// Get the file watcher
    pub fn watcher(&self) -> &FileWatcher {
        &self.watcher
    }

    /// Get mutable access to the file watcher
    pub fn watcher_mut(&mut self) -> &mut FileWatcher {
        &mut self.watcher
    }

    /// Get the sync engine
    pub fn sync_engine(&self) -> &SyncEngine {
        &self.sync_engine
    }

    /// Start the bridge (networking, watching, syncing)
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Code Bridge...");

        // Start the P2P network
        self.network.start().await?;

        tracing::info!("Code Bridge started successfully");
        Ok(())
    }

    /// Stop the bridge
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping Code Bridge...");
        self.network.stop().await?;
        Ok(())
    }
}
