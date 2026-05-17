#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

base="normal"

# ===== boot.rs (RT-01) =====
mkdir -p "$base/proc/sys/kernel/random"
echo "testhost"                > "$base/proc/sys/kernel/hostname"
echo "5.15.0-mock"             > "$base/proc/sys/kernel/osrelease"
echo "abc-def-123"             > "$base/proc/sys/kernel/random/boot_id"
echo "12345.67 89012.34"       > "$base/proc/uptime"
echo "0.10 0.20 0.30 1/100 1234" > "$base/proc/loadavg"

# ===== kernel.rs (RT-02) =====
echo "Linux"                   > "$base/proc/sys/kernel/ostype"
echo "mock_module 16384 0"     > "$base/proc/modules"
echo "0"                       > "$base/proc/sys/kernel/tainted"
echo "core"                    > "$base/proc/sys/kernel/core_pattern"
echo "0"                       > "$base/proc/sys/kernel/panic"
echo "7       4       1       7" > "$base/proc/sys/kernel/printk"

# ===== systemd.rs (RT-03) =====
mkdir -p "$base/run/dbus"
touch "$base/run/dbus/system_bus_socket"

# ===== process.rs (RT-04) =====
echo "4194304"                 > "$base/proc/sys/kernel/pid_max"
# mock 3 processes
for pid in 1 100 999; do
    d="$base/proc/$pid"
    mkdir -p "$d"
    echo "mock_exe"            > "$d/comm"
    echo "0"                   > "$d/oom_score"
done

# ===== cpu.rs (RT-05) =====
mkdir -p "$base/sys/devices/system/cpu"
echo "0-3"                     > "$base/sys/devices/system/cpu/possible"
cat > "$base/proc/stat" <<'EOF'
cpu  100 0 50 20000 30 0 5 0 0 0
cpu0 25 0 12 5000 8 0 1 0 0 0
cpu1 25 0 13 5000 7 0 2 0 0 0
cpu2 25 0 12 5000 8 0 1 0 0 0
cpu3 25 0 13 5000 7 0 1 0 0 0
intr 1234567
ctxt 9876543
btime 1700000000
processes 5000
procs_running 3
procs_blocked 0
softirq 100 1 2 3 4 5 6 7 8 9 10
EOF
echo ""                       > "$base/proc/softirqs"
echo ""                       > "$base/proc/interrupts"
mkdir -p "$base/proc/pressure"
echo "some avg10=0.00 avg60=0.01 avg300=0.05 total=1000" > "$base/proc/pressure/cpu"

# ===== memory.rs (RT-06) =====
cat > "$base/proc/meminfo" <<'EOF'
MemTotal:       16384000 kB
MemFree:         8192000 kB
MemAvailable:   10240000 kB
Buffers:         1024000 kB
Cached:          2048000 kB
SwapCached:            0 kB
Active:          4096000 kB
Inactive:        2048000 kB
SwapTotal:       8192000 kB
SwapFree:        8192000 kB
Dirty:              1024 kB
Writeback:             0 kB
Slab:             512000 kB
VmallocTotal:   34359738367 kB
VmallocUsed:           0 kB
HugePages_Total:       0
HugePages_Free:        0
Hugepagesize:       2048 kB
EOF
echo "nr_free_pages 2048000"  > "$base/proc/vmstat"
echo "some avg10=0.00 avg60=0.00 avg300=0.00 total=0" > "$base/proc/pressure/memory"
echo "some avg10=0.00 avg60=0.00 avg300=0.00 total=0" > "$base/proc/pressure/io"

# ===== fd.rs (RT-07) =====
mkdir -p "$base/proc/sys/fs"
echo "1234 0 123456"          > "$base/proc/sys/fs/file-nr"
echo "65536"                   > "$base/proc/sys/fs/file-max"
echo "5678 0"                  > "$base/proc/sys/fs/inode-nr"
echo "5678 0 0 0 0 0 0"       > "$base/proc/sys/fs/inode-state"
echo "100 50 0 0 0 0"          > "$base/proc/sys/fs/dentry-state"
echo "0"                       > "$base/proc/sys/fs/inode-max"
echo "1048576"                 > "$base/proc/sys/fs/nr_open"

# ===== tmpfs.rs (RT-08) =====
mkdir -p "$base/dev/shm" "$base/tmp" "$base/run/user" "$base/run/lock"
touch "$base/dev/shm/.placeholder" "$base/tmp/.placeholder" \
      "$base/run/user/.placeholder" "$base/run/lock/.placeholder"

# ===== mount.rs (RT-09) =====
mkdir -p "$base/proc/1"
cat > "$base/proc/mounts" <<'EOF'
rootfs / rootfs rw 0 0
/dev/sda1 / ext4 rw,relatime 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
tmpfs /tmp tmpfs rw,nosuid,nodev 0 0
EOF
cat > "$base/proc/1/mountinfo" <<'EOF'
1 1 8:1 / / rw,relatime - ext4 /dev/sda1 rw
EOF
echo "nodev  sysfs"           > "$base/proc/filesystems"
echo "ext4"                    >> "$base/proc/filesystems"
echo "/dev/sda1 / ext4 rw,relatime 0 0" > "$base/proc/1/mounts"
echo "device /dev/sda1 mounted on / with fstype ext4" > "$base/proc/1/mountstats"

# ===== block.rs (RT-10) =====
cat > "$base/proc/diskstats" <<'EOF'
   8       0 sda 1000 200 20000 500 500 300 10000 200 0 300 700
   8       1 sda1 500 100 10000 250 500 300 10000 200 0 150 350
