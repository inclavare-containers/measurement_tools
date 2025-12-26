// src/rpc_client.rs
use crate::config::{Config, MeasurementChannel};
use crate::error::{MeasurementError, Result};
use crate::rpc_generated::attestation_agent::ExtendRuntimeMeasurementRequest;
use crate::rpc_generated::attestation_agent_ttrpc::AttestationAgentServiceClient;
use log::{debug, info};
use serde::Serialize;
use ttrpc::asynchronous::Client;

enum ClientImpl {
    Ttrpc(AttestationAgentServiceClient),
    Http {
        http_client: reqwest::Client,
        base_url: String,
    },
}

pub struct AAClient {
    inner: ClientImpl,
}

#[derive(Serialize)]
struct HttpAaelRequest<'a> {
    domain: &'a str,
    operation: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    register_index: Option<u64>,
}

impl AAClient {
    pub async fn from_config(config: &Config) -> Result<Self> {
        match config.aa_channel {
            MeasurementChannel::UnixSocket => {
                info!(
                    "Connecting to Attestation Agent via ttrpc socket: {}",
                    config.attestation_agent_socket
                );
                let client = Client::connect(&config.attestation_agent_socket).map_err(|e| {
                    MeasurementError::RpcClient(format!(
                        "Failed to connect to AA: {}",
                        e.to_string()
                    ))
                })?;
                Ok(Self {
                    inner: ClientImpl::Ttrpc(AttestationAgentServiceClient::new(client)),
                })
            }
            MeasurementChannel::HttpApi => {
                let base_url = config.trustiflux_api_endpoint.clone().ok_or_else(|| {
                    MeasurementError::Config(
                        "trustiflux_api_endpoint must be set when measurement_channel=http_api"
                            .to_string(),
                    )
                })?;
                info!(
                    "Using trustiflux API server for measurement: {}",
                    base_url
                );
                let http_client = reqwest::Client::builder()
                    .user_agent("measurement-tool/0.1.0")
                    .build()
                    .map_err(|e| {
                        MeasurementError::Http(format!("Failed to build HTTP client: {}", e))
                    })?;
                Ok(Self {
                    inner: ClientImpl::Http {
                        http_client,
                        base_url,
                    },
                })
            }
        }
    }

    pub async fn extend_runtime_measurement(
        &self,
        pcr_index_opt: Option<u64>,
        domain: &str,
        operation: &str,
        content: &str,
    ) -> Result<()> {
        match &self.inner {
            ClientImpl::Ttrpc(client) => {
                debug!(
                    "Extending runtime measurement via ttrpc: pcr_opt={:?}, domain={}, op={}, content={}",
                    pcr_index_opt, domain, operation, content
                );
                let mut req = ExtendRuntimeMeasurementRequest::new();
                req.Domain = domain.to_string();
                req.Operation = operation.to_string();
                req.Content = content.to_string();
                if let Some(pcr_index) = pcr_index_opt {
                    req.RegisterIndex = Some(pcr_index);
                }

                match client
                    .extend_runtime_measurement(default_ttrpc_context(), &req)
                    .await
                {
                    Ok(_) => {
                        debug!("Successfully extended runtime measurement via ttrpc.");
                        Ok(())
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to extend runtime measurement: {}", e);
                        log::error!("{}", err_msg);
                        Err(MeasurementError::AttestationAgentClient(e))
                    }
                }
            }
            ClientImpl::Http {
                http_client,
                base_url,
            } => {
                let url = format!("{}/aa/aael", base_url.trim_end_matches('/'));
                let payload = HttpAaelRequest {
                    domain,
                    operation,
                    content,
                    register_index: pcr_index_opt,
                };
                debug!(
                    "Extending runtime measurement via HTTP {} with domain={}, op={}",
                    url, domain, operation
                );
                let resp = http_client
                    .post(&url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| {
                        MeasurementError::Http(format!(
                            "HTTP request to {} failed: {}",
                            url,
                            e.to_string()
                        ))
                    })?;
                if resp.status().is_success() {
                    debug!("Successfully extended runtime measurement via HTTP.");
                    return Ok(());
                }
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(MeasurementError::Http(format!(
                    "HTTP {} returned status {}: {}",
                    url,
                    status,
                    body
                )))
            }
        }
    }
}

fn default_ttrpc_context() -> ttrpc::context::Context {
    ttrpc::context::Context {
        timeout_nano: 5_000_000_000,
        ..Default::default()
    }
}
