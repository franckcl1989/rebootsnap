# Linux OS 运行时信息三级分类与候选采集项

## 文档定位

本文在 `docs/linux-runtime-info-subcategories.md` 的二级分类之下，定义完整的三级候选采集项组。

本文回答一个问题：如果后续要从 Linux OS 运行时现场中选择最终采集范围，每个 `RT-xx-yy` 二级分类下有哪些可稳定引用的候选采集项组。

本文不做最终采集决策，不定义采集命令、字段名、输出 schema、权限模型、脱敏规则、存储格式、报告布局或产品交互。后续最终采集范围必须从本文筛选，并在单独的采集决策文档中说明取舍。

## 继承与对齐

- 本文继承 `docs/linux-runtime-info-categories.md` 的一级大类边界。
- 本文逐项挂靠 `docs/linux-runtime-info-subcategories.md` 的二级分类。
- 三级 ID 使用 `RT-xx-yy-zz`，其中 `RT-xx` 是一级大类，`RT-xx-yy` 是二级分类，`zz` 是候选采集项组序号。
- 每个三级项必须只归属于一个二级分类。跨分类关系在后续数据模型或报告中通过引用表达，不改变主归属。
- 三级项是候选采集项组，不等同于具体命令、字段或实现接口。

## 完整性口径

本文中的“完整”指分类和候选采集项组完整：

- 覆盖 `RT-01` 到 `RT-21` 的全部一级大类。
- 覆盖现有全部 `RT-xx-yy` 二级分类。
- 每个二级分类至少定义一个可筛选、可追溯的三级候选采集项组。
- 每个三级项都能回到官方文档和上游源码路径进行二次核对。

本文中的“完整”不表示每个 Linux 发行版、内核版本、构建选项或权限条件都能实际采集全部项目。后续 collector 设计必须单独处理能力探测、权限边界、敏感信息、脱敏和失败降级。

## 二次确认依据组

每个候选采集项组的 `依据组` 同时代表官方文档入口和源码二次确认路径。后续修改三级项时，必须先确认对应依据组仍能支撑该项边界。

上游入口：

- Linux kernel documentation: https://docs.kernel.org/
- Linux kernel source tree: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/
- Linux man-pages: https://man7.org/linux/man-pages/
- systemd documentation: https://www.freedesktop.org/software/systemd/man/
- systemd source tree: https://github.com/systemd/systemd/tree/main/src
- OpenSSH portable source tree: https://github.com/openssh/openssh-portable
- Linux-PAM source tree: https://github.com/linux-pam/linux-pam
- MIT Kerberos documentation: https://web.mit.edu/kerberos/krb5-latest/doc/
- sudo source tree: https://github.com/sudo-project/sudo

| 依据组 | 官方文档入口 | 源码二次确认路径 |
| --- | --- | --- |
| `E-BOOT` | Linux kernel boot/admin-guide、`proc(5)`、systemd boot/unit 文档 | Linux `init/main.c`、`kernel/reboot.c`、`fs/proc/`；systemd `src/core/` |
| `E-PROC` | Linux kernel procfs 文档、`proc(5)` | Linux `fs/proc/`、`kernel/pid.c`、`kernel/fork.c`、`kernel/signal.c` |
| `E-SYSCTL` | Linux kernel sysctl 文档 | Linux `fs/proc/proc_sysctl.c`、`kernel/sysctl.c`、各子系统 sysctl table |
| `E-SCHED` | Linux scheduler、IRQ、PSI 文档 | Linux `kernel/sched/`、`kernel/irq/`、`kernel/softirq.c`、`kernel/sched/psi.c` |
| `E-MM` | Linux memory management、zswap、hugetlb、PSI 文档 | Linux `mm/`、`include/linux/mm.h`、`kernel/sched/psi.c` |
| `E-FD` | `proc(5)`、`epoll(7)`、`inotify(7)`、`fanotify(7)`、`io_uring` 文档 | Linux `fs/file.c`、`fs/eventpoll.c`、`fs/notify/`、`io_uring/` |
| `E-VFS` | Linux VFS、filesystems、mount namespace 文档 | Linux `fs/namespace.c`、`fs/super.c`、`fs/mount.h`、filesystem drivers |
| `E-BLOCK` | Linux block layer、device-mapper、NVMe、SCSI 文档 | Linux `block/`、`drivers/md/`、`drivers/nvme/`、`drivers/scsi/` |
| `E-NETDEV` | Linux networking、rtnetlink、iproute2-facing kernel docs | Linux `net/core/dev.c`、`net/core/rtnetlink.c`、`net/ipv4/`、`net/ipv6/` |
| `E-SOCKET` | `socket(7)`、`tcp(7)`、`udp(7)`、`unix(7)`、`netlink(7)` | Linux `net/socket.c`、`net/ipv4/tcp*.c`、`net/ipv4/udp.c`、`net/unix/`、`net/netlink/` |
| `E-NETFILTER` | nftables/netfilter、conntrack、tc、XFRM 文档 | Linux `net/netfilter/`、`net/sched/`、`net/xfrm/`、`net/core/filter.c` |
| `E-IPC-NS-CG` | `namespaces(7)`、`cgroups(7)`、cgroup v2 文档、IPC man-pages | Linux `ipc/`、`kernel/nsproxy.c`、`kernel/cgroup/`、`fs/mqueue/` |
| `E-LOGIN` | systemd-logind、PAM、OpenSSH、utmp/loginuid 文档 | systemd `src/login/`；Linux `kernel/audit/`；OpenSSH portable source |
| `E-SECURITY` | `capabilities(7)`、seccomp、LSM、audit、keyrings、IMA/EVM 文档 | Linux `security/`、`kernel/capability.c`、`kernel/seccomp.c`、`kernel/audit*`、`security/keys/` |
| `E-DEVICE` | Linux driver model、udev/systemd device 文档 | Linux `drivers/base/`、`drivers/pci/`、`drivers/usb/`、`drivers/firmware/`；systemd `src/udev/` |
| `E-POWER` | CPUFreq、cpuidle、thermal、power supply、RAS 文档 | Linux `drivers/cpufreq/`、`drivers/cpuidle/`、`drivers/thermal/`、`drivers/edac/`、`drivers/ras/` |
| `E-TIME` | Linux timekeeping、timer、workqueue、systemd timer 文档 | Linux `kernel/time/`、`kernel/workqueue.c`；systemd `src/core/timer.c` |
| `E-TRACE` | ftrace、perf、BPF ring buffer、audit backlog 文档 | Linux `kernel/trace/`、`kernel/events/`、`kernel/bpf/`、`kernel/audit*` |
| `E-CACHE` | procfs memory/cache docs、resolver/NSS/systemd-resolved 文档 | Linux `mm/`、`fs/dcache.c`、`fs/inode.c`、`net/`；systemd `src/resolve/` |
| `E-SYSTEMD` | systemd unit、service、socket、path、timer、journal、resolved、logind 文档 | systemd `src/core/`、`src/journal/`、`src/resolve/`、`src/login/`、`src/udev/` |
| `E-AUTH` | sudo、PAM、MIT Kerberos credential cache、OpenSSH 文档 | sudo source、Linux-PAM source、MIT krb5 source、OpenSSH portable source |

