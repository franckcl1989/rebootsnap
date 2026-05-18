# rebootsnap

RebootSnap captures a pre-reboot snapshot of Linux runtime state, preserving key system evidence for post-restart troubleshooting and root cause analysis.

## Quick Start

```bash
# Build (requires Rust toolchain)
cargo build --release

# Run (captures snapshot to /tmp)
target/release/rebootsnap /tmp
# Output: /tmp/rebootsnap-20260518-120000.tar.gz
```

## Requirements

- Linux kernel 4.18+ (target: Rocky Linux 8.x, systemd >= 239)
- Rust toolchain (edition 2024)
- Root privileges or `CAP_SYS_ADMIN` for full dmesg/netlink/journal access

## Output

The tar.gz archive contains 21 collector output files + manifest.json + summary.json:

| File | Contents |
|---|---|
| `boot.json` | Hostname, kernel version, boot ID, uptime |
| `kernel.json` | Kernel parameters, modules, taint, watchdog, kexec/crash |
| `systemd.json` | Unit states, job queue, inhibitors, failed unit details |
| `processes.jsonl` | Per-process identity, state, scheduling, limits, namespaces, cgroup |
| `cpu.json` | CPU statistics, interrupts, softirqs, pressure, topology |
| `memory.json` | Memory usage, swap, NUMA, zram, THP, OOM, KSM, hugepages |
| `fds.json` | File descriptor and inode counters, locks |
| `tmpfs.json` | tmpfs mount stats, runtime directory entry counts |
| `mounts.json` | Mount topology, mountstats, filesystem types |
| `block.json` | Disk I/O, partition table, per-device queue/dm/loop/zram info |
| `netdev.json` | Network interfaces, routing, ARP, netstat, IP addresses, tunnels |
| `sockets.json` | TCP/UDP/Unix socket tables, protocol counters |
| `netfilter.json` | conntrack, iptables/nftables tables, qdisc, XFRM |
| `ipc_ns_cg.json` | SysV IPC, cgroup v1/v2 hierarchy, POSIX mqueue |
| `sessions.json` | utmp/wtmp/btmp records, logind sessions/seats |
| `security.json` | LSM, seccomp, audit, IMA, lockdown, network security sysctls |
| `devices.json` | Device enumeration, sysfs driver/model info |
| `power.json` | CPU frequency, thermal zones, cpuidle, EDAC, throttle |
| `time.json` | Clocksource, RTC, time sync (NTP) status, cron |
| `dmesg.json` | dmesg ring buffer, printk parameters, volatile journal files |
| `caches.json` | Slab, dentry, inode cache, page cache stats |

All output files are permission-restricted (0600).

## Known Limitations (0.1.0)

- **XFRM SA/Policy**: `NETLINK_XFRM` protocol family not yet implemented in Rust netlink crates. XFRM stats are collected from `/proc/net/xfrm_stat` only.
- **nftables full rule dump**: `neli` crate establishes NETLINK_NETFILTER socket connection but nftables message body codec is deferred to 0.2.0. Table names and iptables tables are collected.
- **systemd volatile journal**: Journal file list is enumerated from `/run/log/journal/` but journal content extraction requires `GetJournal()` D-Bus fd handling (deferred).

## Documentation

- [文档索引](docs/index.md)
- [人与 AI 共治规则](docs/project-governance.md)
- [变更记录](CHANGELOG.md)
- [变更管理规范](docs/change-management.md)
- [Linux 服务器操作系统运行时信息大类定义](docs/linux-runtime-info-categories.md)
- [Linux OS 运行时信息二级分类](docs/linux-runtime-info-subcategories.md)
- [Linux OS 运行时信息三级分类与候选采集项](docs/linux-runtime-info-collection-items.md)
- [Linux OS 运行时信息最终采集决策](docs/linux-runtime-info-collection-decision.md)
- [Collector 安全治理](docs/collector-security-governance.md)
- [Collector 测试治理](docs/collector-testing-governance.md)
- [Collector 架构](docs/collector-architecture.md)
- [Phase A 实现审核报告](docs/phase-a-review.md)
- [设计决策记录](docs/decisions/README.md)
- [ADR 0001：Linux OS 运行时信息边界](docs/decisions/0001-runtime-info-boundary.md)
- [ADR 0002：人与 AI 共治执行模型](docs/decisions/0002-human-ai-governance.md)
- [ADR 0003：实现技术栈](docs/decisions/0003-implementation-tech-stack.md)
- [ADR 0004：输出格式](docs/decisions/0004-output-format.md)

## Development

```bash
scripts/setup-dev.sh   # install git hooks
scripts/verify.sh      # run all checks (build, clippy, tests, docs)
cargo test --workspace # 21 unit + 4 integration tests
```
