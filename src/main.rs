// src/main.rs
mod config;
mod error;
mod modules;
mod rpc_client;
mod rpc_generated; // Module for ttrpc generated code

use crate::config::Config;
use crate::modules::{
    ConfigChangeHandler, ConfigFileWatcher, ConfigWatcher, FileMeasurementChangeHandler,
    FileMeasurer, Measurable, ModelDirMeasurementChangeHandler, ModelDirMeasurer,
};
use crate::rpc_client::AAClient;
use anyhow::Result;
use log::{error, info};
use std::env;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger based on RUST_LOG env var, or default to info
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config_path_str = env::args().nth(1);
    let config_path = config_path_str.as_ref().map(PathBuf::from);
    if let Some(ref path) = config_path {
        info!("Loading configuration from: {:?}", path);
    } else {
        info!("Use default configuration");
    }

    info!("measurement tool starting...");

    let config = match Config::load(config_path.as_deref()) {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            exit(1);
        }
    };

    let aa_client = match AAClient::from_config(&config).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("Failed to connect to Attestation Agent: {}", e);
            exit(1);
        }
    };

    // Shared config for runtime watchers
    let shared_config = Arc::new(RwLock::new((*config).clone()));

    // --- Register Measurers ---
    // Add new measurers to this vector as they are implemented.
    let measurers: Vec<Box<dyn Measurable + Send + Sync>> = vec![
        Box::new(FileMeasurer::new()),
        Box::new(ModelDirMeasurer::new()),
        // Box::new(ProcessMeasurer::new()), // Example for future measurer
    ];
    // --------------------------

    // Initial one-shot run
    {
        let config_snapshot = {
            let guard = shared_config.read().await;
            guard.clone()
        };
        let arc_snapshot = Arc::new(config_snapshot);
        let mut success = true;
        for measurer in measurers {
            if measurer.is_enabled(arc_snapshot.clone()) {
                info!("Running measurer: {}", measurer.name());
                if let Err(e) = measurer
                    .measure(arc_snapshot.clone(), aa_client.clone())
                    .await
                {
                    error!("Error during {} execution: {}", measurer.name(), e);
                    success = false;
                }
            } else {
                info!("Measurer {} is disabled. Skipping.", measurer.name());
            }
        }
        if !success {
            error!("One or more measurements failed during initial run.");
        } else {
            info!("Initial measurement run completed successfully.");
        }
    }

    if config.one_shot {
        info!("One-shot mode enabled. Exiting after initial measurement.");
        return Ok(());
    }

    // Determine effective config path for watcher
    let effective_config_path =
        config_path.unwrap_or_else(|| PathBuf::from("runtime-measurer-config.toml"));

    // Spawn config watchers
    let config_handlers: Vec<Box<dyn ConfigChangeHandler>> = vec![
        Box::new(FileMeasurementChangeHandler::new()),
        Box::new(ModelDirMeasurementChangeHandler::new()),
    ];

    let watchers: Vec<Box<dyn ConfigWatcher + Send + Sync>> = vec![Box::new(
        ConfigFileWatcher::new(config_handlers),
    )];
    for watcher in watchers {
        if watcher.is_enabled(Arc::new(shared_config.read().await.clone())) {
            let cfg = shared_config.clone();
            let aa = aa_client.clone();
            let path = effective_config_path.clone();
            tokio::spawn(async move {
                if let Err(e) = watcher.watch(path, cfg, aa).await {
                    error!("Config watcher exited with error: {}", e);
                }
            });
        } else {
            info!("Watcher {} is disabled. Skipping.", watcher.name());
        }
    }

    // Keep running as a daemon
    std::future::pending::<()>().await;
    #[allow(unreachable_code)]
    Ok(())
}
