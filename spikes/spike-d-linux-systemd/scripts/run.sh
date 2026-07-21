#!/usr/bin/env bash
#
# SPIKE-D full lifecycle on Linux. Requires sudo (installs a systemd service).
#
#   sudo scripts/run.sh
#
# Exercises, in order:
#   1. hardened unit starts and notifies readiness (Type=notify)
#   2. the daemon is reachable and applied its state tag
#   3. F-2: kill the daemon uncleanly, simulate reboot, prove boot-cleanup
#      reclaims the orphaned state
#   4. clean uninstall leaves nothing behind
set -uo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIBEXEC=/usr/local/libexec
UNIT_DIR=/etc/systemd/system
STATE_TAG=/run/networkhub-spike-d.state
FAIL=0

echo "==> installing binaries and units"
install -m 0755 "$HERE/helper/target/release/spike-d-daemon"  "$LIBEXEC/networkhub-spike-d-daemon"
install -m 0755 "$HERE/helper/target/release/spike-d-cleanup" "$LIBEXEC/networkhub-spike-d-cleanup"
install -m 0644 "$HERE/units/networkhub-spike-d.service"         "$UNIT_DIR/"
install -m 0644 "$HERE/units/networkhub-spike-d-cleanup.service" "$UNIT_DIR/"
systemctl daemon-reload

echo "==> [1] start and wait for readiness"
if systemctl start networkhub-spike-d.service; then
    echo "    started (systemctl start returned after sd_notify READY)"
    systemctl --no-pager --property=ActiveState,SubState show networkhub-spike-d.service | sed 's/^/    /'
else
    echo "    FAIL: service did not start"; FAIL=1
fi

echo "==> [2] daemon reachable and state applied"
[ -S /run/networkhub-spike-d.sock ] && echo "    socket present" || { echo "    FAIL: no socket"; FAIL=1; }
[ -f "$STATE_TAG" ] && echo "    state tag present: $STATE_TAG" || { echo "    FAIL: no state tag"; FAIL=1; }

echo "==> [3] F-2 scenario: unclean kill, then simulated reboot cleanup"
MAINPID=$(systemctl show -p MainPID --value networkhub-spike-d.service)
# Stop systemd from restarting it, then SIGKILL: the daemon never runs its
# SIGTERM cleanup, leaving the state tag orphaned — exactly the F-2 hazard.
systemctl stop networkhub-spike-d.service 2>/dev/null
kill -9 "$MAINPID" 2>/dev/null
# Re-create the orphan deterministically (stop may have triggered clean SIGTERM).
echo "orphaned-by-networkhub" > "$STATE_TAG"
rm -f /run/networkhub-spike-d.sock
echo "    orphaned state present before cleanup: $([ -f "$STATE_TAG" ] && echo yes || echo no)"

# Simulate the next boot: run the cleanup oneshot.
systemctl start networkhub-spike-d-cleanup.service
sleep 0.5
if [ -f "$STATE_TAG" ]; then
    echo "    FAIL: boot-cleanup did NOT remove the orphaned state (F-2 not satisfied)"
    FAIL=1
else
    echo "    boot-cleanup removed the orphan — F-2 satisfied, machine not stranded"
fi

echo "==> [4] uninstall clean"
systemctl disable --now networkhub-spike-d.service 2>/dev/null
systemctl disable networkhub-spike-d-cleanup.service 2>/dev/null
rm -f "$UNIT_DIR/networkhub-spike-d.service" "$UNIT_DIR/networkhub-spike-d-cleanup.service"
rm -f "$LIBEXEC/networkhub-spike-d-daemon" "$LIBEXEC/networkhub-spike-d-cleanup"
rm -f "$STATE_TAG" /run/networkhub-spike-d.sock
systemctl daemon-reload
LEFT=$(find /etc/systemd/system /usr/local/libexec /run -name '*networkhub-spike-d*' 2>/dev/null)
[ -z "$LEFT" ] && echo "    clean: nothing left behind" || { echo "    FAIL: leftover: $LEFT"; FAIL=1; }

echo
[ "$FAIL" -eq 0 ] && echo "SPIKE-D: PASS" || echo "SPIKE-D: FAIL"
exit $FAIL
