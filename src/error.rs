// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MeasurementError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Glob pattern error: {0}")]
    Pattern(#[from] glob::PatternError),

    #[error("RPC client error: {0}")]
    RpcClient(String),

    #[error("Unsupported hash algorithm: {0}")]
    UnsupportedHashAlgorithm(String),

    #[error("Invalid directory for measurement: {0}")]
    InvalidDirectory(String),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Attestation agent client error: {0}")]
    AttestationAgentClient(#[from] ttrpc::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, MeasurementError>;
