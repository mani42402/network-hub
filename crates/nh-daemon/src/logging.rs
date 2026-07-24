//! Structured logging with secret redaction (§15).
//!
//! The primary guarantee that secrets never reach a log is the compile-time one:
//! secret material is typed `nh_types::Secret<T>`, which cannot be `Display`ed
//! (WP-M0.1). This module is the **belt-and-braces** second line: a scrub pass
//! over formatted log lines that catches sensitive-looking plaintext that was
//! never wrapped in the first place (a raw passphrase in an imported config, a
//! key echoed from a child process). It is not a substitute for `Secret<T>` —
//! it is the safety net under it.

use std::sync::LazyLock;

use regex::Regex;

/// Patterns whose matched *value* is replaced with `<redacted>`. Kept
/// deliberately broad: over-redacting a log line is a minor annoyance; leaking a
/// key is not.
static REDACTIONS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // key=VALUE / password: VALUE / token = VALUE, for the common secret
        // field names, capturing the label so it stays but the value goes.
        Regex::new(
            r"(?i)\b(password|passphrase|psk|secret|token|api[_-]?key|private[_-]?key)\b\s*[:=]\s*\S+",
        )
        .expect("valid redaction regex"),
        // A bare WireGuard-style key: 43 base64 chars followed by '=' (the base64
        // of 32 bytes). Distinctive enough to redact on sight even without a
        // label. No word boundaries — '=' is not a word char, so a trailing \b
        // would never match, and over-redaction here is acceptable.
        Regex::new(r"[A-Za-z0-9+/]{43}=").expect("valid redaction regex"),
    ]
});

/// Returns `line` with any secret-looking substrings replaced by `<redacted>`.
#[must_use]
pub fn redact_secrets(line: &str) -> String {
    let mut out = line.to_owned();
    for (i, re) in REDACTIONS.iter().enumerate() {
        out = if i == 0 {
            // Keep the label, redact the value.
            re.replace_all(&out, |caps: &regex::Captures| {
                format!("{}=<redacted>", &caps[1])
            })
            .into_owned()
        } else {
            re.replace_all(&out, "<redacted>").into_owned()
        };
    }
    out
}

/// Initializes the global tracing subscriber.
///
/// Uses a JSON formatter when `json` is set (production, machine-parseable), or a
/// compact human format otherwise (development). The level is read from the
/// `NH_LOG` environment variable, defaulting to `info`.
///
/// Redaction is applied by typing secrets as `Secret<T>`; [`redact_secrets`] is
/// available for any place that must format untrusted text into a log line.
///
/// # Errors
/// Returns an error string if a global subscriber is already installed.
pub fn init(json: bool) -> Result<(), String> {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::prelude::*;

    let filter = EnvFilter::try_from_env("NH_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);
    if json {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .try_init()
            .map_err(|e| e.to_string())
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().compact())
            .try_init()
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_labeled_secrets_keeps_label() {
        assert_eq!(redact_secrets("password=hunter2"), "password=<redacted>");
        assert_eq!(redact_secrets("PSK: s3cr3tvalue"), "PSK=<redacted>");
        assert_eq!(
            redact_secrets("connecting with token = abc.def.ghi"),
            "connecting with token=<redacted>"
        );
        assert_eq!(
            redact_secrets("api_key=AKIA1234567890"),
            "api_key=<redacted>"
        );
    }

    #[test]
    fn redacts_bare_wireguard_key() {
        // A 44-char base64 key (43 base64 chars + '=') is redacted even without
        // a label. That is the encoding of a 32-byte WireGuard key.
        let key = "aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789+/AbCdE=";
        assert_eq!(key.len(), 44, "43 base64 chars + '=' = a 32-byte WG key");
        let line = format!("peer public key {key} added");
        let out = redact_secrets(&line);
        assert!(!out.contains(key), "the key must be gone: {out}");
        assert!(out.contains("<redacted>"));
    }

    #[test]
    fn leaves_ordinary_text_untouched() {
        let line = "hotspot started on interface wlan0 with 3 clients";
        assert_eq!(redact_secrets(line), line);
    }

    #[test]
    fn case_insensitive_labels() {
        assert_eq!(redact_secrets("PASSWORD=x"), "PASSWORD=<redacted>");
        assert_eq!(redact_secrets("Passphrase=y"), "Passphrase=<redacted>");
    }
}
