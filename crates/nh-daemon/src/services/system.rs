//! `SystemService` — version negotiation, health, and capabilities.

use std::sync::Arc;

use nh_proto::v1::system_service_server::SystemService;
use nh_proto::v1::{
    Capability as ProtoCapability, CapabilityStatus as ProtoCapStatus, GetCapabilitiesRequest,
    GetCapabilitiesResponse, GetHealthRequest, GetHealthResponse, GetVersionRequest,
    GetVersionResponse, ModuleHealth, ServingStatus,
};
use nh_types::capability::{CapabilityMatrix, CapabilityStatus};
use tonic::{Request, Response, Status};

/// The daemon's own version.
pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The API contract version served.
pub const API_VERSION: &str = "v1";
/// The oldest client this daemon will serve. Bumped only on a breaking change a
/// client must adapt to (ADR-0013).
pub const MIN_CLIENT_VERSION: &str = "0.0.0";

/// Shared daemon state the system service reads.
#[derive(Debug)]
pub struct SystemState {
    /// The capability matrix probed at startup.
    pub capabilities: CapabilityMatrix,
}

/// Implementation of the `SystemService` RPCs.
#[derive(Debug, Clone)]
pub struct SystemServiceImpl {
    state: Arc<SystemState>,
}

impl SystemServiceImpl {
    /// Builds the service over shared daemon state.
    #[must_use]
    pub fn new(state: Arc<SystemState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SystemService for SystemServiceImpl {
    async fn get_version(
        &self,
        _request: Request<GetVersionRequest>,
    ) -> Result<Response<GetVersionResponse>, Status> {
        Ok(Response::new(GetVersionResponse {
            daemon_version: DAEMON_VERSION.to_owned(),
            api_version: API_VERSION.to_owned(),
            min_client_version: MIN_CLIENT_VERSION.to_owned(),
            os: std::env::consts::OS.to_owned(),
        }))
    }

    async fn get_health(
        &self,
        _request: Request<GetHealthRequest>,
    ) -> Result<Response<GetHealthResponse>, Status> {
        // For M0 the daemon reports itself serving with the modules that exist.
        // Real per-module health is wired as modules land (§15).
        Ok(Response::new(GetHealthResponse {
            overall: ServingStatus::Serving as i32,
            modules: vec![ModuleHealth {
                name: "ipc".to_owned(),
                status: ServingStatus::Serving as i32,
                detail: String::new(),
            }],
        }))
    }

    async fn get_capabilities(
        &self,
        _request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        let capabilities = self
            .state
            .capabilities
            .iter()
            .map(|(key, cap)| (key.clone(), to_proto(cap)))
            .collect();
        Ok(Response::new(GetCapabilitiesResponse { capabilities }))
    }
}

/// Converts a domain capability to its proto form.
fn to_proto(cap: &nh_types::capability::Capability) -> ProtoCapability {
    let status = match cap.status {
        CapabilityStatus::Supported => ProtoCapStatus::Supported,
        CapabilityStatus::Degraded => ProtoCapStatus::Degraded,
        CapabilityStatus::Unsupported => ProtoCapStatus::Unsupported,
        CapabilityStatus::Unknown => ProtoCapStatus::Unknown,
    };
    ProtoCapability {
        status: status as i32,
        reason: cap.reason.clone(),
        details: cap.details.clone().into_iter().collect(),
    }
}
