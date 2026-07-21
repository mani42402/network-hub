//! Data-quantity and rate units, with formatting that does not lie.
//!
//! Two rules this module encodes, because getting them wrong is a common source
//! of confusing UI:
//!
//! - **Bytes are stored, not formatted, internally.** A `ByteCount` is an exact
//!   `u64`; human formatting is a separate, explicit step so arithmetic never
//!   happens on rounded values.
//! - **Data quantities use binary (KiB/MiB, ÷1024); network rates use decimal
//!   (kbit/s, Mbit/s, ÷1000).** That mismatch is not sloppiness — it is how the
//!   two domains are actually defined (a "100 Mbit" link is `100_000_000` bit/s,
//!   while a "1 KiB" file is 1024 bytes). Mixing them is how a UI ends up
//!   off by ~5% and users notice.

use serde::{Deserialize, Serialize};

/// An exact count of bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ByteCount(pub u64);

impl ByteCount {
    /// The raw byte count.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Formats using **binary** units (KiB, MiB, GiB — powers of 1024), the
    /// correct convention for stored data quantities.
    #[must_use]
    pub fn human_binary(self) -> String {
        const UNITS: [&str; 7] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
        format_scaled(self.0, 1024.0, &UNITS)
    }
}

impl std::fmt::Display for ByteCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.human_binary())
    }
}

/// A network throughput, stored as **bits per second** (the unit link speeds are
/// quoted in).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BitRate(pub u64);

impl BitRate {
    /// Constructs a rate from bits per second.
    #[must_use]
    pub const fn from_bits_per_sec(bits: u64) -> Self {
        Self(bits)
    }

    /// Constructs a rate from bytes per second (multiplies by 8).
    #[must_use]
    pub const fn from_bytes_per_sec(bytes: u64) -> Self {
        Self(bytes.saturating_mul(8))
    }

    /// The raw bits-per-second value.
    #[must_use]
    pub const fn bits_per_sec(self) -> u64 {
        self.0
    }

    /// Formats using **decimal** units (kbit/s, Mbit/s — powers of 1000), the
    /// correct convention for network rates.
    #[must_use]
    pub fn human(self) -> String {
        const UNITS: [&str; 6] = ["bit/s", "kbit/s", "Mbit/s", "Gbit/s", "Tbit/s", "Pbit/s"];
        format_scaled(self.0, 1000.0, &UNITS)
    }
}

impl std::fmt::Display for BitRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.human())
    }
}

/// Scales a value down by `base` until it fits a unit, then formats it with a
/// sensible number of decimal places. Shared by both unit families so their
/// formatting stays consistent (integer for the base unit, up to two decimals
/// above it).
fn format_scaled(value: u64, base: f64, units: &[&str]) -> String {
    if value == 0 {
        return format!("0 {}", units[0]);
    }
    #[allow(clippy::cast_precision_loss)] // display only; exactness not required
    let mut scaled = value as f64;
    let mut unit = 0;
    while scaled >= base && unit < units.len() - 1 {
        scaled /= base;
        unit += 1;
    }
    if unit == 0 {
        // Whole base units (bytes, bits) print without a decimal point.
        format!("{value} {}", units[0])
    } else {
        // Trim trailing zeros so 1.50 shows as 1.5 and 2.00 as 2.
        let s = format!("{scaled:.2}");
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        format!("{trimmed} {}", units[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_use_binary_units() {
        assert_eq!(ByteCount(0).human_binary(), "0 B");
        assert_eq!(ByteCount(512).human_binary(), "512 B");
        assert_eq!(ByteCount(1024).human_binary(), "1 KiB");
        assert_eq!(ByteCount(1536).human_binary(), "1.5 KiB");
        assert_eq!(ByteCount(1024 * 1024).human_binary(), "1 MiB");
        assert_eq!(ByteCount(3 * 1024 * 1024 * 1024).human_binary(), "3 GiB");
    }

    #[test]
    fn rates_use_decimal_units() {
        assert_eq!(BitRate(0).human(), "0 bit/s");
        assert_eq!(BitRate(999).human(), "999 bit/s");
        assert_eq!(BitRate(1000).human(), "1 kbit/s");
        assert_eq!(BitRate(1_500_000).human(), "1.5 Mbit/s");
        // A "100 Mbit" link is exactly 100_000_000 bit/s in decimal — the whole
        // reason rates are not binary.
        assert_eq!(BitRate(100_000_000).human(), "100 Mbit/s");
    }

    #[test]
    fn bytes_per_sec_converts_to_bits() {
        assert_eq!(
            BitRate::from_bytes_per_sec(125_000).bits_per_sec(),
            1_000_000
        );
        assert_eq!(BitRate::from_bytes_per_sec(125_000).human(), "1 Mbit/s");
    }

    #[test]
    fn the_two_conventions_differ_as_intended() {
        // 1000 bytes is under 1 KiB (binary), but 1000 bit/s is exactly 1 kbit/s
        // (decimal). Same number, different unit families — the bug this module
        // exists to prevent.
        assert_eq!(ByteCount(1000).human_binary(), "1000 B");
        assert_eq!(BitRate(1000).human(), "1 kbit/s");
    }
}
