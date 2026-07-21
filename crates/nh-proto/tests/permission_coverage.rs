//! Every RPC must declare a `required_permission`.
//!
//! Mechanical enforcement of ARCHITECTURE.md §10.3: authorization is declared on
//! the RPC itself, and nothing ships without a declared permission. Forgetting
//! to annotate a method — especially a dangerous write method — would leave it
//! at `PERMISSION_UNSPECIFIED` (0), the exact class of bug that produces an
//! unprotected privileged endpoint. This test makes that impossible to merge.

use nh_proto::permissions::all_methods;

// PERMISSION_UNSPECIFIED == 0 in common.proto; never a valid grant.
const PERMISSION_UNSPECIFIED: i32 = 0;

#[test]
fn every_rpc_declares_a_valid_permission() {
    let methods = all_methods();
    assert!(
        !methods.is_empty(),
        "no RPCs found — codegen or descriptor broken?"
    );

    let unannotated: Vec<&str> = methods
        .iter()
        .filter(|m| m.permission == PERMISSION_UNSPECIFIED)
        .map(|m| m.grpc_path.as_str())
        .collect();

    assert!(
        unannotated.is_empty(),
        "these RPCs are missing a valid `required_permission` (§10.3):\n  {}",
        unannotated.join("\n  ")
    );

    eprintln!(
        "permission coverage: all {} RPC(s) annotated",
        methods.len()
    );
}
