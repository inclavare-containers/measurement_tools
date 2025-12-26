use crate::config::{Config, ModelDirMeasurementConfig};
use crate::error::{MeasurementError, Result};
use crate::modules::measurable::Measurable;
use crate::rpc_client::AAClient;
use async_trait::async_trait;
use log::{debug, info, warn};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::process::Command;

const DOMAIN: &str = "model_dir";

pub struct ModelDirMeasurer;

impl ModelDirMeasurer {
    pub fn new() -> Self {
        Self
    }

    pub async fn measure_specific_dirs(
        &self,
        directories: &[String],
        config: &ModelDirMeasurementConfig,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let mut measured_dirs = HashSet::new();
        for dir in directories {
            if measured_dirs.insert(dir.clone()) {
                self.measure_single_dir(dir, config, aa_client.clone()).await?;
            } else {
                debug!("Skipping duplicate directory entry: {}", dir);
            }
        }
        Ok(())
    }

    async fn measure_single_dir(
        &self,
        dir: &str,
        config: &ModelDirMeasurementConfig,
        aa_client: Arc<AAClient>,
    ) -> Result<()> {
        let dir_path = PathBuf::from(dir);
        let canonical_dir = dir_path
            .canonicalize()
            .map_err(|e| MeasurementError::InvalidDirectory(format!("{} ({})", dir, e)))?;
        let canonical_dir_str = canonical_dir.to_string_lossy().to_string();

        if !canonical_dir.is_dir() {
            return Err(MeasurementError::InvalidDirectory(format!(
                "{} is not a directory",
                canonical_dir_str
            )));
        }

        let hash_file = NamedTempFile::new().map_err(|e| {
            MeasurementError::CommandExecution(format!(
                "Failed to create temp hash file for {}: {}",
                canonical_dir.to_string_lossy(),
                e
            ))
        })?;
        let hash_file_path = hash_file.path().to_path_buf();

        info!(
            "Formatting model directory with cryptpilot: {:?}",
            canonical_dir
        );
        let hash_output_str = hash_file_path.to_string_lossy().to_string();
        self.run_command(
            &config.cryptpilot_binary,
            &[
                "verity",
                "format",
                canonical_dir_str.as_str(),
                "--hash-output",
                hash_output_str.as_str(),
            ],
        )
        .await?;

        info!(
            "Dumping root hash for model directory with cryptpilot: {:?}",
            canonical_dir
        );
        let dump_output = self
            .run_command(
                &config.cryptpilot_binary,
                &[
                    "verity",
                    "dump",
                    "--data-dir",
                    canonical_dir_str.as_str(),
                    "--print-root-hash",
                ],
            )
            .await?;

        let root_hash = String::from_utf8_lossy(&dump_output.stdout)
            .trim()
            .to_string();

        if root_hash.is_empty() {
            return Err(MeasurementError::CommandExecution(format!(
                "Empty root hash returned for directory {}",
                canonical_dir.to_string_lossy()
            )));
        }

        debug!(
            "Extending model directory measurement: domain={}, operation={}, root_hash={}",
            DOMAIN,
            canonical_dir_str.as_str(),
            root_hash
        );

        aa_client
            .extend_runtime_measurement(
                config.pcr_index.map(|v| v as u64),
                DOMAIN,
                canonical_dir_str.as_str(),
                &root_hash,
            )
            .await?;

        Ok(())
    }

    async fn run_command(&self, binary: &str, args: &[&str]) -> Result<std::process::Output> {
        let output = Command::new(binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                MeasurementError::CommandExecution(format!(
                    "Failed to run command '{} {}': {}",
                    binary,
                    args.join(" "),
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MeasurementError::CommandExecution(format!(
                "Command '{} {}' failed with status {}: {}",
                binary,
                args.join(" "),
                output.status,
                stderr.trim()
            )));
        }

        Ok(output)
    }
}

#[async_trait]
impl Measurable for ModelDirMeasurer {
    fn name(&self) -> &str {
        "ModelDirMeasurer"
    }

    fn is_enabled(&self, config: Arc<Config>) -> bool {
        config.model_dir_measurement.enable
    }

    async fn measure(&self, config: Arc<Config>, aa_client: Arc<AAClient>) -> Result<()> {
        let md_config = &config.model_dir_measurement;
        if !md_config.enable {
            debug!("Model directory measurement is disabled. Skipping.");
            return Ok(());
        }

        if md_config.directories.is_empty() {
            warn!("Model directory measurement is enabled but no directories configured.");
            return Ok(());
        }

        info!(
            "Starting model directory measurement with domain '{}' using cryptpilot binary '{}'",
            DOMAIN, md_config.cryptpilot_binary
        );

        let mut measured_dirs = HashSet::new();

        for dir in &md_config.directories {
            if measured_dirs.insert(dir.clone()) {
                self.measure_single_dir(dir, md_config, aa_client.clone())
                    .await?;
            } else {
                debug!("Skipping duplicate directory entry: {}", dir);
            }
        }

        info!(
            "Model directory measurement completed for {} unique directories.",
            measured_dirs.len()
        );
        Ok(())
    }
}

