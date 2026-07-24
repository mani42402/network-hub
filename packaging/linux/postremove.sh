#!/bin/sh
# Runs after the package's files are removed. On deb, $1 is "remove"/"purge";
# on rpm, $1 is a count. Remove runtime state always; remove config and the
# group only on a deb "purge".
set -e

systemctl daemon-reload 2>/dev/null || true
rm -rf /run/networkhub

if [ "$1" = "purge" ]; then
    rm -rf /var/lib/networkhub
    groupdel networkhub 2>/dev/null || delgroup networkhub 2>/dev/null || true
fi

exit 0
