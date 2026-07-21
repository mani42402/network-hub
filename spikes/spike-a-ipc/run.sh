#!/usr/bin/env bash
#
# SPIKE-A end-to-end run: builds the Rust server, starts it, runs the Dart
# client against it, and tears down. Works on Linux and macOS.
# For Windows, use run-windows.ps1.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE" || { echo "cannot enter $HERE" >&2; exit 1; }

export PATH="$PATH:$HOME/.pub-cache/bin"
SOCKET="/tmp/networkhub-spike-a.sock"
SERVER_PID=""

cleanup() {
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null
    rm -f "$SOCKET"
}
trap cleanup EXIT INT TERM

echo "==> building Rust server"
( cd rust-server && cargo build --quiet ) || { echo "server build failed"; exit 1; }

echo "==> fetching Dart dependencies"
( cd dart-client && dart pub get >/dev/null ) || { echo "pub get failed"; exit 1; }

echo "==> starting server"
rm -f "$SOCKET"
./rust-server/target/debug/spike-a-server > /tmp/spike-a-server.log 2>&1 &
SERVER_PID=$!

# Wait for the socket rather than sleeping a fixed interval — a race here would
# make the spike look like a transport failure.
for _ in $(seq 1 50); do
    [ -S "$SOCKET" ] && break
    sleep 0.1
done
if [ ! -S "$SOCKET" ]; then
    echo "server never created $SOCKET; log:"
    cat /tmp/spike-a-server.log
    exit 1
fi
echo "    socket up: $(ls -l "$SOCKET" | awk '{print $1, $3, $4}')"
echo

( cd dart-client && dart run bin/client.dart )
RESULT=$?

echo
echo "==> server log"
sed 's/^/    /' /tmp/spike-a-server.log | head -20

exit $RESULT
