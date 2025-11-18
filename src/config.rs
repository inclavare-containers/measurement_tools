// src/config.rs
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub attestation_agent_socket: String,
    #[serde(default)]
    pub file_measurement: FileMeasurementConfig,
    // Add other measurement configs here as they are implemented
    // pub process_measurement: ProcessMeasurementConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileMeasurementConfig {
    #[serde(default = "default_false")]
    pub enable: bool,
    #[serde(default = "default_pcr_index")]
    pub pcr_index: u32,
    #[serde(default = "default_hash_algorithm")]
    pub hash_algorithm: String, // e.g., "sha256", "sha384"
    #[serde(default)]
    pub files: Vec<String>,
}

fn default_false() -> bool {
    false
}

fn default_pcr_index() -> u32 {
    18 // Default PCR for this tool, distinct from AA's internal one
}

fn default_hash_algorithm() -> String {
    "sha256".to_string()
}

impl Default for FileMeasurementConfig {
    fn default() -> Self {
        Self {
            enable: default_false(),
            pcr_index: default_pcr_index(),
            hash_algorithm: default_hash_algorithm(),
            files: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let path = config_path.unwrap_or_else(|| Path::new("runtime-measurer-config.toml"));
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file: {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML from config file: {:?}", path))?;
        Ok(config)
    }
}
