#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# partial = copy normal then remove systemd-related, conntrack, and tracefs files
rm -rf partial
cp -r normal partial

rm -f partial/run/dbus/system_bus_socket
rm -f partial/proc/net/nf_conntrack
rm -f partial/proc/net/stat/nf_conntrack
rm -f partial/proc/net/nf_tables_names
rm -rf partial/sys/kernel/debug

# malformed = copy normal then corrupt some files with unparseable data
rm -rf malformed
cp -r normal malformed

echo "-1" > malformed/proc/sys/kernel/pid_max
echo "garbage not a number" > malformed/proc/uptime
echo "broken json }{ :::" > malformed/proc/loadavg
printf '\x00\x01\x02\xff\xfe' > malformed/proc/meminfo
echo "99999999999999999999" > malformed/proc/sys/fs/file-nr
printf '\x00\xff' > malformed/etc/localtime
echo "not valid rtc time" > malformed/proc/driver/rtc
echo "0xdeadbeef" > malformed/proc/sys/kernel/random/entropy_avail
