#!/usr/bin/env bash
#
# Fails if the committed Dart/TS clients are out of date with the .proto sources.
#
#   tools/verify-proto-codegen.sh
#
# This is the mechanism behind the WP-M0.2 exit criterion "changing a proto and
# forgetting to regenerate fails CI" (ADR-0013). It regenerates into the working
# tree and asserts git sees no change; a diff means someone edited a .proto but
# did not run tools/gen-proto.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || { echo "cannot enter repo root" >&2; exit 1; }

echo "==> regenerating clients from proto"
tools/gen-proto.sh

echo "==> checking the working tree is unchanged"
GEN_DIRS=(sdk/dart/lib/src/generated sdk/typescript/src/generated)
# A difference in tracked files (git diff) OR any untracked file (git status)
# both mean the committed output does not match a fresh generation.
if git diff --quiet -- "${GEN_DIRS[@]}" && [ -z "$(git status --porcelain -- "${GEN_DIRS[@]}")" ]; then
    echo "proto codegen is up to date."
else
    echo
    echo "ERROR: generated clients are stale. A .proto changed but the committed"
    echo "Dart/TS output was not regenerated. Run:"
    echo "    tools/gen-proto.sh"
    echo "and commit the result. Offending paths:"
    git status --porcelain -- "${GEN_DIRS[@]}" | sed 's/^/    /'
    exit 1
fi
