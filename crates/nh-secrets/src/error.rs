//! Errors from secret storage and sealing.

/// The result type for secret operations.
pub type SecretsResult<T> = Result<T, SecretsError>;

/// A failure storing, loading, or sealing a secret.
///
/// Deliberately coarse and message-only: an error here must never echo the
/// secret material it was handling.
#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    /// The OS keychain (or file fallback) failed.
    #[error("keyring backend error: {0}")]
    Keyring(String),
    /// Sealing or opening failed — a wrong key, a truncated blob, or tampering
    /// (the GCM authentication tag did not verify).
    #[error("cryptographic operation failed: {0}")]
    Crypto(String),
    /// The system random number generator failed.
    #[error("random generation failed: {0}")]
    Rng(String),
    /// A stored value was not the shape we expected (e.g. a master key of the
    /// wrong length).
    #[error("stored secret is corrupt: {0}")]
    Corrupt(String),
    /// An underlying IO error (the file keyring).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
