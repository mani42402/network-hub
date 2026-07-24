//! Principals, roles, and permissions — the authorization model (§10.3).
//!
//! A *principal* is whoever is making a request: a local OS user, the gateway,
//! a plugin, or the daemon's own system identity. Each principal holds *roles*,
//! and each role grants a set of *permissions*. Every RPC declares the
//! permission it requires (in the proto, WP-M0.2); the interceptor checks that
//! the caller's roles grant it.
//!
//! Permissions here mirror the `Permission` enum in `common.proto` by integer
//! value, so the value read from the descriptor maps straight onto this model.

use std::collections::BTreeSet;

/// A permission required to perform an operation. The integer values match the
/// `networkhub.v1.Permission` proto enum exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Permission {
    /// Never a valid grant; a required permission of this value is a bug.
    Unspecified = 0,
    /// Read non-sensitive system state (capabilities, health, interfaces).
    SystemRead = 1,
    /// Read device inventory and status.
    DeviceRead = 2,
    /// Change device state (block, pause, quota).
    DeviceWrite = 3,
    /// Read VPN profiles and status.
    VpnRead = 4,
    /// Manage VPN profiles and connections.
    VpnWrite = 5,
    /// Read hotspot profiles and status.
    HotspotRead = 6,
    /// Manage hotspot profiles and lifecycle.
    HotspotWrite = 7,
    /// Read firewall rules.
    FirewallRead = 8,
    /// Change firewall rules (security-sensitive).
    FirewallWrite = 9,
    /// Administer security-sensitive settings (gateway, plugins, secrets export).
    Admin = 10,
}

impl Permission {
    /// Maps a raw proto enum value onto a [`Permission`], or `None` if unknown.
    #[must_use]
    pub fn from_i32(value: i32) -> Option<Self> {
        Some(match value {
            0 => Self::Unspecified,
            1 => Self::SystemRead,
            2 => Self::DeviceRead,
            3 => Self::DeviceWrite,
            4 => Self::VpnRead,
            5 => Self::VpnWrite,
            6 => Self::HotspotRead,
            7 => Self::HotspotWrite,
            8 => Self::FirewallRead,
            9 => Self::FirewallWrite,
            10 => Self::Admin,
            _ => return None,
        })
    }
}

/// A role: a named bundle of permissions (§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Read-only: observe state, but change nothing.
    Viewer,
    /// Day-to-day operations: device/VPN/hotspot changes, but no security
    /// settings (no firewall writes, no admin).
    Operator,
    /// Full control, including firewall and security-sensitive administration.
    Admin,
    /// The AI assistant: read and propose only — never a write permission
    /// (§8.9). Currently equivalent to Viewer; kept distinct so its grants can
    /// diverge without touching Viewer.
    Assistant,
}

impl Role {
    /// The permissions this role grants.
    #[must_use]
    pub fn permissions(self) -> BTreeSet<Permission> {
        use Permission as P;
        let list: &[Permission] = match self {
            Self::Viewer | Self::Assistant => &[
                P::SystemRead,
                P::DeviceRead,
                P::VpnRead,
                P::HotspotRead,
                P::FirewallRead,
            ],
            Self::Operator => &[
                P::SystemRead,
                P::DeviceRead,
                P::DeviceWrite,
                P::VpnRead,
                P::VpnWrite,
                P::HotspotRead,
                P::HotspotWrite,
                P::FirewallRead,
            ],
            Self::Admin => &[
                P::SystemRead,
                P::DeviceRead,
                P::DeviceWrite,
                P::VpnRead,
                P::VpnWrite,
                P::HotspotRead,
                P::HotspotWrite,
                P::FirewallRead,
                P::FirewallWrite,
                P::Admin,
            ],
        };
        list.iter().copied().collect()
    }
}

/// How a principal was identified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalKind {
    /// A local OS user, identified by peer credentials over the local socket.
    LocalUser {
        /// The OS user id.
        uid: u32,
        /// The OS group id.
        gid: u32,
    },
    /// The daemon's own internal identity (automation, reconcilers).
    System,
}

/// Whoever is making a request, with the roles they hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// How the principal was identified.
    pub kind: PrincipalKind,
    /// The roles granted to this principal.
    pub roles: BTreeSet<Role>,
    /// A short human label for audit and logs, e.g. `uid:501` or `system`.
    pub label: String,
}

impl Principal {
    /// The daemon's own system principal, holding every role.
    #[must_use]
    pub fn system() -> Self {
        Self {
            kind: PrincipalKind::System,
            roles: [Role::Admin].into_iter().collect(),
            label: "system".to_owned(),
        }
    }

    /// Whether this principal holds a permission through any of its roles.
    #[must_use]
    pub fn has_permission(&self, permission: Permission) -> bool {
        // The unspecified permission is never granted, so an unannotated RPC
        // (which would require Unspecified) is denied to everyone — a fail-safe
        // backstop to the compile-time annotation-coverage test.
        if permission == Permission::Unspecified {
            return false;
        }
        self.roles
            .iter()
            .any(|role| role.permissions().contains(&permission))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(roles: &[Role]) -> Principal {
        Principal {
            kind: PrincipalKind::LocalUser { uid: 501, gid: 20 },
            roles: roles.iter().copied().collect(),
            label: "uid:501".to_owned(),
        }
    }

    #[test]
    fn viewer_can_read_not_write() {
        let p = user(&[Role::Viewer]);
        assert!(p.has_permission(Permission::SystemRead));
        assert!(p.has_permission(Permission::VpnRead));
        assert!(!p.has_permission(Permission::VpnWrite));
        assert!(!p.has_permission(Permission::Admin));
    }

    #[test]
    fn operator_writes_but_not_security() {
        let p = user(&[Role::Operator]);
        assert!(p.has_permission(Permission::VpnWrite));
        assert!(p.has_permission(Permission::HotspotWrite));
        // Firewall writes and admin are security-sensitive: denied to operator.
        assert!(!p.has_permission(Permission::FirewallWrite));
        assert!(!p.has_permission(Permission::Admin));
    }

    #[test]
    fn admin_can_do_everything() {
        let p = user(&[Role::Admin]);
        for perm in [
            Permission::SystemRead,
            Permission::DeviceWrite,
            Permission::FirewallWrite,
            Permission::Admin,
        ] {
            assert!(p.has_permission(perm), "admin should hold {perm:?}");
        }
    }

    #[test]
    fn assistant_is_read_only() {
        let p = user(&[Role::Assistant]);
        assert!(p.has_permission(Permission::DeviceRead));
        assert!(!p.has_permission(Permission::DeviceWrite));
    }

    #[test]
    fn unspecified_is_never_granted() {
        // An unannotated RPC (required permission Unspecified) is denied even to
        // an admin — the runtime fail-safe behind the compile-time coverage test.
        assert!(!Principal::system().has_permission(Permission::Unspecified));
    }

    #[test]
    fn proto_values_map_correctly() {
        assert_eq!(Permission::from_i32(1), Some(Permission::SystemRead));
        assert_eq!(Permission::from_i32(10), Some(Permission::Admin));
        assert_eq!(Permission::from_i32(0), Some(Permission::Unspecified));
        assert_eq!(Permission::from_i32(99), None);
    }
}
