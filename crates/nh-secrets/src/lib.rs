//! Platform keychain abstraction and encrypted-blob storage for secrets that
//! must never reach SQLite (AD-10, §7.1).
//!
//! A VPN private key or an OpenVPN passphrase must survive a reboot, must never
//! appear in `config.db` or the logs, and must be unreadable by a non-root user.
//! The strategy (§7.1):
//!
//! - A single 256-bit **master key** lives in the OS keychain — DPAPI
//!   machine-scope on Windows, the system Keychain on macOS, the Secret Service
//!   on a Linux desktop, or a **root-only file** on a headless Linux box (a Pi
//!   with no desktop keyring). The [`Keyring`] trait is that abstraction.
//! - Every secret blob is sealed with **AES-256-GCM** under the master key
//!   ([`SecretStore`]) and only the sealed bytes are stored elsewhere (e.g. in
//!   `config.db`). Losing the database therefore does not leak a single secret.
//! - The master key can be **rotated** without losing access to existing blobs.
//!
//! **Status (WP-M1.1):** the crypto core, the in-memory keyring (tests), and the
//! root-only file keyring (headless Linux) are implemented here — pure, safe
//! Rust, testable on every OS. The DPAPI / Keychain / Secret Service backends
//! (which need platform FFI) land next.

mod error;
mod keyring;
mod store;

pub use error::{SecretsError, SecretsResult};
pub use keyring::{FileKeyring, Keyring, MemoryKeyring};
pub use store::{PriorKey, SecretStore};
