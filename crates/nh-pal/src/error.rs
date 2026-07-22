//! Errors from platform operations.
//!
//! A `PalError` is what any backend returns when an OS operation fails. It is
//! deliberately close to the domain error taxonomy (`nh_types::DomainError`) so
//! the daemon can map a platform failure to a user-facing error with a
//! `remediation` without inventing a second vocabulary.

use nh_types::{DomainError, ErrorCode};

/// The result type every fallible PAL operation returns.
pub type PalResult<T> = Result<T, PalError>;

/// A failure from a platform adapter operation.
#[derive(Debug, thiserror::Error)]
pub enum PalError {
    /// The operation is not supported on this platform or hardware. Pairs with
    /// the capability model (AD-6): the daemon should have consulted the
    /// capability matrix first, so this is a backstop, and the message names the
    /// reason.
    #[error("not supported: {0}")]
    NotSupported(String),

    /// The caller asked for something invalid — a bad address, an out-of-range
    /// channel, a malformed rule.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The system is not in a state where the operation is valid (e.g. starting
    /// an access point on an interface that is already an AP).
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// Another component holds the resource, or our change conflicts with
    /// external state (§5.3 interference).
    #[error("conflict: {0}")]
    Conflict(String),

    /// The operation lacked the OS privilege it needed. In normal operation the
    /// daemon runs privileged, so this signals a misconfigured install or a
    /// dropped capability rather than user error.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// An underlying OS/IO error, carried for logs and diagnostics.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A platform-specific failure that does not fit the categories above. The
    /// string is for logs; it must never contain a secret.
    #[error("platform error: {0}")]
    Platform(String),
}

impl PalError {
    /// Maps this platform error to the canonical [`ErrorCode`] the API uses.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotSupported(_) => ErrorCode::NotSupported,
            Self::InvalidArgument(_) => ErrorCode::InvalidArgument,
            Self::InvalidState(_) => ErrorCode::FailedPrecondition,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::PermissionDenied(_) => ErrorCode::PermissionDenied,
            Self::Io(_) | Self::Platform(_) => ErrorCode::Internal,
        }
    }
}

impl From<PalError> for DomainError {
    /// Converts a platform error into a user-facing domain error. The daemon can
    /// then attach a `remediation` appropriate to the operation before returning
    /// it to a client.
    fn from(err: PalError) -> Self {
        // The reason slug is coarse here; the daemon refines it per operation.
        DomainError::new(err.code(), "pal.operation_failed", err.to_string())
    }
}