EOF
echo "major minor  #blocks  name" > "$base/proc/partitions"
echo "   8     0   50000000 sda"  >> "$base/proc/partitions"
echo "   8     1   25000000 sda1" >> "$base/proc/partitions"

# ===== netdev.rs (RT-11) =====
for iface in lo eth0; do
    d="$base/sys/class/net/$iface"
    mkdir -p "$d/statistics"
    echo "1"                  > "$d/ifindex"
    echo "up"                 > "$d/operstate"
    echo "00:00:00:00:00:00"  > "$d/address"
    echo "0x1003"             > "$d/flags"
    echo "1500"               > "$d/mtu"
    if [ "$iface" = "eth0" ]; then
        echo "1000"           > "$d/speed"
    fi
    for s in rx_bytes rx_packets tx_bytes tx_packets rx_dropped tx_dropped; do
        echo "1000000"        > "$d/statistics/$s"
    done
done
mkdir -p "$base/proc/net"
cat > "$base/proc/net/route" <<'EOF'
Iface   Destination     Gateway         Flags   RefCnt  Use     Metric  Mask            MTU     Window  IRTT
eth0    00000000        0101A8C0        0003    0       0       100     00000000        0       0       0
lo      00000000        00000000        0001    0       0       0       00000000        0       0       0
EOF
cat > "$base/proc/net/arp" <<'EOF'
IP address       HW type     Flags       HW address            Mask     Device
192.168.1.1     0x1         0x2         00:11:22:33:44:55     *        eth0
EOF
echo "" > "$base/proc/net/ipv6_route"
cat > "$base/proc/net/netstat" <<'EOF'
TcpExt: SyncookiesSent SyncookiesRecv SyncookiesFailed
TcpExt: 0 0 0
EOF

# ===== socket.rs (RT-12) =====
for f in tcp tcp6 udp udp6 raw raw6 unix snmp snmp6; do
    echo "" > "$base/proc/net/$f"
done
mkdir -p "$base/proc/sys/net/core"
echo "4096" > "$base/proc/sys/net/core/somaxconn"

# ===== netfilter.rs (RT-13) =====
mkdir -p "$base/proc/net/stat" "$base/proc/sys/net"
echo "" > "$base/proc/net/nf_conntrack"
echo "0" > "$base/proc/net/stat/nf_conntrack"
echo "65536" > "$base/proc/sys/net/nf_conntrack_max"
echo "" > "$base/proc/net/nf_tables_names"
echo "" > "$base/proc/net/xfrm_stat"

# ===== ipc_ns_cg.rs (RT-14) =====
mkdir -p "$base/proc/sysvipc" "$base/proc/1/ns"
echo "" > "$base/proc/sysvipc/shm"
echo "" > "$base/proc/sysvipc/msg"
echo "" > "$base/proc/sysvipc/sem"
echo "ipc:[4026531839]"       > "$base/proc/1/ns/ipc"
echo "0::/system.slice/sshd.service" > "$base/proc/1/cgroup"

# ===== session.rs (RT-15) =====
mkdir -p "$base/var/run" "$base/var/log"
touch "$base/var/run/utmp" "$base/var/log/wtmp" "$base/var/log/btmp"

# ===== security.rs (RT-16) =====
mkdir -p "$base/proc/sys/kernel/random" "$base/etc" "$base/proc/sys/net/ipv4" "$base/proc/sys/net/ipv6/conf/all"
echo "256" > "$base/proc/sys/kernel/random/entropy_avail"
echo "4096" > "$base/proc/sys/kernel/random/poolsize"
echo "64" > "$base/proc/sys/kernel/random/read_wakeup_threshold"
echo "hosts: files dns"       > "$base/etc/nsswitch.conf"
echo "multi on"               > "$base/etc/host.conf"
echo "0" > "$base/proc/sys/net/ipv4/ip_forward"
echo "0" > "$base/proc/sys/net/ipv6/conf/all/forwarding"

# ===== device.rs (RT-17) =====
cat > "$base/proc/devices" <<'EOF'
Character devices:
  1 mem
  4 tty
Block devices:
  8 sd
EOF
echo "" > "$base/proc/misc"
mkdir -p "$base/sys/kernel/debug"
touch "$base/sys/kernel/debug/.placeholder"

# ===== power.rs (RT-18) =====
mkdir -p "$base/sys/power" "$base/proc/acpi"
echo "mem disk"               > "$base/sys/power/state"
echo "s2idle shallow deep"    > "$base/sys/power/mem_sleep"
echo ""                       > "$base/proc/acpi/wakeup"

# ===== time.rs (RT-19) =====
mkdir -p "$base/proc/driver"  "$base/var/spool/cron"
echo "rtc_time: 12:00:00"    > "$base/proc/driver/rtc"
echo "rtc_date: 2026-01-01"  >> "$base/proc/driver/rtc"
echo "TZif0"                  > "$base/etc/localtime"
echo "0.0 0 0"                > "$base/etc/adjtime"
echo ""                       > "$base/etc/crontab"
touch "$base/var/spool/cron/.placeholder"

# ===== events.rs (RT-20) =====
mkdir -p "$base/dev"
echo "<6>[  100.000000] mock dmesg entry" > "$base/dev/kmsg"

# ===== cache.rs (RT-21) =====
echo "nameserver 8.8.8.8"    > "$base/etc/resolv.conf"
echo "127.0.0.1 localhost"   > "$base/etc/hosts"
touch "$base/etc/ld.so.cache" "$base/etc/ld.so.conf"

echo "fixtures/normal: all 21 collector probe paths created"
