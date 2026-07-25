//! Errors from VPN profile parsing, sanitising, and import.
//!
//! Messages are safe to log: a parse error names the *field or directive* at
//! fault, never the secret material a line might have carried.

use nh_types::ErrorCode;

/// The result type for profile operations.
pub type VpnResult<T> = Result<T, VpnError>;

/// A failure importing or parsing a VPN profile.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VpnError {
    /// The input exceeds a hard size cap (bytes, lines, or peers). Rejected
    /// before any parsing to bound memory (E-15).
    #[error("input too large: {what} is {actual}, limit is {limit}")]
    TooLarge {
        /// Which cap was hit ("bytes", "lines", "peers", ...).
        what: &'static str,
        /// The offending measured value.
        actual: usize,
        /// The configured cap.
        limit: usize,
    },

    /// The input is not valid UTF-8, or contains control characters that have no
    /// place in a config (an encoding trick, F-16).
    #[error("input is not clean UTF-8 text: {0}")]
    Encoding(&'static str),

    /// A structural problem: a stray key outside any section, a malformed
    /// `key = value`, an unterminated inline block, etc.
    #[error("malformed profile: {0}")]
    Malformed(String),

    /// A required field is missing.
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// A section that must appear at most once appeared twice.
    #[error("duplicate [{0}] section")]
    DuplicateSection(&'static str),

    /// A key (private/public/preshared) is not valid base64-encoded 32-byte
    /// Curve25519 material. The bad value is never echoed.
    #[error("invalid {0}: not base64-encoded 32-byte key material")]
    InvalidKey(&'static str),

    /// An address or CIDR did not parse.
    #[error("invalid address `{0}`")]
    InvalidAddress(String),

    /// An `endpoint`/`remote` host:port did not parse.
    #[error("invalid endpoint `{0}`")]
    InvalidEndpoint(String),

    /// An integer field (MTU, port, keepalive) was out of range or unparseable.
    #[error("invalid {field} value `{value}`")]
    InvalidNumber {
        /// The field name.
        field: &'static str,
        /// The offending token.
        value: String,
    },

    /// An OpenVPN directive that can execute code as root (`up`, `down`,
    /// `script-security`, `plugin`, ...) was present. The importer rejects the
    /// profile rather than silently dropping it, so the user knows (§8.2).
    #[error("refused dangerous OpenVPN directive `{0}` (can execute code as root)")]
    DangerousDirective(String),

    /// A path in a directive escaped its allowed scope or was absolute where only
    /// a relative inline reference is allowed (path traversal, E-15).
    #[error("refused unsafe path in directive `{0}`")]
    UnsafePath(String),
}

impl VpnError {
    /// The canonical [`ErrorCode`] for this failure, for the gRPC surface (§6.3).
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            VpnError::TooLarge { .. } => ErrorCode::ResourceExhausted,
            VpnError::DangerousDirective(_) | VpnError::UnsafePath(_) => {
                ErrorCode::PermissionDenied
            }
            _ => ErrorCode::InvalidArgument,
        }
    }
}
