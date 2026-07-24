//! The CLI commands, each producing a serializable report so the same value
//! feeds both the human and the `--json` renderers, and so tests can assert on
//! it directly.

use std::collections::BTreeMap;
use std::path::Path;

use nh_proto::v1::{
    CapabilityStatus, GetCapabilitiesRequest, GetHealthRequest, GetVersionRequest, ServingStatus,
};
use serde::Serialize;

use crate::{CliError, Client, connect};

/// `status`: daemon version and per-module health.
#[derive(Debug, Serialize)]
pub struct StatusReport {
    /// The daemon's own version.
    pub daemon_version: String,
    /// The API contract version served.
    pub api_version: String,
    /// The oldest client the daemon will serve.
    pub min_client_version: String,
    /// The OS the daemon runs on.
    pub os: String,
    /// Overall serving status.
    pub overall: String,
    /// Per-module health.
    pub modules: Vec<ModuleReport>,
}

/// One module's health.
#[derive(Debug, Serialize)]
pub struct ModuleReport {
    /// Module name.
    pub name: String,
    /// Serving status.
    pub status: String,
    /// Optional detail.
    pub detail: String,
}

/// `capabilities`: the full §3.4 matrix.
#[derive(Debug, Serialize)]
pub struct CapabilitiesReport {
    /// Capability key → verdict.
    pub capabilities: BTreeMap<String, CapReport>,
}

/// One capability's verdict.
#[derive(Debug, Serialize)]
pub struct CapReport {
    /// supported / degraded / unsupported / unknown.
    pub status: String,
    /// Human-readable reason.
    pub reason: String,
    /// Structured specifics.
    pub details: BTreeMap<String, String>,
}

/// `doctor`: a set of pass/fail diagnostic checks.
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    /// True iff every check passed.
    pub ok: bool,
    /// The individual checks.
    pub checks: Vec<CheckReport>,
}

/// One diagnostic check.
#[derive(Debug, Serialize)]
pub struct CheckReport {
    /// What was checked.
    pub name: String,
    /// Whether it passed.
    pub ok: bool,
    /// Explanation.
    pub detail: String,
}

/// `debug tasks`: the daemon's background work. In M0 there is no task-listing
/// RPC yet, so this reports module health as the closest available view.
#[derive(Debug, Serialize)]
pub struct TasksReport {
    /// A note about the M0 scope of this view.
    pub note: String,
    /// The modules currently reporting health.
    pub modules: Vec<ModuleReport>,
}

/// Runs `status`.
///
/// # Errors
/// Returns [`CliError`] if a daemon call fails.
pub async fn status(client: &mut Client) -> Result<StatusReport, CliError> {
    let version = client.get_version(GetVersionRequest {}).await?.into_inner();
    let health = client.get_health(GetHealthRequest {}).await?.into_inner();
    Ok(StatusReport {
        daemon_version: version.daemon_version,
        api_version: version.api_version,
        min_client_version: version.min_client_version,
        os: version.os,
        overall: serving_name(health.overall).to_owned(),
        modules: health.modules.into_iter().map(module_report).collect(),
    })
}

/// Runs `capabilities`.
///
/// # Errors
/// Returns [`CliError`] if the daemon call fails.
pub async fn capabilities(client: &mut Client) -> Result<CapabilitiesReport, CliError> {
    let resp = client
        .get_capabilities(GetCapabilitiesRequest {})
        .await?
        .into_inner();
    let capabilities = resp
        .capabilities
        .into_iter()
        .map(|(key, cap)| {
            (
                key,
                CapReport {
                    status: capability_name(cap.status).to_owned(),
                    reason: cap.reason,
                    details: cap.details.into_iter().collect(),
                },
            )
        })
        .collect();
    Ok(CapabilitiesReport { capabilities })
}

/// Runs `debug tasks`.
///
/// # Errors
/// Returns [`CliError`] if the daemon call fails.
pub async fn debug_tasks(client: &mut Client) -> Result<TasksReport, CliError> {
    let health = client.get_health(GetHealthRequest {}).await?.into_inner();
    Ok(TasksReport {
        note: "module health (full task introspection lands in a later milestone)".to_owned(),
        modules: health.modules.into_iter().map(module_report).collect(),
    })
}

/// Runs `doctor`, connecting itself so an unreachable daemon is a failed check
/// rather than an error.
pub async fn doctor(dir: &Path) -> DoctorReport {
    let mut checks = Vec::new();

    let mut client = match connect(dir).await {
        Ok(client) => {
            checks.push(CheckReport {
                name: "daemon reachable".to_owned(),
                ok: true,
                detail: format!("connected via {}", dir.display()),
            });
            client
        }
        Err(e) => {
            checks.push(CheckReport {
                name: "daemon reachable".to_owned(),
                ok: false,
                detail: e.to_string(),
            });
            return DoctorReport { ok: false, checks };
        }
    };

    match client.get_version(GetVersionRequest {}).await {
        Ok(v) => {
            let v = v.into_inner();
            checks.push(CheckReport {
                name: "api version".to_owned(),
                ok: v.api_version == "v1",
                detail: format!("daemon {} speaks api {}", v.daemon_version, v.api_version),
            });
        }
        Err(e) => checks.push(failed("api version", &e)),
    }

    match client.get_capabilities(GetCapabilitiesRequest {}).await {
        Ok(c) => {
            let n = c.into_inner().capabilities.len();
            checks.push(CheckReport {
                name: "capabilities probed".to_owned(),
                ok: n > 0,
                detail: format!("{n} capabilities reported"),
            });
        }
        Err(e) => checks.push(failed("capabilities probed", &e)),
    }

    match client.get_health(GetHealthRequest {}).await {
        Ok(h) => {
            let h = h.into_inner();
            checks.push(CheckReport {
                name: "serving".to_owned(),
                ok: h.overall == ServingStatus::Serving as i32,
                detail: serving_name(h.overall).to_owned(),
            });
        }
        Err(e) => checks.push(failed("serving", &e)),
    }

    let ok = checks.iter().all(|c| c.ok);
    DoctorReport { ok, checks }
}

fn failed(name: &str, e: &tonic::Status) -> CheckReport {
    CheckReport {
        name: name.to_owned(),
        ok: false,
        detail: e.to_string(),
    }
}

fn module_report(m: nh_proto::v1::ModuleHealth) -> ModuleReport {
    ModuleReport {
        name: m.name,
        status: serving_name(m.status).to_owned(),
        detail: m.detail,
    }
}

fn serving_name(value: i32) -> &'static str {
    match ServingStatus::try_from(value) {
        Ok(ServingStatus::Serving) => "serving",
        Ok(ServingStatus::Degraded) => "degraded",
        Ok(ServingStatus::NotServing) => "not_serving",
        _ => "unspecified",
    }
}

fn capability_name(value: i32) -> &'static str {
    match CapabilityStatus::try_from(value) {
        Ok(CapabilityStatus::Supported) => "supported",
        Ok(CapabilityStatus::Degraded) => "degraded",
        Ok(CapabilityStatus::Unsupported) => "unsupported",
        Ok(CapabilityStatus::Unknown) => "unknown",
        _ => "unspecified",
    }
}
