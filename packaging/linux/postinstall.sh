#!/bin/sh
# Runs after the package's files are installed (deb and rpm). Idempotent: it may
# run on a fresh install or an upgrade.
set -e

# Members of the networkhub group may administer the daemon without root; the
# socket is group-accessible (ADR-0002).
if ! getent group networkhub >/dev/null 2>&1; then
    groupadd --system networkhub 2>/dev/null || addgroup --system networkhub 2>/dev/null || true
fi

# Register and start the services. The cleanup deadman is ordered before the
# network so it reclaims stale state on every boot (F-2).
systemctl daemon-reload 2>/dev/null || true
systemctl enable --now networkhub-cleanup.service 2>/dev/null || true
systemctl enable --now networkhub.service 2>/dev/null || true

exit 0
