//! gRPC service implementations.
//!
//! For WP-M0.6 this is [`system::SystemServiceImpl`] — version negotiation,
//! health, and the capability matrix — the first service any client calls.
//! Feature services (devices, VPN, hotspot, monitor) land in their milestones.

pub mod system;
