# NetworkHub Dart client

Generated Dart/gRPC bindings for the NetworkHub API, consumed by the desktop app
(`apps/desktop`, from WP-M0.12).

**`lib/src/generated/` is machine-generated — do not edit it by hand.** The
source of truth is `proto/networkhub/v1/*.proto`. Regenerate with:

```bash
tools/gen-proto.sh
```

CI (`tools/verify-proto-codegen.sh`) fails if the committed output is stale, so a
proto change without regeneration cannot merge (ADR-0013).