## 三级候选采集项

### RT-01 启动实例、系统身份与全局基线

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-01-01` 启动实例身份 | `RT-01-01-01` boot 实例唯一标识；`RT-01-01-02` boot 生命周期边界与重启指纹 | `E-BOOT`、`E-PROC` |
| `RT-01-02` 全局运行时间关系 | `RT-01-02-01` uptime 与启动时间关系；`RT-01-02-02` wall/monotonic/boottime 全局偏移 | `E-BOOT`、`E-TIME` |
| `RT-01-03` 运行内核与启动参数生效基线 | `RT-01-03-01` 运行内核身份；`RT-01-03-02` 实际生效启动参数与全局内核基线 | `E-BOOT`、`E-SYSCTL` |
| `RT-01-04` 主机命名与系统域运行态 | `RT-01-04-01` hostname/domainname 生效值；`RT-01-04-02` machine-id、系统角色和本机身份上下文 | `E-BOOT`、`E-SYSTEMD` |

### RT-02 内核状态、运行时参数与动态内核对象

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-02-01` 内核运行时参数 | `RT-02-01-01` sysctl 运行参数快照；`RT-02-01-02` 参数约束、开关和限制状态 | `E-SYSCTL` |
| `RT-02-02` 已加载模块与模块运行参数 | `RT-02-02-01` 已加载模块身份和依赖；`RT-02-02-02` 模块参数、引用计数和 taint 关联 | `E-PROC`、`E-SYSCTL` |
| `RT-02-03` 内核完整性、taint 与热修补状态 | `RT-02-03-01` taint、lockdown、watchdog 和 lockup 状态；`RT-02-03-02` livepatch、kexec、crashkernel 和 panic 现场 | `E-SYSCTL`、`E-SECURITY` |
| `RT-02-04` 动态探针、追踪与性能事件对象 | `RT-02-04-01` kprobe/uprobe/ftrace 动态对象；`RT-02-04-02` perf event、dynamic debug 和观测附着关系 | `E-TRACE` |
| `RT-02-05` eBPF 程序、映射与链接对象 | `RT-02-05-01` BPF program/map/link 对象身份；`RT-02-05-02` BPF 引用、pin、attach 和运行计数 | `E-TRACE`、`E-NETFILTER` |

### RT-03 初始化系统、服务管理与本机控制面状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-03-01` unit 生命周期与依赖状态 | `RT-03-01-01` unit active/sub/load 状态；`RT-03-01-02` unit 依赖图和 ordering 当前结果 | `E-SYSTEMD` |
| `RT-03-02` job、事务与启动关闭队列 | `RT-03-02-01` job 和 transaction 队列；`RT-03-02-02` shutdown/reboot 阻塞与失败上下文 | `E-SYSTEMD` |
| `RT-03-03` transient unit、scope 与 slice 运行态 | `RT-03-03-01` transient unit/scope 身份；`RT-03-03-02` slice、scope 与进程/cgroup 关联 | `E-SYSTEMD`、`E-IPC-NS-CG` |
| `RT-03-04` 服务主进程、重启计数与失败状态 | `RT-03-04-01` service main/control PID 视图；`RT-03-04-02` exit、restart、watchdog 和失败结果 | `E-SYSTEMD`、`E-PROC` |
| `RT-03-05` socket、path 与 timer unit 运行态 | `RT-03-05-01` socket/path/timer unit 等待状态；`RT-03-05-02` activation 触发、关联和队列 | `E-SYSTEMD`、`E-TIME` |
| `RT-03-06` inhibitor、会话控制与本机管理锁 | `RT-03-06-01` inhibitor lock 身份和持有者；`RT-03-06-02` 关机、重启、睡眠和会话动作阻塞状态 | `E-SYSTEMD`、`E-LOGIN` |
| `RT-03-07` system bus 名称所有权与控制面连接 | `RT-03-07-01` system bus 名称所有权；`RT-03-07-02` 控制面 D-Bus 连接和服务暴露 | `E-SYSTEMD` |

