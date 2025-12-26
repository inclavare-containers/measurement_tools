// src/modules/file_config_watcher.rs
use crate::config::Config;
use crate::error::{MeasurementError, Result};
use crate::modules::model_dir_measurer::ModelDirMeasurer;
use crate::modules::{watcher::ConfigWatcher, FileMeasurer};
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use log::{debug, info, warn};
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[async_trait]
pub trait ConfigChangeHandler: Send + Sync {
    fn name(&self) -> &str;
    fn is_enabled(&self, cfg: &Config) -> bool;
    async fn handle_change(
        &self,
        old_config: &Config,
        new_config: &Config,
        aa_client: Arc<AAClient>,
    ) -> Result<()>;
}

pub struct FileMeasurementChangeHandler {
    measurer: FileMeasurer,
}

impl FileMeasurementChangeHandler {
    pub fn new() -> Self {
        Self {
            measurer: FileMeasurer::new(),
        }
    }
}

#[async_trait]
impl ConfigChangeHandler for FileMeasurementChangeHandler {
    fn name(&self) -> &str {
        "FileMeasurementChangeHandler"
    }

    fn is_enabled(&self, cfg: &Config) -> bool {
        cfg.file_measurement.enable
    }

    async fn handle_change(
        &self,
        old_config: &Config,
        new_config: &Config,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let old_files: HashSet<String> = old_config.file_measurement.files.iter().cloned().collect();
        let new_files: HashSet<String> = new_config.file_measurement.files.iter().cloned().collect();
        let added: Vec<String> = new_files.difference(&old_files).cloned().collect();

        if added.is_empty() {
            debug!("No new file measurement patterns detected.");
            return Ok(());
        }

        info!(
            "Detected {} new file measurement patterns; triggering measurement.",
            added.len()
        );
        self.measurer
            .measure_patterns(&added, &new_config.file_measurement, aa_client)
            .await
    }
}

pub struct ModelDirMeasurementChangeHandler {
    measurer: ModelDirMeasurer,
}

impl ModelDirMeasurementChangeHandler {
    pub fn new() -> Self {
        Self {
            measurer: ModelDirMeasurer::new(),
        }
    }
}

#[async_trait]
impl ConfigChangeHandler for ModelDirMeasurementChangeHandler {
    fn name(&self) -> &str {
        "ModelDirMeasurementChangeHandler"
    }

    fn is_enabled(&self, cfg: &Config) -> bool {
        cfg.model_dir_measurement.enable
    }

    async fn handle_change(
        &self,
        old_config: &Config,
        new_config: &Config,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let old_dirs: HashSet<String> =
            old_config.model_dir_measurement.directories.iter().cloned().collect();
        let new_dirs: HashSet<String> =
            new_config.model_dir_measurement.directories.iter().cloned().collect();
        let added: Vec<String> = new_dirs.difference(&old_dirs).cloned().collect();

        if added.is_empty() {
            debug!("No new model directory entries detected.");
            return Ok(());
        }

        info!(
            "Detected {} new model directories; triggering measurement.",
            added.len()
        );

        // Reuse measurer logic; it will deduplicate internally.
        self.measurer
            .measure_specific_dirs(&added, &new_config.model_dir_measurement, aa_client)
            .await
    }
}

pub struct ConfigFileWatcher {
    handlers: Vec<Box<dyn ConfigChangeHandler>>,
}

impl ConfigFileWatcher {
    pub fn new(handlers: Vec<Box<dyn ConfigChangeHandler>>) -> Self {
        Self { handlers }
    }
}

#[async_trait]
impl ConfigWatcher for ConfigFileWatcher {
    fn name(&self) -> &str {
        "ConfigFileWatcher"
    }

    fn is_enabled(&self, _config: Arc<Config>) -> bool {
        // Enabled if any handler is enabled for the current config; actual check is done per event.
        true
    }

    async fn watch(
        &self,
        config_path: PathBuf,
        shared_config: Arc<RwLock<Config>>,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let parent_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let Some(config_file_name) = config_path.file_name().map(|s| s.to_os_string()) else {
            return Err(MeasurementError::InvalidDirectory(format!(
                "Config path {:?} is missing file name",
                config_path
            )));
        };

        tokio::task::spawn_blocking(move || {
            let tx_clone = tx.clone();
            let watcher_result = recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let _ = tx_clone.send(event);
                }
            })
            .and_then(|mut watcher| {
                watcher.watch(&parent_dir, RecursiveMode::NonRecursive)?;
                Ok(watcher)
            });

            if watcher_result.is_err() {
                return;
            }

            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        });

        loop {
            if let Some(event) = rx.recv().await {
                if !is_relevant_event(&event.kind) {
                    continue;
                }
                if !event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == Some(&config_file_name))
                {
                    continue;
                }

                // Debounce rapid writes
                sleep(Duration::from_millis(150)).await;

                let old_config = { shared_config.read().await.clone() };
                let new_config = match Config::load(Some(&config_path)) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        warn!("Failed to reload config after change: {}", e);
                        continue;
                    }
                };

                {
                    let mut guard = shared_config.write().await;
                    *guard = new_config.clone();
                }

                for handler in &self.handlers {
                    if handler.is_enabled(&new_config) {
                        if let Err(e) = handler
                            .handle_change(&old_config, &new_config, aa_client.clone())
                            .await
                        {
                            warn!(
                                "Handler {} failed during config change: {}",
                                handler.name(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }
}

fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any
    )
}
