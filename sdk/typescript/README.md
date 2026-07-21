# NetworkHub TypeScript types

Generated TypeScript types for the NetworkHub API (via `@bufbuild/protoc-gen-es`),
for the SDK and web dashboard (P2.6 / P3.5).

**`src/generated/` is machine-generated — do not edit it by hand.** The source of
truth is `proto/networkhub/v1/*.proto`. Regenerate with:

```bash
tools/gen-proto.sh
```

`node_modules/` holds only the codegen plugin and is not committed; the generated
output is. CI (`tools/verify-proto-codegen.sh`) fails if the committed output is
stale (ADR-0013).
