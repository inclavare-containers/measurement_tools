// src/modules/file_config_watcher.rs
use crate::config::Config;
use crate::error::Result;
use crate::modules::{watcher::ConfigWatcher, FileMeasurer};
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use log::{info, warn};
use notify::{recommended_watcher, EventKind, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

pub struct FileConfigWatcher;

impl FileConfigWatcher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ConfigWatcher for FileConfigWatcher {
    fn name(&self) -> &str {
        "FileConfigWatcher"
    }

    fn is_enabled(&self, config: Arc<Config>) -> bool {
        config.file_measurement.enable
    }

    async fn watch(
        &self,
        config_path: PathBuf,
        shared_config: Arc<RwLock<Config>>,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let mut last_files_set: HashSet<String> = {
            let cfg = shared_config.read().await;
            cfg.file_measurement.files.iter().cloned().collect()
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let parent_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let config_file_name = config_path
            .file_name()
            .map(|s| s.to_os_string())
            .expect("config path must include a file name");

        tokio::task::spawn_blocking(move || {
            let tx_clone = tx.clone();
            let mut watcher = recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let _ = tx_clone.send(event);
                }
            })
            .expect("failed to create file watcher");

            watcher
                .watch(&parent_dir, RecursiveMode::NonRecursive)
                .expect("failed to start watching config directory");

            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        });

        let file_measurer = FileMeasurer::new();

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

                let new_config = match Config::load(Some(&config_path)) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        warn!("Failed to reload config after change: {}", e);
                        continue;
                    }
                };

                let new_files_set: HashSet<String> =
                    new_config.file_measurement.files.iter().cloned().collect();
                let added: Vec<String> =
                    new_files_set.difference(&last_files_set).cloned().collect();

                if !added.is_empty() {
                    info!("Detected new file measurement patterns: {}", added.len());
                }

                {
                    let mut guard = shared_config.write().await;
                    *guard = new_config.clone();
                }
                last_files_set = new_files_set;

                if new_config.file_measurement.enable && !added.is_empty() {
                    file_measurer
                        .measure_patterns(&added, &new_config.file_measurement, aa_client.clone())
                        .await?;
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
