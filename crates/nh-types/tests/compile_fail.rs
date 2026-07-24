//! Compile-fail tests: they assert that certain misuse does **not** compile.
//!
//! The headline guarantee of `Secret<T>` (ARCHITECTURE.md §1.6) is that a secret
//! cannot reach a log or a message by accident. The strongest possible proof of
//! that is a test which fails to build if the guarantee is ever weakened — for
//! example if someone adds a `Display` impl to `Secret`. `trybuild` runs each
//! case below and checks it produces the expected compiler error.

#[test]
fn secret_cannot_be_misused() {
    let t = trybuild::TestCases::new();
    // Each .rs file here must fail to compile, with the error captured in the
    // matching .stderr file. If a change makes one of them *compile*, this test
    // fails — which is exactly the alarm we want.
    t.compile_fail("tests/compile-fail/*.rs");
}
