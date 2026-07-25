//! Hard caps on imported profiles, checked before parsing so hostile input
//! cannot exhaust memory (E-15, §10.7). A real `.conf`/`.ovpn` is a few KiB; a
//! megabyte of it is an attack, not a config.

use crate::error::{VpnError, VpnResult};

/// Largest accepted profile, in bytes. Generous for a legitimate `.ovpn` with
/// inline CA + cert + key blocks (a handful of KiB each), hostile past that.
pub const MAX_BYTES: usize = 256 * 1024;

/// Largest accepted line length, in bytes. Catches a single absurd line (a
/// pasted blob with no newlines) before it is split or scanned.
pub const MAX_LINE_BYTES: usize = 16 * 1024;

/// Most `[Peer]` sections (WireGuard) accepted in one profile.
pub const MAX_PEERS: usize = 64;

/// Most `<...>` inline blocks (OpenVPN) accepted in one profile.
pub const MAX_INLINE_BLOCKS: usize = 16;

/// Rejects input larger than [`MAX_BYTES`] before any further work.
///
/// # Errors
/// [`VpnError::TooLarge`] if `input` exceeds the byte cap.
pub fn check_size(input: &[u8]) -> VpnResult<()> {
    if input.len() > MAX_BYTES {
        return Err(VpnError::TooLarge {
            what: "bytes",
            actual: input.len(),
            limit: MAX_BYTES,
        });
    }
    Ok(())
}

/// Decodes `input` as UTF-8 and rejects control characters other than tab and
/// newline — an encoding trick (NULs, embedded escapes) is not a config (F-16).
///
/// # Errors
/// [`VpnError::Encoding`] if the bytes are not UTF-8 or carry stray control
/// characters; [`VpnError::TooLarge`] if any single line exceeds
/// [`MAX_LINE_BYTES`].
pub fn decode_text(input: &[u8]) -> VpnResult<&str> {
    let text = std::str::from_utf8(input).map_err(|_| VpnError::Encoding("not valid UTF-8"))?;

    for ch in text.chars() {
        // Allow tab, LF, CR; reject every other C0/C1 control and the NUL byte.
        if ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r' {
            return Err(VpnError::Encoding("contains control characters"));
        }
    }

    for line in text.lines() {
        if line.len() > MAX_LINE_BYTES {
            return Err(VpnError::TooLarge {
                what: "line",
                actual: line.len(),
                limit: MAX_LINE_BYTES,
            });
        }
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_input() {
        let big = vec![b'a'; MAX_BYTES + 1];
        assert!(matches!(
            check_size(&big),
            Err(VpnError::TooLarge { what: "bytes", .. })
        ));
    }

    #[test]
    fn accepts_normal_size() {
        assert!(check_size(b"[Interface]\n").is_ok());
    }

    #[test]
    fn rejects_non_utf8() {
        assert!(matches!(
            decode_text(&[0xff, 0xfe]),
            Err(VpnError::Encoding(_))
        ));
    }

    #[test]
    fn rejects_embedded_nul() {
        assert!(matches!(
            decode_text(b"[Interface]\0PrivateKey"),
            Err(VpnError::Encoding(_))
        ));
    }

    #[test]
    fn allows_tabs_and_newlines() {
        assert!(decode_text(b"a = b\n\tc = d\r\n").is_ok());
    }

    #[test]
    fn rejects_absurd_single_line() {
        let line = vec![b'x'; MAX_LINE_BYTES + 1];
        assert!(matches!(
            decode_text(&line),
            Err(VpnError::TooLarge { what: "line", .. })
        ));
    }
}
