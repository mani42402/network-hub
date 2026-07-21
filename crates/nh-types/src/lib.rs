//! # nh-types — NetworkHub's shared domain vocabulary
//!
//! The foundation crate. It defines the words every other crate speaks —
//! entity identifiers, the error contract, the capability model, network
//! primitives, and units — and it depends on **no other NetworkHub crate**, so
//! everything else can depend on it freely (the layer rule in ARCHITECTURE.md
//! §5.1, enforced by `cargo xtask check-layers`).
//!
//! What lives here is only the *vocabulary*: types, their parsing, validation,
//! display, and serialization. Behaviour — probing capabilities, reconciling
//! state, talking to the OS — lives in the crates that own it. Keeping this crate
//! behaviour-free is what lets it stay a dependency-light leaf.
//!
//! ## Contents
//!
//! - [`Secret`] — a wrapper that makes leaking a secret a compile error.
//! - [`ids`] — strongly-typed UUIDv7 entity identifiers.
//! - [`error`] — the [`DomainError`] contract that crosses the API boundary.
//! - [`capability`] — the [`Capability`] model behind AD-6.
//! - [`net`] — [`MacAddr`] and [`Ssid`], the primitives `std::net` does not give
//!   us.
//! - [`units`] — [`ByteCount`] and [`BitRate`], formatted in the right unit
//!   family for each.

pub mod capability;
pub mod error;
pub mod ids;
pub mod net;
mod secret;
pub mod units;

// Re-export the most-used types at the crate root so callers write
// `nh_types::Secret` and `nh_types::DomainError`, not the full module path.
pub use capability::{Capability, CapabilityMatrix, CapabilityStatus};
pub use error::{DomainError, ErrorCode};
pub use net::{MacAddr, Ssid};
pub use secret::Secret;
pub use units::{BitRate, ByteCount};
