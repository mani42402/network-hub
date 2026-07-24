//! Rendering command reports as human-readable text or JSON.

use serde::Serialize;

use crate::CliError;
use crate::commands::{CapabilitiesReport, DoctorReport, StatusReport, TasksReport};

/// Prints a report as pretty JSON.
///
/// # Errors
/// Returns [`CliError::Output`] if serialization fails.
pub fn print_json<T: Serialize>(report: &T) -> Result<(), CliError> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

/// Human rendering of `status`.
#[must_use]
pub fn render_status(r: &StatusReport) -> String {
    let mut lines = vec![
        format!(
            "daemon {} (api {}) on {}",
            r.daemon_version, r.api_version, r.os
        ),
        format!("overall: {}", r.overall),
    ];
    if r.modules.is_empty() {
        lines.push("modules: (none)".to_owned());
    } else {
        lines.push("modules:".to_owned());
        for m in &r.modules {
            let detail = if m.detail.is_empty() {
                String::new()
            } else {
                format!(" — {}", m.detail)
            };
            lines.push(format!("  {:<12} {}{}", m.name, m.status, detail));
        }
    }
    lines.join("\n")
}

/// Human rendering of `capabilities`.
#[must_use]
pub fn render_capabilities(r: &CapabilitiesReport) -> String {
    let mut lines = Vec::with_capacity(r.capabilities.len());
    for (key, cap) in &r.capabilities {
        lines.push(format!("{key:<26} {:<12} {}", cap.status, cap.reason));
    }
    if lines.is_empty() {
        "(no capabilities reported)".to_owned()
    } else {
        lines.join("\n")
    }
}

/// Human rendering of `doctor`.
#[must_use]
pub fn render_doctor(r: &DoctorReport) -> String {
    let mut lines: Vec<String> = r
        .checks
        .iter()
        .map(|c| {
            let mark = if c.ok { "ok  " } else { "FAIL" };
            format!("[{mark}] {:<20} {}", c.name, c.detail)
        })
        .collect();
    lines.push(if r.ok {
        "doctor: all checks passed".to_owned()
    } else {
        "doctor: problems found".to_owned()
    });
    lines.join("\n")
}

/// Human rendering of `debug tasks`.
#[must_use]
pub fn render_tasks(r: &TasksReport) -> String {
    let mut lines = vec![format!("note: {}", r.note)];
    for m in &r.modules {
        lines.push(format!("  {:<12} {}", m.name, m.status));
    }
    lines.join("\n")
}
