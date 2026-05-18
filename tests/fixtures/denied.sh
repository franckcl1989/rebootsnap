#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
rm -rf denied
cp -r normal denied
# Make kmsg unreadable (chmod 0000)
rm -f denied/dev/kmsg
# Remove conntrack (simulating permission denied)
rm -f denied/proc/net/nf_conntrack denied/proc/net/stat/nf_conntrack
# Remove D-Bus socket
rm -f denied/run/dbus/system_bus_socket