### RT-04 进程、线程与执行上下文

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-04-01` 进程身份与生命周期状态 | `RT-04-01-01` PID/TGID/PPID 和状态；`RT-04-01-02` fork、exec、exit、zombie 和 wait 现场 | `E-PROC` |
| `RT-04-02` 父子关系、进程组、会话与控制终端引用 | `RT-04-02-01` 进程树和 parent/child 关系；`RT-04-02-02` pgrp、session、TTY 和作业控制引用 | `E-PROC`、`E-LOGIN` |
| `RT-04-03` 可执行镜像、启动参数与执行目录视图 | `RT-04-03-01` exe、cmdline 和 cwd/root 视图；`RT-04-03-02` environment 可见性和敏感边界标记 | `E-PROC`、`E-SECURITY` |
| `RT-04-04` 线程集合与线程执行状态 | `RT-04-04-01` task/TID 集合；`RT-04-04-02` 线程状态、wchan、blocked point 和等待原因 | `E-PROC`、`E-SCHED` |
| `RT-04-05` 进程级调度属性与 CPU 亲和性 | `RT-04-05-01` scheduler policy、priority、nice 和 rt 属性；`RT-04-05-02` CPU affinity、cpuset 和 NUMA 策略引用 | `E-PROC`、`E-SCHED` |
| `RT-04-06` 资源限制、运行计数与退出上下文 | `RT-04-06-01` rlimit 和资源使用计数；`RT-04-06-02` pending signal、exit code 和父进程回收上下文 | `E-PROC` |
| `RT-04-07` 隔离、资源控制与路径视图引用 | `RT-04-07-01` namespace/cgroup 引用；`RT-04-07-02` mount/root/path 视图和隔离边界 | `E-PROC`、`E-IPC-NS-CG`、`E-VFS` |

### RT-05 CPU、调度、中断与负载状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-05-01` CPU 在线状态与执行容量 | `RT-05-01-01` CPU online/offline 和 present/possible mask；`RT-05-01-02` capacity、isolated、nohz 和 SMT 运行态 | `E-SCHED`、`E-SYSCTL` |
| `RT-05-02` 调度队列、负载与运行压力 | `RT-05-02-01` run queue 和 runnable 积压；`RT-05-02-02` load average、sched pressure 和 runnable latency | `E-SCHED` |
| `RT-05-03` 调度统计、迁移与上下文切换 | `RT-05-03-01` context switch、preempt 和 migration 统计；`RT-05-03-02` per-CPU/per-task 调度计数 | `E-SCHED`、`E-PROC` |
| `RT-05-04` 中断、软中断与 backlog | `RT-05-04-01` IRQ 分布、亲和性和计数；`RT-05-04-02` softirq、tasklet、NAPI backlog 和丢包侧信号 | `E-SCHED`、`E-NETDEV` |
| `RT-05-05` CPU wait、steal、iowait 与 PSI | `RT-05-05-01` iowait、steal 和 CPU stall；`RT-05-05-02` CPU PSI avg/total 和触发器状态 | `E-SCHED` |
| `RT-05-06` 调度域与 CPU 侧均衡状态 | `RT-05-06-01` sched domain、group 和 mask；`RT-05-06-02` load balance、NUMA balance 和 CPU 组织关系 | `E-SCHED` |

### RT-06 内存、虚拟内存、swap 与内存压力状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-06-01` 物理内存占用与可用性 | `RT-06-01-01` free/available/used/reserved 内存；`RT-06-01-02` dirty、writeback、anon/file 页和内存水位 | `E-MM`、`E-PROC` |
| `RT-06-02` zone、node、NUMA 与页分配状态 | `RT-06-02-01` zone/node 水位和页分配状态；`RT-06-02-02` NUMA 拓扑、迁移和 per-node 压力 | `E-MM` |
| `RT-06-03` 进程内存映射与进程级内存占用 | `RT-06-03-01` VMA/map 和权限视图；`RT-06-03-02` RSS/PSS/USS、anon/file/shared 和 swap 使用 | `E-MM`、`E-PROC` |
| `RT-06-04` swap、zswap、zram 与交换工作集 | `RT-06-04-01` swap device、usage 和 in/out 统计；`RT-06-04-02` zswap/zram 后端、压缩和工作集状态 | `E-MM`、`E-BLOCK` |
| `RT-06-05` 页回收、规整、碎片与 THP 状态 | `RT-06-05-01` reclaim、compaction 和 fragmentation；`RT-06-05-02` THP、hugetlb、KSM 和大页运行态 | `E-MM` |
| `RT-06-06` OOM、内存压力与内存 stall | `RT-06-06-01` OOM killer、分配失败和 oom score 上下文；`RT-06-06-02` memory PSI、压力触发器和 stall 现场 | `E-MM`、`E-SCHED` |

