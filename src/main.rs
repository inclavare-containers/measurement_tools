// src/main.rs
mod config;
mod error;
mod modules;
mod rpc_client;
mod rpc_generated; // Module for ttrpc generated code

use crate::config::Config;
use crate::modules::{FileMeasurer, Measurable};
use crate::rpc_client::AAClient;
use anyhow::Result;
use log::{error, info};
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger based on RUST_LOG env var, or default to info
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_path_str = env::args().nth(1);
    let config_path = config_path_str.as_ref().map(PathBuf::from);

    info!("Runtime Measurer starting...");

    let config = match Config::load(config_path.as_deref()) {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            exit(1);
        }
    };

    let aa_client = match AAClient::new(&config.attestation_agent_socket).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("Failed to connect to Attestation Agent: {}", e);
            exit(1);
        }
    };

    // --- Register Measurers ---
    // Add new measurers to this vector as they are implemented.
    let measurers: Vec<Box<dyn Measurable + Send + Sync>> = vec![
        Box::new(FileMeasurer::new()),
        // Box::new(ProcessMeasurer::new()), // Example for future measurer
    ];
    // --------------------------

    let mut success = true;

    for measurer in measurers {
        if measurer.is_enabled(config.clone()) {
            info!("Running measurer: {}", measurer.name());
            if let Err(e) = measurer.measure(config.clone(), aa_client.clone()).await {
                error!("Error during {} execution: {}", measurer.name(), e);
                success = false;
            }
        } else {
            info!("Measurer {} is disabled. Skipping.", measurer.name());
        }
    }

    if success {
        info!("All enabled measurements completed successfully.");
        Ok(())
    } else {
        error!("One or more measurements failed.");
        exit(1);
    }
}
