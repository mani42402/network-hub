//! Network primitives with careful parsing and validation.
//!
//! IP addresses use `std::net::{IpAddr, Ipv4Addr, Ipv6Addr}` directly — the
//! standard library already parses and displays them correctly, including the
//! many IPv6 forms, and reinventing that would only add bugs. This module covers
//! the primitives the standard library does *not*: MAC addresses and SSIDs.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A 48-bit IEEE 802 MAC address.
///
/// Stored as six bytes and displayed canonically as lower-case, colon-separated
/// hex (`a1:b2:c3:d4:e5:f6`). Parsing accepts colon or hyphen separators and is
/// case-insensitive, because both forms appear in the wild (Windows tools use
/// hyphens, most others use colons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    /// Constructs from raw octets.
    #[must_use]
    pub const fn new(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    /// The six octets, in order.
    #[must_use]
    pub const fn octets(&self) -> [u8; 6] {
        self.0
    }

    /// Whether the locally-administered bit is set. Randomized MAC addresses —
    /// which iOS, Android, and Windows 11 use by default — set this bit, so it
    /// is a useful (if not definitive) hint that an address is not a hardware
    /// address. Device identity must not rely on the MAC alone (§7.2).
    #[must_use]
    pub const fn is_locally_administered(&self) -> bool {
        self.0[0] & 0b0000_0010 != 0
    }

    /// Whether this is a multicast/group address (the low bit of the first
    /// octet). Unicast client addresses never have it set.
    #[must_use]
    pub const fn is_multicast(&self) -> bool {
        self.0[0] & 0b0000_0001 != 0
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            o[0], o[1], o[2], o[3], o[4], o[5]
        )
    }
}

/// The ways a MAC address string can be invalid, kept specific so error
/// messages can be actionable rather than a generic "parse error".
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MacParseError {
    /// Not exactly six groups separated by `:` or `-`.
    #[error("expected six colon- or hyphen-separated octets, found {found}")]
    WrongGroupCount {
        /// How many groups were actually found.
        found: usize,
    },
    /// A group was not a valid two-digit hex octet.
    #[error("group {index} ({group:?}) is not a two-digit hex octet")]
    InvalidOctet {
        /// Zero-based position of the offending group.
        index: usize,
        /// The offending group's text, for the error message.
        group: String,
    },
    /// Separators were mixed (e.g. `a1:b2-c3`), which usually signals a typo.
    #[error("separators are mixed; use all colons or all hyphens")]
    MixedSeparators,
}

impl FromStr for MacAddr {
    type Err = MacParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sep = if s.contains(':') {
            if s.contains('-') {
                return Err(MacParseError::MixedSeparators);
            }
            ':'
        } else {
            '-'
        };

        let groups: Vec<&str> = s.split(sep).collect();
        if groups.len() != 6 {
            return Err(MacParseError::WrongGroupCount {
                found: groups.len(),
            });
        }

        let mut octets = [0u8; 6];
        for (index, group) in groups.iter().enumerate() {
            if group.len() != 2 {
                return Err(MacParseError::InvalidOctet {
                    index,
                    group: (*group).to_owned(),
                });
            }
            octets[index] =
                u8::from_str_radix(group, 16).map_err(|_| MacParseError::InvalidOctet {
                    index,
                    group: (*group).to_owned(),
                })?;
        }
        Ok(Self(octets))
    }
}

impl Serialize for MacAddr {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MacAddr {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A Wi-Fi network name (SSID).
///
/// The subtle, security-relevant rule (edge case E-7): the 802.11 SSID limit is
/// **32 bytes, not 32 characters.** A name of emoji or non-Latin script can be
/// far fewer than 32 characters yet exceed 32 bytes, and hardware that silently
/// truncates an over-long SSID produces a network with a different name than the
/// user typed — confusing at best, a spoofing vector at worst. So validation is
/// on the UTF-8 byte length.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Ssid(String);

/// The 802.11 maximum SSID length, in bytes.
pub const SSID_MAX_BYTES: usize = 32;

impl Ssid {
    /// Validates and constructs an SSID, checking the **byte** length (E-7).
    ///
    /// # Errors
    /// Returns [`SsidError`] if the name is empty or exceeds 32 UTF-8 bytes.
    pub fn new(name: impl Into<String>) -> Result<Self, SsidError> {
        let name = name.into();
        if name.is_empty() {
            return Err(SsidError::Empty);
        }
        let bytes = name.len(); // String::len is the UTF-8 byte length — the point.
        if bytes > SSID_MAX_BYTES {
            return Err(SsidError::TooLong { bytes });
        }
        Ok(Self(name))
    }

