//! Reading the per-RPC `required_permission` option from the API descriptor.
//!
//! Authorization is declared on each RPC in the `.proto` (common.proto's
//! `required_permission` method option) so that enforcement and documentation
//! come from one source and cannot drift (ARCHITECTURE.md §10.3). prost does not
//! surface custom options as Rust constants and drops them on decode, so this
//! module reads them from the compiled file descriptor set via `prost-reflect`.
//!
//! The daemon's authorization interceptor (WP-M0.6) uses [`permission_map`] to
//! answer "what permission does this gRPC method require?", and the
//! permission-coverage test uses [`all_methods`] to assert none is unannotated.

use std::collections::BTreeMap;

use prost_reflect::DescriptorPool;

use crate::descriptor::FILE_DESCRIPTOR_SET;

/// The fully-qualified name of the `required_permission` extension, as declared
/// in `common.proto`.
const PERMISSION_EXTENSION: &str = "networkhub.v1.required_permission";

/// A method's gRPC path and the permission it requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodPermission {
    /// The gRPC path, `/networkhub.v1.SystemService/GetVersion`, as it appears on
    /// the wire — the key the interceptor matches against an incoming call.
    pub grpc_path: String,
    /// The `Permission` enum value the caller must hold. `0`
    /// (`PERMISSION_UNSPECIFIED`) means the RPC is unannotated, which is a bug.
    pub permission: i32,
}

/// Loads the descriptor pool once. Cheap enough to call at startup; the daemon
/// caches the resulting map.
fn pool() -> DescriptorPool {
    DescriptorPool::decode(FILE_DESCRIPTOR_SET)
        .expect("the compiled-in descriptor set must be valid")
}

/// Every RPC in the API, with its declared permission (or `0` if missing).
///
/// # Panics
/// Panics if the compiled-in descriptor set cannot be decoded, which would mean
/// the build produced a corrupt artifact.
#[must_use]
pub fn all_methods() -> Vec<MethodPermission> {
    let pool = pool();
    let extension = pool.get_extension_by_name(PERMISSION_EXTENSION);

    let mut methods = Vec::new();
    for service in pool.services() {
        for method in service.methods() {
            let permission = extension.as_ref().map_or(0, |ext| {
                method
                    .options()
                    .get_extension(ext)
                    .as_enum_number()
                    .unwrap_or(0)
            });
            methods.push(MethodPermission {
                grpc_path: format!("/{}/{}", service.full_name(), method.name()),
                permission,
            });
        }
    }
    methods
}

/// A lookup from gRPC method path to required permission, for the authz
/// interceptor.
///
/// # Panics
/// Panics if the descriptor set is invalid (see [`all_methods`]).
#[must_use]
pub fn permission_map() -> BTreeMap<String, i32> {
    all_methods()
        .into_iter()
        .map(|m| (m.grpc_path, m.permission))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods_are_discovered() {
        // The descriptor must expose the services we defined; if this is empty,
        // codegen or the descriptor emission is broken.
        let methods = all_methods();
        assert!(!methods.is_empty());
        assert!(
            methods
                .iter()
                .any(|m| m.grpc_path.contains("SystemService"))
        );
    }

    #[test]
    fn grpc_paths_are_wire_shaped() {
        let map = permission_map();
        // The interceptor matches paths of exactly this form.
        assert!(map.contains_key("/networkhub.v1.SystemService/GetVersion"));
    }
}
