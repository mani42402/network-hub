# Protobuf contracts

Single source of truth for every client API (ADR-0013). Rust, Dart, and TypeScript
clients are generated from here; `buf` enforces backward compatibility in CI.

Lands in **WP-M0.2**. Per-RPC permission annotations drive the authorisation
interceptor, so the permission model and its documentation cannot diverge.
