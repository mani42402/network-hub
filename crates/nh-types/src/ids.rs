//! Strongly-typed entity identifiers.
//!
//! Every persisted entity has its own ID type — a newtype around a [`Uuid`], not
//! a bare `Uuid` or `String`. This means the compiler rejects passing a
//! `DeviceId` where a `VpnProfileId` is expected, a mistake that is otherwise
//! easy to make and hard to notice when everything is a UUID.
//!
//! IDs are **UUIDv7**: time-ordered, so rows sort by creation without a separate
//! timestamp column, and merge-friendly for the future cloud-sync work (§7.2)
//! where two machines may mint IDs independently.

use uuid::Uuid;

/// Defines a newtype ID with construction, parsing, display, and serde, so every
/// entity ID behaves identically and none drifts.
macro_rules! entity_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Mints a fresh, time-ordered identifier.
            ///
            /// Uses UUIDv7, so the value embeds its creation time and sorts
            /// chronologically. This calls the system clock and RNG, so it is
            /// not available in contexts that forbid them (e.g. workflow
            /// scripts) — construct from a known value there instead.
            // No `Default`: an all-zero id is not a valid identity, and a
            // `Default` that mints a random one would hide a clock/RNG call
            // behind an innocuous-looking `Default::default()`. `new()` is
            // explicit on purpose.
            #[allow(clippy::new_without_default)]
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wraps an existing UUID, e.g. one loaded from the database.
            #[must_use]
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// The underlying UUID, for storage or interop.
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        // No `Default`: an all-zero UUID is not a valid identity, and a
        // `Default` that silently mints a random one hides a clock/RNG call.

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // Hyphenated lower-case, the canonical UUID form.
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

entity_id!(
    /// Identifies a device seen on a managed network. Note this is a *synthetic*
    /// identity that survives MAC randomization (§7.2), not the MAC itself.
    DeviceId
);
entity_id!(
    /// Identifies a stored VPN profile (WireGuard or OpenVPN).
    VpnProfileId
);
entity_id!(
    /// Identifies a stored hotspot profile.
    HotspotProfileId
);
entity_id!(
    /// Identifies a firewall rule set (a versioned document, §7.2). A specific
    /// revision is this ID plus a revision number.
    FirewallRuleSetId
);
entity_id!(
    /// Identifies an automation rule.
    AutomationRuleId
);
entity_id!(
    /// Identifies a plugin installation.
    PluginId
);
entity_id!(
    /// Identifies an issued API key (the key material itself is a [`crate::Secret`]).
    ApiKeyId
);
entity_id!(
    /// Identifies a principal (a local user, a gateway account, a plugin, …).
    PrincipalId
);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_string() {
        let id = DeviceId::new();
        let s = id.to_string();
        let parsed: DeviceId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn v7_ids_are_time_ordered() {
        // UUIDv7 embeds a millisecond timestamp, so later-minted IDs sort after
        // earlier ones. This is what lets rows order by ID without a separate
        // created-at column.
        let a = DeviceId::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = DeviceId::new();
        assert!(a < b, "expected {a} < {b}");
    }

    #[test]
    fn distinct_id_types_do_not_mix() {
        // This is a compile-time property; the test documents it. The following
        // would not compile, which is the entire point of separate ID types:
        //     let d: DeviceId = VpnProfileId::new();
        let d = DeviceId::new();
        let v = VpnProfileId::new();
        // They can still be compared only via their UUIDs, deliberately.
        assert_ne!(d.as_uuid(), v.as_uuid());
    }

    #[test]
    fn rejects_malformed_uuid() {
        assert!("not-a-uuid".parse::<DeviceId>().is_err());
    }

    #[test]
    fn serde_is_transparent() {
        let id = VpnProfileId::new();
        let json = serde_json::to_string(&id).unwrap();
        // Serializes as the bare UUID string, not a wrapper object.
        assert_eq!(json, format!("\"{id}\""));
        let back: VpnProfileId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
