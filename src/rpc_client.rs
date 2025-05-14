// src/rpc_client.rs
use crate::error::{MeasurementError, Result};
use crate::rpc_generated::attestation_agent_ttrpc::AttestationAgentServiceClient;
use crate::rpc_generated::attestation_agent::ExtendRuntimeMeasurementRequest;
use log::{debug, info};
use ttrpc::asynchronous::Client;

pub struct AAClient {
    client: AttestationAgentServiceClient,
}

impl AAClient {
    pub async fn new(socket_addr: &str) -> Result<Self> {
        info!("Connecting to Attestation Agent at: {}", socket_addr);
        let client = Client::connect(socket_addr)
            .map_err(|e| MeasurementError::RpcClient(format!("Failed to connect to AA: {}", e)))?;
        Ok(Self {
            client: AttestationAgentServiceClient::new(client),
        })
    }

    pub async fn extend_runtime_measurement(
        &self,
        pcr_index_opt: Option<u64>,
        domain: &str,
        operation: &str,
        content: &str,
    ) -> Result<()> {
        debug!(
            "Extending runtime measurement: pcr_opt={:?}, domain={}, op={}, content={}",
            pcr_index_opt,
            domain,
            operation,
            content
        );
        let mut req = ExtendRuntimeMeasurementRequest::new();
        req.Domain = domain.to_string();
        req.Operation = operation.to_string();
        req.Content = content.to_string();
        if let Some(pcr_index) = pcr_index_opt {
            req.RegisterIndex = Some(pcr_index);
        }

        match self.client.extend_runtime_measurement(default_ttrpc_context(), &req).await {
            Ok(_) => {
                debug!("Successfully extended runtime measurement.");
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to extend runtime measurement: {}", e);
                log::error!("{}", err_msg);
                Err(MeasurementError::AttestationAgentClient(e))
            }
        }
    }
}

fn default_ttrpc_context() -> ttrpc::context::Context {
    let mut ctx = ttrpc::context::Context::default();
    ctx.timeout_nano = 5_000_000_000; // 5 seconds timeout
    ctx
} 