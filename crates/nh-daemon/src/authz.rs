//! Request authorization (§10.3).
//!
//! Every RPC declares the permission it requires in the proto (WP-M0.2); the
//! daemon reads those declarations from the compiled descriptor into a map, and
//! this [`Authorizer`] answers "may this principal call this method?". Keeping
//! the decision pure — no transport, no async — is what lets the two hard
//! guarantees be unit-tested directly:
//!
//! 1. **Nothing unauthorized gets through.** A principal without the required
//!    permission is denied.
//! 2. **No RPC is accidentally unprotected.** A method that is unknown or whose
//!    required permission is `Unspecified` is denied to *everyone* — so a
//!    forgotten annotation fails closed rather than open.

use std::collections::BTreeMap;

use crate::principal::{Permission, Principal};

/// Decides whether a principal may call a given gRPC method.
#[derive(Debug, Clone)]
pub struct Authorizer {
    /// gRPC path (`/networkhub.v1.SystemService/GetVersion`) → required permission.
    required: BTreeMap<String, Permission>,
}

/// Why authorization was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthzError {
    /// The method is not in the permission map — an unknown or unannotated RPC.
    /// Denied to everyone, so a missing annotation cannot expose an endpoint.
    #[error("unknown or unannotated method: {method}")]
    UnknownMethod {
        /// The gRPC path that was not found.
        method: String,
    },
    /// The principal does not hold the required permission.
    #[error("{label} lacks permission {required:?} for {method}")]
    Denied {
        /// The caller's label.
        label: String,
        /// The permission the method requires.
        required: Permission,
        /// The gRPC path.
        method: String,
    },
}

impl Authorizer {
    /// Builds an authorizer from the compiled proto descriptor (the single source
    /// of truth for per-RPC permissions).
    ///
    /// # Panics
    /// Panics if the descriptor is malformed, which would mean a corrupt build
    /// artifact — a bug to catch at startup, not to paper over.
    #[must_use]
    pub fn from_descriptor() -> Self {
        let required = nh_proto::permissions::permission_map()
            .into_iter()
            .map(|(path, value)| {
                let perm = Permission::from_i32(value).unwrap_or(Permission::Unspecified);
                (path, perm)
            })
            .collect();
        Self { required }
    }

    /// Builds an authorizer from an explicit map, for tests.
    #[must_use]
    pub fn from_map(required: BTreeMap<String, Permission>) -> Self {
        Self { required }
    }

    /// The permission a method requires, if it is known.
    #[must_use]
    pub fn required_for(&self, method: &str) -> Option<Permission> {
        self.required.get(method).copied()
    }

    /// The set of known method paths, for coverage checks.
    pub fn methods(&self) -> impl Iterator<Item = &str> {
        self.required.keys().map(String::as_str)
    }

    /// Authorizes a call, or returns why it was refused.
    ///
    /// # Errors
    /// Returns [`AuthzError::UnknownMethod`] for an unknown/unannotated method
    /// (denied to all), or [`AuthzError::Denied`] if the principal lacks the
    /// permission.
    pub fn authorize(&self, method: &str, principal: &Principal) -> Result<(), AuthzError> {
        let Some(required) = self.required_for(method) else {
            return Err(AuthzError::UnknownMethod {
                method: method.to_owned(),
            });
        };
        // Unspecified is never granted (see Principal::has_permission), so an
        // unannotated method is denied to everyone here too.
        if principal.has_permission(required) {
            Ok(())
        } else {
            Err(AuthzError::Denied {
                label: principal.label.clone(),
                required,
                method: method.to_owned(),
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::principal::{PrincipalKind, Role};

    fn principal(roles: &[Role]) -> Principal {
        Principal {
            kind: PrincipalKind::LocalUser { uid: 501, gid: 20 },
            roles: roles.iter().copied().collect(),
            label: "uid:501".to_owned(),
        }
    }

    #[test]
    fn every_real_rpc_has_a_valid_permission() {
        // The runtime counterpart of the WP-M0.2 compile-time coverage test: the
        // authorizer built from the real descriptor must have a non-Unspecified
        // permission for every method, or it would be denied to everyone.
        let authz = Authorizer::from_descriptor();
        let methods: Vec<&str> = authz.methods().collect();
        assert!(!methods.is_empty(), "no methods in the descriptor");
        for method in methods {
            let required = authz.required_for(method).unwrap();
            assert_ne!(
                required,
                Permission::Unspecified,
                "{method} has no valid required permission"
            );
        }
    }

    #[test]
    fn viewer_allowed_read_denied_write() {
        let mut map = BTreeMap::new();
        map.insert("/svc/Read".to_owned(), Permission::SystemRead);
        map.insert("/svc/Write".to_owned(), Permission::FirewallWrite);
        let authz = Authorizer::from_map(map);

        let viewer = principal(&[Role::Viewer]);
        assert!(authz.authorize("/svc/Read", &viewer).is_ok());
        assert!(matches!(
            authz.authorize("/svc/Write", &viewer),
            Err(AuthzError::Denied { .. })
        ));
    }

    #[test]
    fn unknown_method_is_denied_to_everyone() {
        let authz = Authorizer::from_map(BTreeMap::new());
        let admin = principal(&[Role::Admin]);
        assert!(matches!(
            authz.authorize("/svc/Ghost", &admin),
            Err(AuthzError::UnknownMethod { .. })
        ));
    }

    #[test]
    fn unannotated_method_is_denied_even_to_admin() {
        // A method whose required permission is Unspecified (a forgotten
        // annotation) fails closed.
        let mut map = BTreeMap::new();
        map.insert("/svc/Forgot".to_owned(), Permission::Unspecified);
        let authz = Authorizer::from_map(map);
        let admin = principal(&[Role::Admin]);
        assert!(matches!(
            authz.authorize("/svc/Forgot", &admin),
            Err(AuthzError::Denied { .. })
        ));
    }

    #[test]
    fn system_principal_is_authorized_for_writes() {
        let mut map = BTreeMap::new();
        map.insert("/svc/Apply".to_owned(), Permission::FirewallWrite);
        let authz = Authorizer::from_map(map);
        let _ = BTreeSet::<Role>::new();
        assert!(authz.authorize("/svc/Apply", &Principal::system()).is_ok());
    }
}
