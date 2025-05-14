use crate::config::Config;
use crate::error::Result;
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Measurable {
    /// Returns the name of the measurer (e.g., "FileMeasurer").
    fn name(&self) -> &str;

    /// Checks if this measurer is enabled in the configuration.
    fn is_enabled(&self, config: Arc<Config>) -> bool;

    /// Performs the measurement and sends results via the AAClient.
    async fn measure(&self, config: Arc<Config>, aa_client: Arc<AAClient>) -> Result<()>;
} 