#!/usr/bin/env bash
#
# SPIKE-C verification. Runs unprivileged: it connects to the root helper and
# confirms launchd started it as root. No sudo needed.
#
#   scripts/verify.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LABEL="com.networkhub.spike-c"
PROBE="$HERE/helper/target/release/spike-c-probe"

echo "==> launchd sees the daemon?"
if launchctl print "system/$LABEL" >/dev/null 2>&1; then
    launchctl print "system/$LABEL" 2>/dev/null | grep -E "state =|pid =" | sed 's/^/    /'
else
    echo "    NOT loaded — run: sudo scripts/install.sh"
    exit 1
fi

echo "==> probing the helper over its socket"
"$PROBE"
