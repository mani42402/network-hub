#!/usr/bin/env bash
#
# Regenerates the committed Dart and TypeScript clients from the .proto sources.
#
#   tools/gen-proto.sh
#
# Rust is NOT generated here — it is produced at compile time by nh-proto's
# build.rs. This script handles only the clients that live outside the cargo
# build and are therefore committed (ADR-0013). CI runs the freshness check
# (tools/verify-proto-codegen.sh), which calls this and asserts no diff.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || { echo "cannot enter repo root" >&2; exit 1; }

command -v buf >/dev/null 2>&1 || { echo "buf not found; see tools/setup-*.sh"; exit 1; }

# protoc-gen-dart lives in the Dart pub cache after
# `dart pub global activate protoc_plugin`.
export PATH="$PATH:$HOME/.pub-cache/bin"
if ! command -v protoc-gen-dart >/dev/null 2>&1; then
    echo "protoc-gen-dart not found. Install with: dart pub global activate protoc_plugin"
    exit 1
fi

# protoc-gen-es is a devDependency of the TS SDK; install it if needed and put
# it on PATH.
if [ ! -x "sdk/typescript/node_modules/.bin/protoc-gen-es" ]; then
    echo "==> installing TypeScript codegen plugin"
    ( cd sdk/typescript && npm install --no-audit --no-fund )
fi
export PATH="$PATH:$ROOT/sdk/typescript/node_modules/.bin"

# Regenerate from clean output dirs so a removed proto message does not leave a
# stale generated file behind (which the freshness check would otherwise miss).
echo "==> clearing previous output"
rm -rf sdk/dart/lib/src/generated sdk/typescript/src/generated
mkdir -p sdk/dart/lib/src/generated sdk/typescript/src/generated

echo "==> buf generate"
( cd proto && buf generate )

echo "==> done"
echo "  Dart: $(find sdk/dart/lib/src/generated -name '*.dart' | wc -l | tr -d ' ') files"
echo "  TS:   $(find sdk/typescript/src/generated -name '*.ts' | wc -l | tr -d ' ') files"
