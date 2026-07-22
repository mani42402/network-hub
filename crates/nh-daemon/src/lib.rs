//! # nh-daemon — the privileged system service
//!
//! The composition root (AD-1): where the domain vocabulary, contracts, PAL, and
//! state engine are wired into a running daemon. It owns the process lifecycle
//! (§4.2), the local IPC surface with peer-credential authentication and per-RPC
//! authorization (§10.3), the audit log, structured logging with secret
//! redaction (§15), and child-process supervision (F-4).
//!
//! For WP-M0.6 the daemon runs as a plain process; WP-M0.7 makes it a system
//! service with boot-time cleanup.

pub mod authz;
pub mod capabilities;
pub mod child;
pub mod ipc;
pub mod lifecycle;
pub mod logging;
pub mod principal;
pub mod services;
mod singleton;

pub use lifecycle::{Daemon, DaemonConfig};
pub use principal::{Permission, Principal, Role};
