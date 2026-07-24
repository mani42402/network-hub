//! The error taxonomy (§6.3).
//!
//! Every failure that can cross the API boundary is a [`DomainError`]: a
//! canonical [`ErrorCode`] (so clients can branch on the kind without parsing
//! prose), a machine-readable `reason` slug (stable across releases and
//! translations), a human `detail` sentence, and an optional `remediation` —
//! the sentence the UI shows the user telling them what to do next.
//!
//! This structure is what makes the product's Definition of Done rule real:
//! "every user-triggerable failure has a human sentence, not a stack trace"
//! (CONTRIBUTING.md). An error without a `detail` is a bug; an error the user
//! can fix without a `remediation` is a missed opportunity.

use serde::{Deserialize, Serialize};

/// A canonical, stable classification of a failure. Deliberately small: clients
/// switch on this to decide behaviour (retry, re-auth, show a fixable hint),
/// while `reason`/`detail` carry the specifics.
///
/// The set is modelled on the well-understood gRPC/absl status codes, narrowed
/// to what this domain actually produces, so the mapping to gRPC status at the
/// transport layer is mechanical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The request was malformed or a value was out of range.
    InvalidArgument,
    /// The named entity does not exist.
    NotFound,
    /// An entity that must be unique already exists.
    AlreadyExists,
    /// The caller is not authenticated (no valid identity).
    Unauthenticated,
    /// The caller is authenticated but lacks permission for this operation.
    PermissionDenied,
    /// The system is not in a state where this operation is valid (e.g. "stop
    /// the hotspot before changing its band").
    FailedPrecondition,
    /// A concurrent change conflicts with this one (e.g. editing a stale
    /// firewall revision).
    Conflict,
    /// A limit was hit — DHCP pool exhausted, rule count cap, rate limit.
    ResourceExhausted,
    /// The feature is not available on this machine. Pairs with the capability
    /// model (AD-6): the `remediation` should point at why, from the matrix.
    NotSupported,
    /// A dependency is temporarily unavailable; the operation may succeed later.
    /// The one code clients should consider retrying.
    Unavailable,
    /// The operation did not complete in time.
    Timeout,
    /// An unexpected internal failure. Carries no sensitive detail; the specifics
    /// go to the log with a diagnostic id, not to the client.
    Internal,
}

impl ErrorCode {
    /// Whether an operation failing with this code might succeed on retry
    /// without the caller changing anything. Used by clients to decide whether
    /// to auto-retry versus surface the error.
    #[must_use]
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::Unavailable | Self::Timeout)
    }

    /// A stable, lower-case identifier for logs and metrics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::NotFound => "not_found",
            Self::AlreadyExists => "already_exists",
            Self::Unauthenticated => "unauthenticated",
            Self::PermissionDenied => "permission_denied",
            Self::FailedPrecondition => "failed_precondition",
            Self::Conflict => "conflict",
            Self::ResourceExhausted => "resource_exhausted",
            Self::NotSupported => "not_supported",
            Self::Unavailable => "unavailable",
            Self::Timeout => "timeout",
            Self::Internal => "internal",
        }
    }
}

/// A structured, user-facing error that can cross the API boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainError {
    /// Canonical kind, for programmatic handling.
    pub code: ErrorCode,
    /// A stable machine-readable slug identifying the specific error, e.g.
    /// `hotspot.subnet_conflict`. Stable across releases and independent of
    /// translation, so clients and tests can match on it.
    pub reason: String,
    /// A human sentence describing what went wrong. Always present.
    pub detail: String,
    /// An optional human sentence telling the user how to fix it. The UI renders
    /// this directly (§6.3): "Port 53 is in use by dnsmasq (pid 412). Stop it or
    /// choose another interface."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

impl DomainError {
    /// Builds an error with a code, machine reason, and human detail.
    pub fn new(code: ErrorCode, reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
            detail: detail.into(),
            remediation: None,
        }
    }

    /// Attaches remediation guidance (builder style).
    #[must_use]
    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    /// Whether retrying unchanged might succeed.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.code.is_retryable()
    }

    // Constructors for the common codes, so call sites read as
    // `DomainError::not_found("device.unknown", "No device with that id")`.

    /// A [`ErrorCode::NotFound`] error.
    pub fn not_found(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, reason, detail)
    }

    /// An [`ErrorCode::InvalidArgument`] error.
    pub fn invalid_argument(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidArgument, reason, detail)
    }

    /// A [`ErrorCode::PermissionDenied`] error.
    pub fn permission_denied(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::PermissionDenied, reason, detail)
    }

    /// A [`ErrorCode::NotSupported`] error — pair with a capability reason.
    pub fn not_supported(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotSupported, reason, detail)
    }

    /// A [`ErrorCode::FailedPrecondition`] error.
    pub fn failed_precondition(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::FailedPrecondition, reason, detail)
    }

    /// An [`ErrorCode::Internal`] error. The detail should be generic; specifics
    /// belong in the log, not in a message that reaches an untrusted client.
    pub fn internal(reason: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, reason, detail)
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `[code/reason] detail` — the log form. Remediation is for the UI, not
        // the log line, so it is not included here.
        write!(
            f,
            "[{}/{}] {}",
            self.code.as_str(),
            self.reason,
            self.detail
        )
    }
}

impl std::error::Error for DomainError {}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn display_is_log_shaped() {
        let e = DomainError::not_found("device.unknown", "No device with id abc");
        assert_eq!(
            e.to_string(),
            "[not_found/device.unknown] No device with id abc"
        );
    }

    #[test]
    fn retryable_only_for_transient_codes() {
        assert!(ErrorCode::Unavailable.is_retryable());
        assert!(ErrorCode::Timeout.is_retryable());
        assert!(!ErrorCode::InvalidArgument.is_retryable());
        assert!(!ErrorCode::PermissionDenied.is_retryable());
    }

    #[test]
    fn remediation_round_trips_and_is_optional() {
        let with = DomainError::failed_precondition(
            "dhcp.port_in_use",
            "Port 67 is already in use on the hotspot interface.",
        )
        .with_remediation("Stop the other DHCP server, or pick a different interface.");
        let json = serde_json::to_string(&with).unwrap();
        assert!(json.contains("remediation"));
        let back: DomainError = serde_json::from_str(&json).unwrap();
        assert_eq!(with, back);

        // No remediation → the field is omitted from the wire form.
        let without = DomainError::internal("db.write", "Could not persist state.");
        let json = serde_json::to_string(&without).unwrap();
        assert!(!json.contains("remediation"));
    }

    #[test]
    fn code_serializes_snake_case() {
        let e = DomainError::not_supported(
            "hotspot.macos",
            "Programmatic hotspot is not available on macOS.",
        );
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"not_supported\""));
    }

    #[test]
    fn is_usable_as_std_error() {
        fn takes_error(_: &dyn std::error::Error) {}
        takes_error(&DomainError::internal("x", "y"));
    }
}
