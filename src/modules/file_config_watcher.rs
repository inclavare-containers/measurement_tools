// src/modules/file_config_watcher.rs
use crate::config::Config;
use crate::error::{MeasurementError, Result};
use crate::modules::model_dir_measurer::ModelDirMeasurer;
use crate::modules::{watcher::ConfigWatcher, FileMeasurer};
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use hex;
use log::{debug, info, warn};
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
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

const MAX_RELOAD_RETRIES: usize = 3;
const RELOAD_RETRY_DELAY_MS: u64 = 200;

fn load_config_with_hash(path: &Path) -> Result<(Config, String)> {
    let content = fs::read_to_string(path).map_err(|e| {
        MeasurementError::InvalidDirectory(format!(
            "Failed to read config {:?}: {}",
            path, e
        ))
    })?;
    let cfg: Config = toml::from_str(&content).map_err(|e| {
        MeasurementError::Config(format!(
            "Failed to parse config {:?}: {}",
            path, e
        ))
    })?;
    let hash = hex::encode(Sha256::digest(content.as_bytes()));
    Ok((cfg, hash))
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

        let mut last_config_hash: Option<String> = None;

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

                let mut new_config: Option<Config> = None;
                let mut new_hash: Option<String> = None;

                for attempt in 1..=MAX_RELOAD_RETRIES {
                    match load_config_with_hash(&config_path) {
                        Ok((cfg, hash)) => {
                            new_config = Some(cfg);
                            new_hash = Some(hash);
                            break;
                        }
                        Err(e) => {
                            warn!(
                                "Failed to reload config (attempt {}/{}): {}",
                                attempt, MAX_RELOAD_RETRIES, e
                            );
                            if attempt < MAX_RELOAD_RETRIES {
                                sleep(Duration::from_millis(RELOAD_RETRY_DELAY_MS)).await;
                            }
                        }
                    }
                }

                let new_config = match new_config {
                    Some(cfg) => cfg,
                    None => {
                        warn!("Giving up config reload after {} attempts.", MAX_RELOAD_RETRIES);
                        continue;
                    }
                };

                let new_hash = new_hash.unwrap_or_default();

                if last_config_hash.as_ref() == Some(&new_hash) {
                    debug!("Config content unchanged; skipping handlers.");
                    continue;
                }

                {
                    let mut guard = shared_config.write().await;
                    *guard = new_config.clone();
                }
                last_config_hash = Some(new_hash);

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
