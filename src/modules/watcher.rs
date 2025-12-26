// src/modules/watcher.rs
use crate::config::Config;
use crate::error::Result;
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait ConfigWatcher {
    /// Returns the name of the watcher (e.g., "FileConfigWatcher").
    fn name(&self) -> &str;

    /// Checks if this watcher is enabled for the provided config snapshot.
    fn is_enabled(&self, config: Arc<Config>) -> bool;

    /// Starts watching based on the provided config path and shared config.
    async fn watch(
        &self,
        config_path: PathBuf,
        shared_config: Arc<RwLock<Config>>,
        aa_client: Arc<AAClient>,
    ) -> Result<()>;
}
