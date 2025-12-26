//! Integrations with external services

pub mod github;
pub mod local_build;

use async_trait::async_trait;
use crate::{Notification, Result};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Trait for notification sources
#[async_trait]
pub trait NotificationSource: Send + Sync {
    /// Get source name
    fn name(&self) -> &str;

    /// Start watching for notifications
    async fn start(&self, tx: mpsc::Sender<Notification>) -> Result<()>;

    /// Stop watching
    async fn stop(&self) -> Result<()>;
}

/// Integration manager
pub struct IntegrationManager {
    device_id: String,
    sources: Vec<Arc<dyn NotificationSource>>,
    running: bool,
}

impl IntegrationManager {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            sources: Vec::new(),
            running: false,
        }
    }

    /// Add a notification source
    pub fn add_source(&mut self, source: Arc<dyn NotificationSource>) {
        self.sources.push(source);
    }

    /// Start all sources
    pub async fn start(&mut self, tx: mpsc::Sender<Notification>) -> Result<()> {
        self.running = true;

        for source in &self.sources {
            let tx = tx.clone();
            let source = Arc::clone(source);

            tokio::spawn(async move {
                if let Err(e) = source.start(tx).await {
                    tracing::error!("Source {} failed: {}", source.name(), e);
                }
            });
        }

        Ok(())
    }

    /// Stop all sources
    pub async fn stop(&mut self) -> Result<()> {
        self.running = false;

        for source in &self.sources {
            source.stop().await?;
        }

        Ok(())
    }
}