### RT-07 打开句柄、文件引用与内核同步对象

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-07-01` fd 表与打开文件引用 | `RT-07-01-01` 进程 fd 表和 open file 引用；`RT-07-01-02` offset、flags、mode 和引用目标 | `E-FD`、`E-PROC` |
| `RT-07-02` 已删除文件、匿名文件与 memfd 引用 | `RT-07-02-01` deleted-but-open 文件引用；`RT-07-02-02` anonymous inode、memfd 和 sealed file 状态 | `E-FD`、`E-VFS` |
| `RT-07-03` pipe、eventfd、timerfd、signalfd 与 pidfd | `RT-07-03-01` pipe/eventfd/timerfd/signalfd 对象；`RT-07-03-02` pidfd 目标、计数、等待和触发状态 | `E-FD`、`E-TIME` |
| `RT-07-04` epoll、inotify、fanotify 与 io_uring 对象 | `RT-07-04-01` epoll interest、ready list 和 waiters；`RT-07-04-02` inotify/fanotify/io_uring 队列和引用 | `E-FD` |
| `RT-07-05` 文件锁、lease、futex 与同步关系 | `RT-07-05-01` flock/POSIX lock/lease 持有与等待；`RT-07-05-02` futex wait、owner 和同步阻塞关系 | `E-FD`、`E-VFS` |
| `RT-07-06` socket fd 引用 | `RT-07-06-01` socket fd 到 socket inode 的引用；`RT-07-06-02` socket fd flags、owner 和进程持有关系 | `E-FD`、`E-SOCKET` |

### RT-08 临时文件系统与运行时目录状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-08-01` tmpfs 与易失文件系统实例 | `RT-08-01-01` tmpfs/devtmpfs/ramfs 实例；`RT-08-01-02` 容量、占用、inode 和挂载生命周期 | `E-VFS`、`E-MM` |
| `RT-08-02` 运行时目录与守护进程状态目录 | `RT-08-02-01` `/run` 和守护进程 runtime dir；`RT-08-02-02` owner、mode、生命周期和服务关联 | `E-VFS`、`E-SYSTEMD` |
| `RT-08-03` PID 文件、锁文件与协调文件 | `RT-08-03-01` PID/lock/state marker 文件身份；`RT-08-03-02` 文件内容指向、锁持有和失效关系 | `E-VFS`、`E-FD` |
| `RT-08-04` Unix socket 文件与运行时入口 | `RT-08-04-01` Unix socket 路径入口；`RT-08-04-02` socket 文件权限、owner 和服务端引用 | `E-VFS`、`E-SOCKET` |
| `RT-08-05` tmpfs 共享内存目录项与临时挂载点 | `RT-08-05-01` tmpfs shm 目录项和权限；`RT-08-05-02` 临时挂载点目录项、引用和所有权 | `E-VFS`、`E-IPC-NS-CG` |
| `RT-08-06` 残留、失效与清理策略运行态 | `RT-08-06-01` orphan/stale runtime 文件；`RT-08-06-02` tmpfiles/cleanup 策略当前效果和待清理对象 | `E-VFS`、`E-SYSTEMD` |

### RT-09 VFS、文件系统与挂载状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-09-01` 挂载拓扑与挂载选项 | `RT-09-01-01` mount tree 和 mount ID；`RT-09-01-02` mount options、rw/ro、propagation 和路径暴露 | `E-VFS` |
| `RT-09-02` mount namespace 视图与传播关系 | `RT-09-02-01` mount namespace 挂载视图；`RT-09-02-02` shared/private/slave propagation 和 bind 关系 | `E-VFS`、`E-IPC-NS-CG` |
| `RT-09-03` superblock 与文件系统运行状态 | `RT-09-03-01` superblock 和 filesystem instance；`RT-09-03-02` fs feature、state、errors policy 和健康标志 | `E-VFS` |
| `RT-09-04` journal、quota、dirty inode 与写回状态 | `RT-09-04-01` journal/quota 当前状态；`RT-09-04-02` dirty inode、writeback 和提交积压 | `E-VFS`、`E-MM` |
| `RT-09-05` overlay、FUSE、bind 与组合挂载关系 | `RT-09-05-01` overlay upper/lower/work 关系；`RT-09-05-02` FUSE、bind 和组合路径映射 | `E-VFS` |
| `RT-09-06` 文件系统错误、只读切换与保护状态 | `RT-09-06-01` fs error、recovery 和 remount-ro 现场；`RT-09-06-02` 保护性降级、挂起和可写性状态 | `E-VFS` |

