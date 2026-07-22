//! The local IPC surface (ADR-0002).
//!
//! gRPC over a Unix domain socket (Unix) or a named pipe (Windows), never TCP.
//! The daemon authenticates callers by OS peer credentials and authorizes every
//! RPC through the [`authz_layer`]. This module wires the transport, the
//! authorization middleware, and the principal resolver together.

mod authz_layer;

use std::sync::Arc;

use http::Request;
use tonic::body::Body;

use crate::authz::Authorizer;
use crate::principal::{Principal, PrincipalKind, Role};

pub use authz_layer::AuthzLayer;

/// Peer credentials read from the local connection — the OS's answer to "who is
/// calling?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerCred {
    /// The caller's OS user id.
    pub uid: u32,
    /// The caller's OS group id.
    pub gid: u32,
}

/// Resolves a request's connection into a [`Principal`]. Injectable so tests can
/// force a specific identity (e.g. an unauthorized user) without needing a real
/// OS account.
pub type PrincipalResolver = Arc<dyn Fn(&Request<Body>) -> Principal + Send + Sync>;

/// Extracts peer credentials from a request's connection info, if present.
#[must_use]
pub fn peer_cred_of(req: &Request<Body>) -> Option<PeerCred> {
    #[cfg(unix)]
    {
        use tonic::transport::server::UdsConnectInfo;
        if let Some(info) = req.extensions().get::<UdsConnectInfo>()
            && let Some(cred) = &info.peer_cred
        {
            return Some(PeerCred {
                uid: cred.uid(),
                gid: cred.gid(),
            });
        }
    }
    let _ = req;
    None
}

/// The default principal resolver: maps peer credentials to a role.
///
/// For M0.6 the mapping is deliberately simple — the user running the daemon
/// (its own euid) and root are administrators; any other local user is a viewer;
/// a connection with no readable credentials gets no roles (denied everything).
/// Group-based mapping via the `NetworkHub Operators` group lands in WP-M0.7.
#[must_use]
pub fn default_resolver() -> PrincipalResolver {
    let daemon_uid = current_uid();
    Arc::new(move |req| match peer_cred_of(req) {
        Some(cred) => {
            let roles = if cred.uid == 0 || cred.uid == daemon_uid {
                [Role::Admin].into_iter().collect()
            } else {
                [Role::Viewer].into_iter().collect()
            };
            Principal {
                kind: PrincipalKind::LocalUser {
                    uid: cred.uid,
                    gid: cred.gid,
                },
                roles,
                label: format!("uid:{}", cred.uid),
            }
        }
        None => Principal {
            kind: PrincipalKind::LocalUser {
                uid: u32::MAX,
                gid: u32::MAX,
            },
            roles: std::collections::BTreeSet::new(),
            label: "unauthenticated".to_owned(),
        },
    })
}

/// A resolver that assigns fixed roles to every caller, for tests.
#[must_use]
pub fn fixed_resolver(roles: &[Role], label: &str) -> PrincipalResolver {
    let roles: std::collections::BTreeSet<Role> = roles.iter().copied().collect();
    let label = label.to_owned();
    Arc::new(move |_req| Principal {
        kind: PrincipalKind::LocalUser { uid: 0, gid: 0 },
        roles: roles.clone(),
        label: label.clone(),
    })
}

/// Builds the authorization layer from the compiled descriptor and a resolver.
#[must_use]
pub fn authz_layer(resolver: PrincipalResolver) -> AuthzLayer {
    AuthzLayer::new(Arc::new(Authorizer::from_descriptor()), resolver)
}

#[cfg(unix)]
fn current_uid() -> u32 {
    nix::unistd::getuid().as_raw()
}

#[cfg(not(unix))]
fn current_uid() -> u32 {
    // On Windows, uid is not meaningful; the WP-M0.7 named-pipe path uses the
    // client token instead. Until then, no uid matches, so only the fixed
    // resolver (tests) or root-equivalent logic applies.
    u32::MAX - 1
}
