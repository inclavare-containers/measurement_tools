// src/modules/file_measurer.rs
use crate::config::{Config, FileMeasurementConfig};
use crate::error::{MeasurementError, Result};
use crate::modules::measurable::Measurable;
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use glob::glob;
use log::{debug, info, warn};
use sha2::{Digest, Sha256, Sha384};
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

pub struct FileMeasurer;

const DOMAIN: &str = "file";

impl FileMeasurer {
    pub fn new() -> Self {
        Self
    }

    pub async fn measure_patterns(
        &self,
        patterns: &[String],
        fm_config: &FileMeasurementConfig,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let mut measured_files = HashSet::new();
        for pattern in patterns {
            match glob(pattern) {
                Ok(entries) => {
                    for entry in entries {
                        if let Ok(path) = entry {
                            if path.is_file() {
                                let path_str = path.to_string_lossy().to_string();
                                if measured_files.insert(path_str.clone()) {
                                    self.measure_single_file(
                                        &path_str,
                                        fm_config,
                                        aa_client.clone(),
                                    )
                                    .await?;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid glob pattern '{}': {}", pattern, e);
                }
            }
        }
        Ok(())
    }

    async fn measure_single_file(
        &self,
        file_path: &str,
        fm_config: &FileMeasurementConfig,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        debug!("Measuring file: {}", file_path);
        match fs::read(file_path) {
            Ok(content) => {
                let file_hash_hex = match fm_config.hash_algorithm.to_lowercase().as_str() {
                    "sha256" => {
                        let mut hasher = Sha256::new();
                        hasher.update(&content);
                        hex::encode(hasher.finalize())
                    }
                    "sha384" => {
                        let mut hasher = Sha384::new();
                        hasher.update(&content);
                        hex::encode(hasher.finalize())
                    }
                    other => {
                        return Err(MeasurementError::UnsupportedHashAlgorithm(
                            other.to_string(),
                        ));
                    }
                };

                debug!(
                    "Extending measurement for file: {}, PCR: {}, Domain: {}, Operation: {}, Content: {}",
                    file_path, fm_config.pcr_index, DOMAIN, file_path, file_hash_hex
                );

                aa_client
                    .extend_runtime_measurement(
                        Some(fm_config.pcr_index as u64),
                        DOMAIN,
                        file_path,
                        &file_hash_hex,
                    )
                    .await?;
                Ok(())
            }
            Err(e) => {
                warn!("Failed to read file for measurement '{}': {}", file_path, e);
                // Decide if this should be a hard error or just a warning
                // For now, just warn and continue with other files.
                Ok(())
            }
        }
    }
}

#[async_trait]
impl Measurable for FileMeasurer {
    fn name(&self) -> &str {
        "FileMeasurer"
    }

    fn is_enabled(&self, config: Arc<Config>) -> bool {
        config.file_measurement.enable
    }

    async fn measure(&self, config: Arc<Config>, aa_client: Arc<AAClient>) -> Result<()> {
        let fm_config = &config.file_measurement;
        if !fm_config.enable {
            debug!("File measurement is disabled. Skipping.");
            return Ok(());
        }

        info!(
            "Starting file measurement with PCR index: {}, Domain: {}, Hash Alg: {}",
            fm_config.pcr_index, DOMAIN, fm_config.hash_algorithm
        );

        let mut measured_files = HashSet::new();

        for pattern in &fm_config.files {
            debug!("Processing pattern: {}", pattern);

            match glob(pattern) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(path) => {
                                if path.is_file() {
                                    let path_str = path.to_string_lossy().to_string();
                                    if measured_files.insert(path_str.clone()) {
                                        self.measure_single_file(
                                            &path_str,
                                            fm_config,
                                            aa_client.clone(),
                                        )
                                        .await?;
                                    } else {
                                        debug!("Skipping already measured file: {}", path_str);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Error while accessing path matched by pattern '{}': {}",
                                    pattern, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Invalid glob pattern '{}': {}", pattern, e);
                }
            }
        }

        info!(
            "File measurement completed. Measured {} unique files.",
            measured_files.len()
        );
        Ok(())
    }
}