### RT-10 块设备、存储映射与 I/O 路径状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-10-01` 块设备身份、可用性与队列状态 | `RT-10-01-01` block device identity、size 和 holder/slave；`RT-10-01-02` request queue 基础状态和打开关系 | `E-BLOCK` |
| `RT-10-02` I/O scheduler、request queue 与拥塞状态 | `RT-10-02-01` I/O scheduler 和 queue depth；`RT-10-02-02` merge、dispatch、congestion 和 blk-mq 状态 | `E-BLOCK` |
| `RT-10-03` device-mapper、dm-crypt 与 LVM 映射 | `RT-10-03-01` dm target、table 和 dependency；`RT-10-03-02` dm-crypt/LVM 激活、参数和映射健康 | `E-BLOCK` |
| `RT-10-04` mdraid、multipath 与路径选择状态 | `RT-10-04-01` mdraid array、member 和 rebuild；`RT-10-04-02` multipath path、policy、failover 和 degraded 状态 | `E-BLOCK` |
| `RT-10-05` loop、zram 与虚拟块设备 | `RT-10-05-01` loop 后端文件和 flags；`RT-10-05-02` zram 压缩、内存后端和虚拟块设备状态 | `E-BLOCK`、`E-MM` |
| `RT-10-06` NVMe、SCSI、iSCSI、NBD 与存储传输运行态 | `RT-10-06-01` controller/session/namespace 身份；`RT-10-06-02` command queue、transport error、path 和 timeout 状态 | `E-BLOCK`、`E-DEVICE` |
| `RT-10-07` I/O 错误、延迟、计数器与压力 | `RT-10-07-01` block error、retry、timeout 和 latency；`RT-10-07-02` I/O stats、io PSI 和队列压力 | `E-BLOCK`、`E-SCHED` |

### RT-11 网络接口、链路、地址与路由状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-11-01` 接口身份、链路状态与队列 | `RT-11-01-01` netdev identity、flags、MTU、MAC 和 carrier；`RT-11-01-02` queue、qdisc root、offload 和 driver link state | `E-NETDEV` |
| `RT-11-02` 二层虚拟设备与链路组合 | `RT-11-02-01` bridge/bond/vlan/macvlan/ipvlan/veth/tap/tun 关系；`RT-11-02-02` master/slave、peer 和 lower/upper link | `E-NETDEV` |
| `RT-11-03` IP 地址、作用域与临时地址 | `RT-11-03-01` IPv4/IPv6 address、scope 和 lifetime；`RT-11-03-02` tentative/deprecated/temporary 地址和选择上下文 | `E-NETDEV` |
| `RT-11-04` 路由表、策略路由与路径选择结果 | `RT-11-04-01` route table、rule 和 FIB 结果；`RT-11-04-02` policy routing、multipath 和 route exception | `E-NETDEV` |
| `RT-11-05` 邻居表、ARP、NDP 与二层解析缓存 | `RT-11-05-01` neighbor entry、ARP/NDP 状态；`RT-11-05-02` reachability、timer、queue 和失效现场 | `E-NETDEV` |
| `RT-11-06` 网络命名空间内接口视图 | `RT-11-06-01` per-netns interface/address/route 视图；`RT-11-06-02` netns link、peer 和 process 引用 | `E-NETDEV`、`E-IPC-NS-CG` |
| `RT-11-07` 接口统计、错误、丢包与链路压力 | `RT-11-07-01` rx/tx stats、drop、error 和 collision；`RT-11-07-02` queue backlog、NAPI 压力和 driver counters | `E-NETDEV`、`E-SCHED` |
| `RT-11-08` 隧道、封装与内核网络端点状态 | `RT-11-08-01` vxlan/geneve/gre/ipip/sit/wireguard endpoint；`RT-11-08-02` encapsulation 参数、peer、key 和运行计数 | `E-NETDEV` |

### RT-12 socket、传输协议与连接状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-12-01` 监听 socket 与绑定端点 | `RT-12-01-01` listening socket、bind address 和 port；`RT-12-01-02` listen backlog、accept queue 和 owner | `E-SOCKET`、`E-PROC` |
| `RT-12-02` TCP、MPTCP、SCTP 等连接与传输状态机 | `RT-12-02-01` transport connection state、endpoint 和 inode；`RT-12-02-02` window、retransmit、MPTCP/SCTP path 和 congestion state | `E-SOCKET` |
| `RT-12-03` UDP、raw、netlink 与 Unix socket 协议状态 | `RT-12-03-01` UDP/raw/netlink/unix socket identity；`RT-12-03-02` peer、group、queue 和 protocol-specific state | `E-SOCKET` |
| `RT-12-04` socket buffer 与发送接收队列 | `RT-12-04-01` send/receive buffer 占用和限制；`RT-12-04-02` skb queue、memory pressure 和 waiters | `E-SOCKET`、`E-MM` |
| `RT-12-05` TIME_WAIT、orphan、SYN backlog 与异常连接集合 | `RT-12-05-01` TIME_WAIT、orphan 和 half-open 集合；`RT-12-05-02` SYN backlog、overflow、reset 和异常计数 | `E-SOCKET` |
| `RT-12-06` 协议栈计数器与拥塞控制运行态 | `RT-12-06-01` TCP/UDP/IP 协议计数器；`RT-12-06-02` congestion control algorithm、ECN 和异常状态 | `E-SOCKET`、`E-NETDEV` |
| `RT-12-07` socket inode 与进程关联 | `RT-12-07-01` socket inode 到协议对象映射；`RT-12-07-02` process/fd/cgroup/netns 关联 | `E-SOCKET`、`E-FD`、`E-IPC-NS-CG` |

