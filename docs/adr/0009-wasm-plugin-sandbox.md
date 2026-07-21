# ADR-0009: Plugins run in a WASM sandbox with capability grants and mandatory signatures

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-9 · ARCHITECTURE.md §9
- **Supersedes:** —

## Context

The product promises an extensible plugin ecosystem — DNS providers, VPN providers,
notification backends, analytics exporters — and eventually a marketplace. Plugins run
alongside a process that controls the firewall. A native plugin model (dynamic libraries)
offers no isolation whatsoever: a plugin would have the daemon's full privileges, and
"sandboxed plugins" would be a marketing claim rather than a property.

Cross-platform native sandboxing (seccomp, AppContainer, App Sandbox) is three different
mechanisms with three different failure modes, and none is straightforward to apply to
in-process code.

## Decision

Plugins are **WebAssembly components** executed by `wasmtime` under WASI, with:

- **capability-scoped host functions** declared in a manifest and granted by the user at
  install time, revocable afterwards — no ambient authority, no filesystem or network access
  by default;
- **resource limits**: memory caps, fuel/epoch CPU limits, and wall-clock watchdogs;
- **mandatory Ed25519 signatures**, with unsigned plugins permitted only in an explicit
  developer mode that displays a persistent warning;
- **failure containment**: a plugin that crashes or times out is restarted with backoff and
  auto-disabled after repeated failure. A plugin can never block a reconciler.

**No native plugins in version 1.** Integrations genuinely requiring OS access are proposed as
core modules instead.

## Consequences

**Positive**
- The sandbox claim is true, with one mechanism on all three platforms.
- Untrusted marketplace code becomes tractable: capability grants are reviewable by users and
  enforceable by us.
- Resource limits mean a badly written plugin degrades itself, not the network stack.
- WASM's stable ABI decouples plugin binaries from our Rust compiler version.

**Negative**
- WASM cannot do everything: no native library reuse, and performance-sensitive paths (a DNS
  filtering hook on every query) need a strict time budget with fail-open/fail-closed policy.
- The component model and WASI toolchains are still maturing, particularly for TypeScript.
- We must design and version a host API surface — effectively a second public contract.
- Plugin authors face a less familiar development experience than a native SDK.

**Neutral**
- Forces provider interfaces to be well specified. Core features will consume the same
  interfaces plugins do, which proves them before the SDK is published.

## Alternatives considered

**Native dynamic-library plugins** — rejected: no isolation, and a plugin bug becomes a root
compromise.

**Subprocess plugins with an IPC protocol** — a reasonable design, rejected for version 1: it
requires per-platform sandboxing to be meaningfully safe, and costs a process per plugin.

**Scripting language runtime (Lua, JS engine)** — rejected: weaker isolation guarantees than
WASM, and a language choice imposed on plugin authors.