    /// The SSID as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The SSID's length in bytes (what the hardware limit applies to).
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Display for Ssid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why an SSID was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SsidError {
    /// An empty SSID is a hidden network, which we do not create by this path.
    #[error("SSID must not be empty")]
    Empty,
    /// Longer than the 32-byte 802.11 limit. Reports bytes, since the byte count
    /// is what overflowed — the character count may be well under 32.
    #[error("SSID is {bytes} bytes; the 802.11 limit is {SSID_MAX_BYTES} bytes")]
    TooLong {
        /// The SSID's actual UTF-8 byte length.
        bytes: usize,
    },
}

impl TryFrom<String> for Ssid {
    type Error = SsidError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Ssid> for String {
    fn from(ssid: Ssid) -> Self {
        ssid.0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn mac_parses_colon_and_hyphen() {
        let a: MacAddr = "a1:b2:c3:d4:e5:f6".parse().unwrap();
        let b: MacAddr = "A1-B2-C3-D4-E5-F6".parse().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.octets(), [0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6]);
    }

    #[test]
    fn mac_displays_lowercase_colon() {
        let m = MacAddr::new([0x0a, 0x1b, 0x2c, 0x3d, 0x4e, 0x5f]);
        assert_eq!(m.to_string(), "0a:1b:2c:3d:4e:5f");
    }

    #[test]
    fn mac_round_trips() {
        let m = MacAddr::new([1, 2, 3, 4, 5, 6]);
        assert_eq!(m, m.to_string().parse().unwrap());
    }

    #[test]
    fn mac_rejects_bad_input() {
        assert_eq!(
            "a1:b2:c3".parse::<MacAddr>(),
            Err(MacParseError::WrongGroupCount { found: 3 })
        );
        assert_eq!(
            "a1:b2:c3-d4:e5:f6".parse::<MacAddr>(),
            Err(MacParseError::MixedSeparators)
        );
        assert!(matches!(
            "a1:b2:c3:d4:e5:zz".parse::<MacAddr>(),
            Err(MacParseError::InvalidOctet { index: 5, .. })
        ));
        assert!(matches!(
            "a1:b2:c3:d4:e5:f".parse::<MacAddr>(),
            Err(MacParseError::InvalidOctet { .. })
        ));
    }

    #[test]
    fn mac_flag_bits() {
        // 0x02 in the first octet = locally administered (randomized MACs).
        assert!(MacAddr::new([0x02, 0, 0, 0, 0, 0]).is_locally_administered());
        assert!(!MacAddr::new([0x00, 0, 0, 0, 0, 0]).is_locally_administered());
        // 0x01 = multicast.
        assert!(MacAddr::new([0x01, 0, 0, 0, 0, 0]).is_multicast());
    }

    #[test]
    fn ssid_accepts_normal_names() {
        assert_eq!(Ssid::new("HomeWiFi").unwrap().as_str(), "HomeWiFi");
    }

    #[test]
    fn ssid_rejects_empty() {
        assert_eq!(Ssid::new(""), Err(SsidError::Empty));
    }

    #[test]
    fn ssid_length_is_bytes_not_chars() {
        // 32 ASCII characters = 32 bytes: allowed (exactly at the limit).
        let thirty_two = "a".repeat(32);
        assert!(Ssid::new(&thirty_two).is_ok());
        assert_eq!(
            Ssid::new("a".repeat(33)),
            Err(SsidError::TooLong { bytes: 33 })
        );

        // E-7: this is where character-based validation would wrongly succeed.
        // Each emoji is 4 UTF-8 bytes, so 9 of them is 36 bytes but only 9
        // characters — under a 32-CHARACTER limit, over the 32-BYTE limit.
        let emoji = "🎉".repeat(9);
        assert_eq!(emoji.chars().count(), 9);
        assert_eq!(emoji.len(), 36);
        assert_eq!(Ssid::new(&emoji), Err(SsidError::TooLong { bytes: 36 }));

        // Eight emoji is exactly 32 bytes: the boundary is allowed.
        let eight = "🎉".repeat(8);
        assert_eq!(eight.len(), 32);
        assert!(Ssid::new(&eight).is_ok());
    }

    #[test]
    fn ssid_serde_round_trip_and_validation() {
        let s = Ssid::new("café-wifi").unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let back: Ssid = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);

        // Deserialization enforces the same byte limit as construction.
        let too_long = format!("\"{}\"", "x".repeat(40));
        assert!(serde_json::from_str::<Ssid>(&too_long).is_err());
    }
}