### RT-13 包过滤、NAT、连接跟踪与流量控制状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-13-01` 过滤规则集与规则命中状态 | `RT-13-01-01` nftables/iptables/ebtables 生效规则集；`RT-13-01-02` chain、hook、priority 和 counter 命中 | `E-NETFILTER` |
| `RT-13-02` set、map、flowtable 与规则辅助对象 | `RT-13-02-01` nft set/map/flowtable identity；`RT-13-02-02` dynamic element、timeout、size 和引用关系 | `E-NETFILTER` |
| `RT-13-03` conntrack、NAT 映射与连接跟踪表 | `RT-13-03-01` conntrack entry、tuple 和 state；`RT-13-03-02` NAT mapping、timeout、zone 和 table pressure | `E-NETFILTER` |
| `RT-13-04` IPVS 与内核负载均衡状态 | `RT-13-04-01` IPVS virtual service 和 scheduler；`RT-13-04-02` real server、connection table 和 health/fail state | `E-NETFILTER` |
| `RT-13-05` qdisc、class、filter 与流量队列 | `RT-13-05-01` qdisc/class/filter hierarchy；`RT-13-05-02` shaping、rate、drop、backlog 和 queue stats | `E-NETFILTER`、`E-NETDEV` |
| `RT-13-06` XDP、tc 附着点与数据面程序引用 | `RT-13-06-01` XDP/tc attach point、mode 和 order；`RT-13-06-02` referenced BPF program、map 和 hit/drop counters | `E-NETFILTER`、`E-TRACE` |
| `RT-13-07` XFRM、IPsec state 与 policy | `RT-13-07-01` XFRM state、SA 和 replay/window；`RT-13-07-02` XFRM policy、selector 和 transform path | `E-NETFILTER`、`E-NETDEV` |
| `RT-13-08` 包标记、限流与数据面计数器 | `RT-13-08-01` packet mark、connmark、classid 和 quota；`RT-13-08-02` rate limit、hashlimit、recent 和临时计数 | `E-NETFILTER` |

### RT-14 IPC、namespace 与 cgroup 状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-14-01` SysV IPC 与 POSIX IPC 对象 | `RT-14-01-01` SysV shm/msg/sem identity 和权限；`RT-14-01-02` POSIX mq/shm identity、容量和引用 | `E-IPC-NS-CG` |
| `RT-14-02` namespace 实例与引用关系 | `RT-14-02-01` pid/net/mnt/uts/ipc/user/cgroup/time namespace identity；`RT-14-02-02` namespace 引用进程和生命周期 | `E-IPC-NS-CG` |
| `RT-14-03` user namespace ID 映射与权限边界 | `RT-14-03-01` uid_map/gid_map/setgroups 状态；`RT-14-03-02` capability 解释边界和 parent namespace 关系 | `E-IPC-NS-CG`、`E-SECURITY` |
| `RT-14-04` cgroup 层级、controller 与生效配置 | `RT-14-04-01` cgroup tree、type 和 controller enablement；`RT-14-04-02` controller config、inheritance 和 effective state | `E-IPC-NS-CG` |
| `RT-14-05` cgroup 成员关系与资源限制 | `RT-14-05-01` cgroup.procs/tasks 成员；`RT-14-05-02` CPU/memory/io/pids 等资源限制当前效果 | `E-IPC-NS-CG`、`E-SCHED`、`E-MM` |
| `RT-14-06` cgroup 统计、事件与 pressure | `RT-14-06-01` cgroup resource usage、events 和 peak；`RT-14-06-02` cgroup PSI、OOM、throttle 和 pressure | `E-IPC-NS-CG`、`E-SCHED` |
| `RT-14-07` IPC 等待、引用与清理状态 | `RT-14-07-01` IPC waiters、owners 和 refcount；`RT-14-07-02` stale IPC、marked-for-delete 和 cleanup state | `E-IPC-NS-CG` |

### RT-15 用户登录、会话与临时认证状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-15-01` 当前登录用户、session 与 seat | `RT-15-01-01` logind session、seat 和 user runtime；`RT-15-01-02` session lifecycle、type、remote/local 和 leader | `E-LOGIN`、`E-SYSTEMD` |
| `RT-15-02` TTY、PTY 与控制终端关系 | `RT-15-02-01` TTY/PTY device、foreground pgrp 和 owner；`RT-15-02-02` controlling terminal、job control 和 hangup 状态 | `E-LOGIN`、`E-PROC` |
| `RT-15-03` SSH 与远程登录运行态 | `RT-15-03-01` sshd session process、pty 和 remote endpoint；`RT-15-03-02` agent forwarding、environment 和 auth context 敏感标记 | `E-LOGIN`、`E-AUTH`、`E-SOCKET` |
| `RT-15-04` PAM session、loginuid 与 utmp 当前记录 | `RT-15-04-01` PAM session open/close 运行态；`RT-15-04-02` loginuid、utmp 和审计会话上下文 | `E-LOGIN`、`E-SECURITY` |
| `RT-15-05` 用户级 systemd manager 与 session bus | `RT-15-05-01` user@.service manager 状态；`RT-15-05-02` user bus、user units 和 session 关联 | `E-SYSTEMD`、`E-LOGIN` |
| `RT-15-06` sudo、Kerberos 与临时认证材料 | `RT-15-06-01` sudo timestamp/cache 状态；`RT-15-06-02` Kerberos ccache、SSH agent socket 和临时凭据敏感标记 | `E-AUTH`、`E-SECURITY` |

### RT-16 安全策略、凭据与访问控制运行态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-16-01` LSM 模式、策略加载与约束状态 | `RT-16-01-01` SELinux/AppArmor/Landlock/BPF LSM mode；`RT-16-01-02` loaded policy、profile 和 enforcement state | `E-SECURITY` |
| `RT-16-02` 进程凭据、安全标签与访问主体 | `RT-16-02-01` process uid/gid/groups/fsuid/fsgid；`RT-16-02-02` security label、loginuid 和 subject context | `E-SECURITY`、`E-PROC` |
| `RT-16-03` capability、seccomp、securebits 与 no_new_privs | `RT-16-03-01` permitted/effective/inheritable/ambient capability；`RT-16-03-02` seccomp filter、securebits 和 no_new_privs | `E-SECURITY` |
| `RT-16-04` keyring、临时密钥与内核密钥引用 | `RT-16-04-01` keyring identity、owner、permissions 和 expiry；`RT-16-04-02` key reference、link 和 payload 敏感标记 | `E-SECURITY` |
| `RT-16-05` audit 规则、状态与运行计数 | `RT-16-05-01` audit enabled/backlog/rate config；`RT-16-05-02` audit rule set、watch、exclude 和计数状态 | `E-SECURITY`、`E-TRACE` |
| `RT-16-06` 随机性、熵池与安全初始化状态 | `RT-16-06-01` CRNG initialized、entropy availability；`RT-16-06-02` random subsystem health 和 blocking risk | `E-SECURITY` |
| `RT-16-07` IMA、EVM 与完整性度量运行态 | `RT-16-07-01` IMA/EVM policy 和 measurement list 状态；`RT-16-07-02` appraisal、violation 和 integrity runtime effect | `E-SECURITY` |

### RT-17 设备、驱动、总线与 udev 运行状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-17-01` 设备枚举、设备节点与可见性 | `RT-17-01-01` device identity、major/minor 和 devnode；`RT-17-01-02` visibility、permissions 和 user-space exposure | `E-DEVICE`、`E-SYSTEMD` |
| `RT-17-02` 总线拓扑与设备层级 | `RT-17-02-01` PCI/USB/SCSI/NVMe/virtio topology；`RT-17-02-02` parent/child、slot、function 和 enumeration state | `E-DEVICE` |
| `RT-17-03` 驱动绑定、probe 与 deferred probe | `RT-17-03-01` driver binding/unbinding 状态；`RT-17-03-02` probe result、deferred probe 和 modalias 匹配 | `E-DEVICE` |
| `RT-17-04` 固件加载与驱动初始化结果 | `RT-17-04-01` firmware request/load result；`RT-17-04-02` driver init status、fallback 和 failure reason | `E-DEVICE`、`E-TRACE` |
| `RT-17-05` udev database、规则结果与设备属性 | `RT-17-05-01` udev database entry 和 rule result；`RT-17-05-02` device property、tag、symlink 和 user-visible state | `E-DEVICE`、`E-SYSTEMD` |
| `RT-17-06` 设备错误、reset 与运行时属性 | `RT-17-06-01` device error/reset/runtime PM attribute；`RT-17-06-02` driver counters、health 和 degraded state | `E-DEVICE`、`E-POWER` |

### RT-18 电源、频率、温度与硬件健康运行状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-18-01` CPU 频率、governor 与调频状态 | `RT-18-01-01` cpufreq governor、policy 和 current frequency；`RT-18-01-02` min/max、boost、throttle 和 driver state | `E-POWER`、`E-SCHED` |
| `RT-18-02` cpuidle、runtime PM 与电源管理状态 | `RT-18-02-01` cpuidle state、residency 和 disable state；`RT-18-02-02` device runtime PM、power domain 和 autosuspend | `E-POWER`、`E-DEVICE` |
| `RT-18-03` thermal zone、cooling device 与节流 | `RT-18-03-01` thermal zone temperature、trip 和 policy；`RT-18-03-02` cooling device、throttle 和 mitigation state | `E-POWER` |
| `RT-18-04` 风扇、电源供应与平台电源输入 | `RT-18-04-01` fan、power supply、battery 和 AC input；`RT-18-04-02` charge、health、capacity 和 platform power source | `E-POWER`、`E-DEVICE` |
| `RT-18-05` EDAC、MCE、RAS 与硬件错误 | `RT-18-05-01` EDAC/MCE/RAS error counters；`RT-18-05-02` corrected/uncorrected、severity 和 affected component | `E-POWER`、`E-TRACE` |
| `RT-18-06` 平台健康、节能限制与降级状态 | `RT-18-06-01` firmware/platform health indicators；`RT-18-06-02` power cap、energy policy、degraded 和 protection state | `E-POWER` |

### RT-19 时间、时钟源、定时器与计划任务状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-19-01` wall clock、monotonic 与 boottime 关系 | `RT-19-01-01` wall/monotonic/boottime 当前值关系；`RT-19-01-02` uptime、suspend time 和 offset | `E-TIME`、`E-BOOT` |
| `RT-19-02` clocksource、clockevent 与时钟精度状态 | `RT-19-02-01` current clocksource 和 available sources；`RT-19-02-02` clockevent、tick、stability 和 watchdog | `E-TIME` |
| `RT-19-03` NTP、PTP 与时间同步状态 | `RT-19-03-01` NTP/systemd-timesyncd/chrony sync state；`RT-19-03-02` PTP clock、offset、stratum 和 source quality | `E-TIME`、`E-SYSTEMD` |
| `RT-19-04` time namespace 与时间偏移视图 | `RT-19-04-01` time namespace identity 和 owner；`RT-19-04-02` monotonic/boottime offset 和 process reference | `E-TIME`、`E-IPC-NS-CG` |
| `RT-19-05` 内核 timer、hrtimer 与超时对象 | `RT-19-05-01` kernel timer/hrtimer pending set；`RT-19-05-02` expiry、callback class 和 wait timeout object | `E-TIME` |
| `RT-19-06` workqueue 与异步任务积压 | `RT-19-06-01` workqueue、worker pool 和 pending work；`RT-19-06-02` delayed work、rescuer、concurrency 和 backlog | `E-TIME`、`E-SCHED` |
| `RT-19-07` cron、systemd timer 与当前任务实例 | `RT-19-07-01` systemd timer/cron current schedule state；`RT-19-07-02` triggered/running/missed job instance | `E-TIME`、`E-SYSTEMD` |

### RT-20 易失事件缓冲、追踪缓冲与内存日志状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-20-01` kernel ring buffer 与 dmesg 现场 | `RT-20-01-01` printk ring buffer content window；`RT-20-01-02` sequence、dropped、wrap 和 severity state | `E-TRACE`、`E-BOOT` |
| `RT-20-02` volatile journald 与内存日志 | `RT-20-02-01` runtime journal storage 和 volatile entries；`RT-20-02-02` cursor、rate limit、dropped 和 flush state | `E-SYSTEMD`、`E-TRACE` |
| `RT-20-03` audit backlog 与易失审计队列 | `RT-20-03-01` audit backlog queue、lost 和 rate limit；`RT-20-03-02` pending audit event 和 writeout pressure | `E-TRACE`、`E-SECURITY` |
| `RT-20-04` trace、ftrace 与 perf buffer | `RT-20-04-01` trace/ftrace buffer content and overwrite state；`RT-20-04-02` perf buffer、event source 和 lost sample state | `E-TRACE` |
| `RT-20-05` BPF ring buffer 与动态观测输出 | `RT-20-05-01` BPF ringbuf/perf event output buffer；`RT-20-05-02` producer/consumer、dropped 和 pending sample state | `E-TRACE` |
| `RT-20-06` 驱动内部错误缓冲与设备事件队列 | `RT-20-06-01` driver error buffer 和 pending device event；`RT-20-06-02` udev/kernel event queue、dropped 和 processing lag | `E-DEVICE`、`E-SYSTEMD`、`E-TRACE` |
| `RT-20-07` 系统守护进程内存事件队列 | `RT-20-07-01` system daemon in-memory event queue；`RT-20-07-02` unflushed state buffer、retry queue 和 pending action | `E-SYSTEMD` |
| `RT-20-08` rate limit、抑制与丢失事件状态 | `RT-20-08-01` kernel/system daemon rate-limit counters；`RT-20-08-02` suppressed/dropped event gaps and current policy | `E-TRACE`、`E-SYSTEMD` |

### RT-21 OS 缓存、解析器与派生运行状态

| 二级分类 | 三级候选采集项组 | 依据组 |
| --- | --- | --- |
| `RT-21-01` DNS resolver 缓存 | `RT-21-01-01` system resolver cache entry 和 scope；`RT-21-01-02` DNSSEC、negative cache、server feature 和 stale state | `E-CACHE`、`E-SYSTEMD` |
| `RT-21-02` NSS、用户组与名称服务缓存 | `RT-21-02-01` NSS user/group/host/service cache；`RT-21-02-02` cache validity、negative entry 和 source backend | `E-CACHE` |
| `RT-21-03` page cache 与文件数据缓存占用 | `RT-21-03-01` page cache occupancy、mapped file 和 reclaimability；`RT-21-03-02` dirty/writeback/cache pressure without payload semantics | `E-CACHE`、`E-MM` |
| `RT-21-04` dentry、inode 与 slab cache | `RT-21-04-01` dentry/inode cache size and pressure；`RT-21-04-02` slab object cache、shrink state 和 reclaim stats | `E-CACHE`、`E-VFS` |
| `RT-21-05` 负查找、路径解析与派生索引缓存 | `RT-21-05-01` negative dentry 和 path lookup cache；`RT-21-05-02` derived index、lookup failure 和 cache aging state | `E-CACHE`、`E-VFS` |
| `RT-21-06` 网络路径派生缓存与协议辅助缓存 | `RT-21-06-01` route/protocol derived cache result；`RT-21-06-02` policy-derived temporary network state without neighbor ownership | `E-CACHE`、`E-NETDEV` |
| `RT-21-07` 设备属性、udev 派生属性与硬件视图缓存 | `RT-21-07-01` udev derived property cache；`RT-21-07-02` hardware view cache、tag、symlink 和 presentation state | `E-CACHE`、`E-DEVICE`、`E-SYSTEMD` |
| `RT-21-08` 系统守护进程派生缓存 | `RT-21-08-01` system daemon derived cache for OS lookup/addressing；`RT-21-08-02` cache invalidation、freshness 和 source relationship | `E-CACHE`、`E-SYSTEMD` |

## 完整性校验

| 项目 | 数量 |
| --- | --- |
| 一级大类 | 21 |
| 二级分类 | 138 |
| 三级候选采集项组 | 276 |

本文为每个二级分类定义 2 个三级候选采集项组。后续最终采集决策文档必须从这些三级项中选择、合并或推迟采集项；如果发现必须新增三级项，应先回到本文修改分类基线，再更新最终采集决策。
