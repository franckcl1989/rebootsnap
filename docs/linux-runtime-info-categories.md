# Linux 服务器操作系统运行时信息大类定义

## 文档定位

本文定义 RebootSnap 所说的“Linux 服务器操作系统运行时信息”的范围、大类和边界。

本文只回答一个问题：哪些信息属于 Linux 操作系统层面的重启前运行时现场。本文不定义采集方式、数据结构、采集优先级、报告样式或产品交互。

本文的大类之下的稳定二级分类见 `docs/linux-runtime-info-subcategories.md`。

本文采用系统视角。分类对象是 Linux 内核、初始化系统、系统守护进程和 OS 子系统维护的运行态，而不是应用、容器平台、编排平台、虚拟化管理平台或业务系统的对象模型。

## 核心定义

本文中的“操作系统运行时信息”指：

在一次 Linux 启动实例中，由内核、初始化系统、系统守护进程和 OS 子系统维护的当前状态、临时对象、运行时关系、易失缓冲、缓存、计数器或实际生效状态；这些信息在系统重启后会消失、重置、重新生成，或者失去对原现场的证明意义。

一个信息是否属于本文范围，不取决于它来自哪个命令、文件路径或工具接口，而取决于它是否满足以下语义：

- 它描述的是当前这一次 boot 中的 Linux OS 现场。
- 它的对象身份、状态值、时间关系或引用关系在重启后无法精确保留。
- 它不能仅凭持久化配置、历史日志或监控数据库完整还原。
- 它属于 OS 层运行态，而不是应用业务内部状态。

## 系统视角边界

本文只讨论 Linux 服务器操作系统本身维护、呈现、约束或派生的运行时信息。

容器、虚拟机、数据库、中间件、Web 服务、业务进程等上层对象，不作为本文一级大类。它们在 Linux OS 层面的表现应拆回对应的系统对象：

- 进程、线程、父子关系归入进程类。
- namespace、cgroup、IPC、挂载视图归入隔离与资源控制类。
- veth、tap、bridge、路由、socket、conntrack 归入网络类。
- overlay、loop、device-mapper、块设备队列归入文件系统或块设备类。
- 运行时目录、Unix socket 文件、PID 文件、锁文件归入临时文件系统与运行时目录类。

本文关心“Linux 看到的系统对象是什么状态”，不关心“上层平台把这些对象命名为什么业务实体”。

## 重启后的失真方式

运行时信息在重启后通常通过以下方式失真：

- 对象消失：进程、线程、socket、IPC 对象、锁、临时文件、timer 等生命周期结束。
- 身份重分配：PID、TID、fd、socket inode、namespace inode、cgroup 实例、设备枚举顺序等可能被重新分配。
- 状态重置：uptime、负载、队列、缓存、连接状态、内核缓冲、运行时计数器等回到新的初始状态。
- 状态重算：服务状态、路由、挂载、sysctl、策略规则等可能由配置重新计算得到，但已不是原现场。
- 缓冲丢失或覆盖：环形缓冲、volatile journal、trace buffer、驱动错误缓冲等可能清空或被新事件覆盖。
- 上下文失效：当前工作目录、打开但已删除的文件、登录会话、认证缓存、临时权限关系等失去原始上下文。
- 未持久化修改丢失：运行中手动调整但没有写回配置的状态在重启后消失。

## 排除边界

以下内容不作为本文的大类对象：

- 持久化历史追加数据，例如 `/var/log`、持久化 `journald`、审计日志归档、监控时序库。
- 持久化配置和事实来源，例如 unit 文件、sysctl 配置文件、网络配置文件、包管理数据库、内核启动配置。
- 应用业务数据，例如数据库记录、业务队列、对象存储内容、应用内部会话、应用自有缓存。
- 上层平台对象模型，例如 Pod、Container、VM、数据库实例、消息队列 topic、Web upstream 等概念本身。
- 静态资产清单，例如硬件型号、磁盘序列号、发行版版本；但这些对象的当前运行状态可以属于本文范围。
- 能从持久化来源完全精确还原、且不包含本次 boot 特有对象身份或时间关系的数据。

持久化来源产生的“当前生效结果”可以属于本文范围。例如，unit 文件不属于运行时信息，但服务当前状态、job 队列和主进程 PID 属于运行时信息；网络配置文件不属于运行时信息，但当前地址、路由、邻居表和连接状态属于运行时信息。

## 分类原则

本文按 Linux OS 子系统和运行时对象来分大类，而不是按命令、文件路径、上层产品或故障类型来分。

- 以内核对象和 OS 子系统为主边界，例如进程、内存、VFS、块设备、网络、IPC、namespace、cgroup、LSM。
- 同一底层对象允许被多个视角引用，但主归属必须明确。例如 socket 作为 fd 引用归入打开句柄，其协议状态归入 socket 与传输协议。
- 上层平台不形成一级类。容器或虚拟机在 OS 层产生的状态，应拆回进程、namespace、cgroup、挂载、网络、设备等系统对象。
- 当前状态优先于历史记录。历史日志证明过去发生过什么，运行时信息证明重启前此刻系统处于什么状态。
- 静态事实只有在解释当前运行态时作为上下文出现，不作为本文大类本体。
- 对象元数据与对象内容分开。共享内存、page cache、tmpfs 文件等对象的 OS 元数据属于本文范围，其中承载的应用业务 payload 不因此纳入本文范围。

## 判定流程

判断一个候选信息是否属于本文范围时，按以下顺序处理：

1. 是否属于 Linux OS 层，而不是应用或上层平台对象模型。
2. 是否描述当前运行态，而不是历史记录、配置来源或静态资产。
3. 重启后是否会消失、重置、重新生成，或失去对原现场的证明意义。
4. 是否无法仅凭持久化配置、历史日志或监控数据库精确还原。
5. 若同时落入多个大类，按主对象归类；其他关系只作为交叉引用。

## 大类分层

本文将 Linux OS 运行时现场分成 7 个层次、21 个大类：

| 层次 | 覆盖对象 | 大类 |
| --- | --- | --- |
| 系统实例层 | boot、内核全局状态、init 和本机控制面 | RT-01 到 RT-03 |
| 执行与资源层 | 进程、CPU、内存、fd 和内核引用 | RT-04 到 RT-07 |
| 文件与存储层 | 临时目录、VFS、文件系统、块设备和 I/O 路径 | RT-08 到 RT-10 |
| 网络数据路径层 | 网络设备、路由、socket、协议栈、包过滤和流量控制 | RT-11 到 RT-13 |
| 隔离与身份安全层 | IPC、namespace、cgroup、会话、认证、凭据和访问控制 | RT-14 到 RT-16 |
| 设备与平台层 | 设备、驱动、总线、电源、频率、温度和硬件健康 | RT-17 到 RT-18 |
| 时间与易失派生层 | 时间、定时器、事件缓冲、追踪缓冲、OS 缓存和解析器状态 | RT-19 到 RT-21 |

## 大类总览

| ID | 大类 | 稳定判定标准 |
| --- | --- | --- |
| RT-01 | 启动实例、系统身份与全局基线 | 标识当前 boot 及本次运行实例的全局 OS 上下文。 |
| RT-02 | 内核状态、运行时参数与动态内核对象 | 描述内核当前生效状态、可变参数和动态进入内核的对象。 |
| RT-03 | 初始化系统、服务管理与本机控制面状态 | 描述 init/service manager、系统守护进程和本机 OS 控制面的运行态。 |
| RT-04 | 进程、线程与执行上下文 | 描述当前执行主体、进程层级、线程状态和进程级运行属性。 |
| RT-05 | CPU、调度、中断与负载状态 | 描述 CPU 在线状态、调度队列、中断软中断、负载和执行压力。 |
| RT-06 | 内存、虚拟内存、swap 与内存压力状态 | 描述物理内存、虚拟内存、swap、页回收、OOM、NUMA 和内存压力现场。 |
| RT-07 | 打开句柄、文件引用与内核同步对象 | 描述 fd、打开文件、匿名 inode、pipe、epoll、锁和进程持有的内核引用。 |
| RT-08 | 临时文件系统与运行时目录状态 | 描述 tmpfs、`/run`、PID 文件、锁文件、Unix socket 文件等易失目录现场。 |
| RT-09 | VFS、文件系统与挂载状态 | 描述 VFS 层、挂载拓扑、superblock、文件系统错误和挂载命名空间视图。 |
| RT-10 | 块设备、存储映射与 I/O 路径状态 | 描述 block layer、device-mapper、LVM、mdraid、multipath、I/O 队列和存储路径现场。 |
| RT-11 | 网络接口、链路、地址与路由状态 | 描述网络设备、链路、地址、邻居、路由、网络命名空间内视图和接口级统计。 |
| RT-12 | socket、传输协议与连接状态 | 描述监听 socket、连接、协议状态、socket 队列和协议栈计数器。 |
| RT-13 | 包过滤、NAT、连接跟踪与流量控制状态 | 描述当前生效的数据面规则、conntrack、NAT、qdisc、tc、XDP 和 XFRM 状态。 |
| RT-14 | IPC、namespace 与 cgroup 状态 | 描述进程间通信对象、隔离边界、资源控制层级、成员关系和运行统计。 |
| RT-15 | 用户登录、会话与临时认证状态 | 描述当前登录用户、TTY/PTY、SSH、logind、PAM/sudo/Kerberos 等临时认证上下文。 |
| RT-16 | 安全策略、凭据与访问控制运行态 | 描述 LSM、进程凭据、capability、seccomp、keyring、audit 和随机性安全状态。 |
| RT-17 | 设备、驱动、总线与 udev 运行状态 | 描述设备枚举、驱动绑定、热插拔、固件加载、设备错误和 udev 运行现场。 |
| RT-18 | 电源、频率、温度与硬件健康运行状态 | 描述 CPU 频率、电源状态、thermal、RAS、硬件错误和平台健康当前状态。 |
| RT-19 | 时间、时钟源、定时器与计划任务状态 | 描述 wall clock、monotonic、clocksource、NTP/PTP、内核 timer 和当前任务实例。 |
| RT-20 | 易失事件缓冲、追踪缓冲与内存日志状态 | 描述 dmesg、volatile journal、trace/perf/BPF buffer、audit backlog 等未持久化事件现场。 |
| RT-21 | OS 缓存、解析器与派生运行状态 | 描述 DNS/NSS 缓存、page/dentry/inode/slab cache 和其他 OS 派生缓存状态。 |

## 大类定义

### RT-01 启动实例、系统身份与全局基线

- 判定标准：信息用于标识当前这一次系统启动实例，或者定义本次运行实例的全局 OS 上下文。
- 包括：boot ID、uptime、当前运行内核、启动参数的生效结果、当前 hostname/domainname、本次 boot 的基础时间关系和内核全局基线。
- 不包括：发行版版本文件、machine-id、bootloader 配置、内核包文件、静态资产清单和其他持久化事实来源。
- 重启失真：boot ID、uptime 和本次运行实例的时间关系会重新建立；即使部分静态标识相同，也不能证明重启前的全局现场。

### RT-02 内核状态、运行时参数与动态内核对象

- 判定标准：信息属于当前内核实例的全局生效状态、可变参数，或运行中动态创建、加载、附着到内核的对象。
- 包括：运行时 sysctl 值、sysfs 中的内核运行时开关、内核 taint 状态、已加载模块及其运行时参数、livepatch 状态、kexec/crashkernel 当前状态、watchdog/lockup detector 状态、eBPF program/map/link、kprobe、uprobe、ftrace、perf event、dynamic debug 和其他动态内核设施。
- 不包括：sysctl 配置文件、模块加载配置、模块文件、BPF 对象文件、调试脚本和持久化调试配置。
- 边界：eBPF program/map/link 等内核对象本身归入本类；如果它们附着在网络数据面并形成包处理效果，其附着点、命中计数和队列影响归入 RT-13。设备私有 sysfs 属性、驱动绑定和电源状态分别归入 RT-17 或 RT-18。
- 重启失真：运行时参数可能由配置重算，模块和动态内核对象会被卸载或重新创建；map 内容、探针附着、临时开关和内核 taint 现场无法自然保留。

### RT-03 初始化系统、服务管理与本机控制面状态

- 判定标准：信息描述初始化系统、服务管理器、系统守护进程或本机 OS 控制面的当前状态。
- 包括：systemd unit 当前 active/sub 状态、job 队列、target 状态、socket/path/scope/slice 状态、timer unit 在服务管理器中的状态、transient unit、服务主进程关联、服务重启计数、失败状态、依赖阻塞、shutdown/reboot job、inhibitor lock、system bus name ownership、系统守护进程的运行时连接和本机控制面状态。
- 不包括：unit 文件、服务配置文件、包安装记录、服务历史日志和应用内部健康状态。
- 边界：timer unit 作为服务管理器对象的状态归入本类；内核 timer、hrtimer、monotonic 时间关系和任务实际等待/触发现场归入 RT-19。
- 重启失真：服务管理器会重新计算 unit 状态并重新建立 job、bus name 和进程关系；重启前的阻塞、失败、重启计数和本机控制面连接会失去原现场意义。

### RT-04 进程、线程与执行上下文

- 判定标准：信息描述 Linux 当前正在执行、等待执行或尚未回收的进程、线程和执行上下文。
- 包括：进程树、PID/TID、父子关系、进程状态、线程状态、命令行、环境、当前工作目录、root 目录、优先级、nice、调度策略、CPU affinity、进程资源限制、僵尸进程、进程退出等待状态和进程级运行属性。
- 不包括：可执行文件内容、应用配置、应用协议状态、业务任务状态和进程内部业务数据结构。
- 边界：进程持有的 fd 和锁归入 RT-07；进程内存映射和内存占用归入 RT-06；进程安全标签和凭据归入 RT-16。
- 重启失真：进程和线程全部结束，PID/TID 与执行上下文重新分配；同一个程序再次启动也是新的 OS 运行实例。

### RT-05 CPU、调度、中断与负载状态

- 判定标准：信息描述 CPU 执行资源、调度器、中断路径和执行压力的当前状态。
- 包括：CPU online/offline 状态、run queue、load average、调度统计、上下文切换、抢占与迁移统计、irq/softirq 计数、中断亲和性、软中断 backlog、CPU steal/iowait 表现、CPU PSI、调度域和 CPU 侧执行压力。
- 不包括：CPU 型号、核心数量等静态硬件资产事实，除非它们的当前在线或运行状态发生变化。
- 边界：CPU 频率、电源状态和硬件节流归入 RT-18；cgroup CPU 限制和成员关系归入 RT-14。
- 重启失真：调度队列、中断分布、运行负载和 CPU 侧即时压力会重置；累计计数即使再次出现，也不能还原重启前的队列和执行现场。

### RT-06 内存、虚拟内存、swap 与内存压力状态

- 判定标准：信息描述内存管理子系统的当前占用、映射、回收、交换和压力状态。
- 包括：物理内存使用、free/available、虚拟内存统计、进程级内存映射摘要、swap active set、swap 使用、zswap/zram 使用、页回收、dirty/writeback、OOM killer 状态、OOM 相关计数、PSI memory、NUMA 分布、hugepage、THP、KSM、内存压缩/规整、内存碎片、vmstat、zone/node 状态和页表相关运行态。
- 不包括：应用堆内业务对象、数据库 buffer pool 的业务语义、监控系统历史曲线和容量规划文档。
- 边界：page cache、dentry cache、inode cache、slab cache 作为缓存对象归入 RT-21；内存硬件错误和 RAS 状态归入 RT-18；cgroup memory 限制与成员关系归入 RT-14。
- 重启失真：内存占用、页表、回收状态、OOM 风险、NUMA 分布和内存压力全部重新建立；历史监控不能精确恢复重启前的内存对象关系。

### RT-07 打开句柄、文件引用与内核同步对象

- 判定标准：信息描述进程持有的 fd、打开对象引用、匿名内核对象、同步对象或通知对象。
- 包括：fd 表、打开文件、文件偏移、打开标志、已删除但仍被打开的文件、pipe、eventfd、timerfd、signalfd、memfd、pidfd、userfaultfd、io_uring、epoll、inotify/fanotify、flock、POSIX lock、lease、futex 和进程持有的 socket fd 引用。
- 不包括：普通文件的持久化内容、文件系统权限配置、应用层锁协议和网络协议层连接语义。
- 边界：socket 作为 fd 引用归入本类，socket 协议状态归入 RT-12；文件系统本身的挂载和 superblock 状态归入 RT-09。
- 重启失真：fd、锁、匿名 inode 和同步对象都会释放；已删除但仍被打开的文件可能彻底消失，原始引用关系无法恢复。

### RT-08 临时文件系统与运行时目录状态

- 判定标准：信息存在于 tmpfs、运行时目录或被系统策略定义为重启清理的目录中，并承担 OS 协调或运行时引用作用。
- 包括：`/run`、`/run/lock`、`/dev/shm`、tmpfs、PID 文件、锁文件、Unix socket 文件、运行时状态文件、守护进程运行目录、临时挂载点和被系统策略定义为重启清理的目录项。
- 不包括：持久化目录中的普通应用数据、长期保留的 `/var/tmp` 内容、配置文件和可从应用存储恢复的缓存。
- 边界：临时文件系统中的目录项、引用关系、协调文件和 OS 对象属于本类；如果临时文件承载的是应用业务 payload，业务内容本身不因此纳入本文范围。
- 重启失真：这类目录通常被清空或重新创建；即使同名文件再次出现，也不是重启前的同一运行时对象。

### RT-09 VFS、文件系统与挂载状态

- 判定标准：信息描述 VFS 和文件系统层当前如何组织、暴露、保护和维护路径空间。
- 包括：挂载表、挂载选项、挂载传播关系、mount namespace 中的挂载视图、bind mount、FUSE mount、overlay mount 的 OS 层挂载关系、superblock 运行态、文件系统 journal 运行态、dirty inode、quota 当前状态、文件系统只读/错误状态、inode/dentry 相关 VFS 状态和文件系统运行时错误计数。
- 不包括：文件持久化内容、文件系统配置模板、备份元数据和存储系统中的历史告警。
- 边界：块设备队列、device-mapper 和 I/O 路径归入 RT-10；page/dentry/inode cache 作为缓存对象归入 RT-21。
- 重启失真：挂载拓扑、namespace 内挂载视图、superblock 运行态和临时挂载关系会重新建立；只读切换、journal 现场和错误现场可能消失或改变。

### RT-10 块设备、存储映射与 I/O 路径状态

- 判定标准：信息描述块设备层、存储映射层或 I/O 路径的当前状态。
- 包括：block device 状态、块设备队列、I/O scheduler、request queue、device-mapper、dm-crypt、LVM、mdraid、multipath、loop 设备、zram block device、NVMe/SCSI 运行态、设备繁忙状态、I/O 错误、路径切换、队列拥塞、I/O PSI 和 block layer 计数器。
- 不包括：块设备上保存的文件内容、分区表静态事实、LVM 配置备份、存储阵列持久化配置和外部存储系统历史告警。
- 边界：文件系统挂载和 superblock 归入 RT-09；设备枚举和驱动绑定归入 RT-17。
- 重启失真：块设备队列、临时映射、I/O 拥塞、路径选择和错误现场会重新建立或丢失；重建后的路径状态不能代表重启前的 I/O 现场。

### RT-11 网络接口、链路、地址与路由状态

- 判定标准：信息描述 Linux 网络设备、链路层、三层寻址路径或网络命名空间内网络视图的当前状态。
- 包括：物理/虚拟网卡 link 状态、MAC 地址、IP 地址、临时地址、MTU、队列状态、接口统计、bridge、bond、vlan、macvlan、ipvlan、vxlan、geneve、gre、veth、tap、tun、wireguard 等内核网络设备状态、路由表、策略路由当前结果、邻居表、ARP/NDP、网络命名空间内接口视图、接口级错误和丢包计数。
- 不包括：网络配置文件、外部交换机配置、DNS 区域数据、云网络控制面期望状态和持久化网络变更记录。
- 边界：socket 与传输协议状态归入 RT-12；netfilter、NAT、conntrack、tc、XDP 和 XFRM/IPsec 数据面状态归入 RT-13；network namespace 对象身份归入 RT-14。
- 重启失真：接口、地址、邻居缓存和路由由配置或外部控制面重新建立；新的链路与路由状态不能证明重启前的网络现场。

### RT-12 socket、传输协议与连接状态

- 判定标准：信息描述 Linux socket 层和传输协议层的当前连接、队列或协议状态。
- 包括：监听 socket、已建立连接、TCP 状态机、UDP socket、raw socket、netlink socket、Unix domain socket 协议状态、socket buffer、send/receive queue、listen/SYN backlog、TIME_WAIT、orphan socket、重传、拥塞控制状态、协议栈计数器和 socket inode 与进程的关联。
- 不包括：应用协议会话、HTTP 请求、数据库连接池语义、RPC 业务状态和应用层连接管理数据。
- 边界：socket fd 引用归入 RT-07；网络接口、地址、邻居和路由归入 RT-11；conntrack/NAT 归入 RT-13。
- 重启失真：socket 和连接全部断开，队列与协议状态清空；同一端口再次监听也不是重启前的 socket 对象。

### RT-13 包过滤、NAT、连接跟踪与流量控制状态

- 判定标准：信息描述当前实际参与数据包处理的数据面控制、转发、过滤、追踪、加密策略或队列状态。
- 包括：nftables/iptables/ip6tables/ebtables 当前规则集和计数器、nft set、nft flowtable、NAT 映射、conntrack 表、IPVS 状态、tc qdisc/class/filter、流量整形队列、XDP/tc 附着点、XFRM/IPsec state/policy、包标记、策略命中计数和数据面队列状态。
- 不包括：防火墙配置文件、规则模板、编排系统期望状态、云安全组远端配置和持久化策略审计记录。
- 边界：BPF 程序对象本身归入 RT-02；网络设备和路由归入 RT-11；socket 协议状态归入 RT-12。
- 重启失真：conntrack、NAT 映射、命中计数和队列状态会丢失；规则即使重新加载，也无法恢复重启前的流量现场。

### RT-14 IPC、namespace 与 cgroup 状态

- 判定标准：信息描述内核级进程间通信对象、隔离边界、资源控制对象、成员关系或资源控制事件。
- 包括：SysV IPC、POSIX message queue、共享内存、信号量、PID/network/mount/UTS/IPC/user/cgroup/time namespace、user namespace id map、cgroup v1/v2 层级、controller 生效状态、资源限制、进程成员关系、cgroup 运行统计、pressure 和事件计数。
- 不包括：容器、Pod、VM 等上层对象概念，不包括编排平台期望状态，也不包括应用协议内部通信状态。
- 边界：IPC 对象的身份、大小、权限、引用和等待关系属于本类；共享内存或消息队列中的应用业务 payload 不因此纳入本文范围。系统整体 CPU/内存/I/O 压力分别归入对应资源类。
- 重启失真：IPC 对象、namespace 实例和 cgroup 运行对象会销毁或重建；同名 slice、scope 或目录不代表同一个运行时对象。

### RT-15 用户登录、会话与临时认证状态

- 判定标准：信息描述当前用户进入系统、保持会话、占用终端或临时获得认证/授权上下文的 OS 状态。
- 包括：当前登录用户、TTY/PTY、控制终端、SSH 会话、systemd-logind session、seat、loginuid、utmp 当前会话、PAM session、用户级 systemd manager/session bus 关系、sudo credential cache、Kerberos credential cache 和用户级临时认证材料。
- 不包括：`/etc/passwd`、`/etc/shadow`、sudoers 配置、SSH 配置、账号目录中的长期数据和持久化认证日志。
- 边界：进程的用户/组凭据和 capability 归入 RT-16；进程本身和控制终端引用归入 RT-04/RT-07。
- 重启失真：当前会话结束，临时认证材料通常失效或被清理；持久化认证日志只能证明历史行为，不能保留会话现场。

### RT-16 安全策略、凭据与访问控制运行态

- 判定标准：信息描述当前实际约束进程、文件、内核对象或系统访问路径的安全运行态。
- 包括：SELinux/AppArmor/Landlock 当前模式和加载态、LSM 当前约束、进程安全标签、seccomp filter、capability 集合、securebits、no_new_privs、ambient capability、dumpable 状态、用户/组凭据、内核 keyring、临时密钥、kernel random/entropy 当前状态、audit 当前规则、IMA/EVM 运行状态和访问控制相关运行计数。
- 不包括：安全策略源文件、用户账号配置、证书长期存储、审计日志归档和离线合规报告。
- 边界：audit backlog 和易失事件输出归入 RT-20；登录会话和认证缓存归入 RT-15。
- 重启失真：进程安全上下文、临时凭据、keyring、seccomp 附着、随机性状态和运行时安全状态会消失或重新加载；原进程受到的实际约束无法仅凭配置精确还原。

### RT-17 设备、驱动、总线与 udev 运行状态

- 判定标准：信息描述 OS 已发现设备、总线枚举、驱动绑定、设备节点或热插拔路径的当前状态。
- 包括：udev 运行态、udev database 当前内容、设备节点当前状态、PCI/USB/SCSI/NVMe/virtio 等总线枚举状态、驱动绑定、固件加载结果、热插拔事件现场、设备错误计数、设备 reset 状态、driver probe/deferred probe 状态和驱动暴露的运行时属性。
- 不包括：静态硬件资产清单、设备采购信息、固件文件本身和硬件管理平台中的持久化历史记录。
- 边界：块设备 I/O 路径归入 RT-10；电源、频率、温度和硬件健康归入 RT-18。
- 重启失真：设备重新枚举，驱动重新绑定，热插拔现场和部分错误状态会变化或消失；同一设备再次出现也不代表相同运行路径。

### RT-18 电源、频率、温度与硬件健康运行状态

- 判定标准：信息描述硬件平台在 OS 管理下的当前电源、频率、温度、节流或健康状态。
- 包括：CPU frequency governor 当前状态、cpuidle 状态、电源管理状态、runtime PM、thermal zone、风扇/温度告警、电池或电源供应状态、EDAC/MCE/RAS 当前错误、硬件节流、NUMA/内存硬件错误和平台健康相关运行计数。
- 不包括：硬件规格、采购资产、固件版本清单和 BMC/IPMI/Redfish 中的持久化历史事件；但这些外部平台反映到 OS 的当前运行状态可以作为本类上下文。
- 边界：设备枚举和驱动绑定归入 RT-17；CPU 调度压力归入 RT-05。
- 重启失真：电源状态、频率策略、温度、节流和硬件错误现场可能重新初始化或被清零；新的健康状态不能完整代表重启前的平台现场。

### RT-19 时间、时钟源、定时器与计划任务状态

- 判定标准：信息描述系统当前时间体系、时钟同步、定时机制、延迟执行对象或当前计划任务实例。
- 包括：wall clock、monotonic/boottime 关系、clocksource、clockevent、time namespace、NTP/PTP 同步状态、时钟偏移、内核 timer、hrtimer、延迟任务、workqueue、cron 当前运行实例、systemd timer 的下一次触发时间和当前触发出的任务实例。
- 不包括：cron 表、systemd timer unit 文件、NTP 配置文件、时区配置、硬件 RTC 静态配置、任务历史日志和业务调度系统中的持久化任务定义。
- 边界：timer unit 作为服务管理器对象的生命周期归入 RT-03；timerfd 作为进程 fd 对象归入 RT-07。
- 重启失真：monotonic 时间重新开始，内核定时器和延迟任务清空，当前任务实例结束；同步状态和时间偏移需要重新建立。

### RT-20 易失事件缓冲、追踪缓冲与内存日志状态

- 判定标准：信息存在于易失事件缓冲、内存日志、追踪缓冲或尚未持久化的系统事件队列中。
- 包括：dmesg/kernel ring buffer、volatile journald、audit backlog、trace buffer、perf buffer、BPF ring buffer、驱动内部错误缓冲、udev 事件队列、内核 rate-limit 中的未输出状态和有明确 OS 子系统语义的系统守护进程内存事件队列。
- 不包括：持久化 `/var/log` 文件、持久化 journald、pstore 中已经具备跨重启保存语义的记录、集中日志平台、审计日志归档和监控事件库。
- 边界：audit 规则本身归入 RT-16；设备错误状态本身归入 RT-17/RT-18，错误事件缓冲归入本类。
- 重启失真：易失缓冲会清空或被新事件覆盖；即使部分事件曾经落盘，内存缓冲中的顺序、上下文、未落盘片段和被限流片段也可能无法恢复。

### RT-21 OS 缓存、解析器与派生运行状态

- 判定标准：信息由 OS 子系统或系统守护进程为了加速解析、寻址、访问或内存管理而临时维护，可重建但不能代表重启前现场。
- 包括：DNS resolver cache、NSS/nscd/systemd-resolved 缓存、page cache、dentry cache、inode cache、slab cache、目录项负缓存、路由派生缓存、设备属性缓存、udev 派生属性缓存和其他有明确 OS 子系统语义的派生临时缓存状态。
- 不包括：持久化包缓存、应用自有业务缓存、浏览器缓存、数据库 buffer pool 和可以作为事实来源的长期索引文件。
- 边界：缓存对象、命中/缺失语义、占用关系和派生结果属于本类；缓存中可能包含的文件内容或应用业务内容不因此纳入本文范围。
- 重启失真：缓存被清空并随新访问重新生成；重新生成的缓存只能反映重启后的访问路径，不能代表重启前命中、缺失、污染或内存占用现场。

## 相邻边界规则

| 边界问题 | 主归属 |
| --- | --- |
| socket fd 与 TCP 状态 | fd 引用归 RT-07；TCP/UDP/Unix socket 协议状态归 RT-12。 |
| 网络命名空间与其中的网络状态 | namespace 对象身份归 RT-14；接口、路由、邻居归 RT-11；socket 归 RT-12。 |
| systemd timer 与内核 timer | timer unit 生命周期归 RT-03；内核 timer、hrtimer 和触发现场归 RT-19。 |
| cgroup 与资源压力 | cgroup 层级、controller、成员和限制归 RT-14；CPU/内存/I/O 压力归 RT-05、RT-06、RT-10。 |
| 文件系统与块设备 | 挂载、superblock、VFS 状态归 RT-09；block queue、device-mapper、I/O 路径归 RT-10。 |
| 设备状态与硬件健康 | 枚举、驱动、设备节点归 RT-17；频率、温度、节流、RAS/EDAC/MCE 归 RT-18。 |
| 容器、Pod、VM 等上层对象 | 上层对象概念不入大类；其 OS 表现拆回进程、namespace、cgroup、挂载、网络、设备等大类。 |
| 事件事实与事件缓冲 | 已持久化历史日志不入本文；易失 ring buffer、volatile journal、trace buffer 归 RT-20。 |

## 覆盖性校验

这套大类从 Linux 运行态的主要来源反推，覆盖以下系统面：

| 运行态来源/系统面 | 覆盖大类 |
| --- | --- |
| boot 与内核全局上下文 | RT-01、RT-02 |
| init/service manager 与系统守护进程 | RT-03 |
| `/proc` 中的进程、线程、fd、调度、内存、socket、IPC、sysctl 运行态 | RT-02、RT-04、RT-05、RT-06、RT-07、RT-12、RT-14 |
| `/sys` 中的设备、驱动、总线、电源、thermal、block、net 运行态 | RT-10、RT-11、RT-17、RT-18 |
| VFS、mount namespace、文件系统、tmpfs 和运行时目录 | RT-08、RT-09 |
| block layer、device-mapper、mdraid、multipath 和 I/O 队列 | RT-10 |
| netlink 可见的网络设备、地址、路由、邻居、socket 和数据面状态 | RT-11、RT-12、RT-13 |
| namespace、cgroup、IPC 和资源控制对象 | RT-14 |
| 用户会话、认证临时状态、凭据、LSM、audit 和 keyring | RT-15、RT-16 |
| 时间、定时器、计划任务和同步状态 | RT-19 |
| 内核/系统易失事件缓冲和追踪缓冲 | RT-20 |
| OS 缓存、解析器、slab/page/dentry/inode 派生状态 | RT-21 |

若一个候选信息无法落入上述任一系统面，通常说明它不是 Linux OS 运行时信息，而是持久化事实、历史记录、应用业务状态或上层平台对象。

## 边界判定示例

| 信息 | 是否属于本文范围 | 归属 |
| --- | --- | --- |
| `/etc/sysctl.conf` | 否 | 持久化配置，不是运行时现场。 |
| 当前 `net.ipv4.ip_forward` 生效值 | 是 | RT-02，运行时内核参数。 |
| systemd unit 文件 | 否 | 持久化服务定义。 |
| 某服务当前 active 状态、job 和主进程 PID | 是 | RT-03，服务管理运行态。 |
| 某容器或 Pod 的平台状态对象 | 否 | 上层平台对象模型，不作为 OS 大类。 |
| 容器相关的 namespace、cgroup、veth、overlay mount | 是 | 分别归入 RT-14、RT-11、RT-09 等 OS 类。 |
| 虚拟机管理平台中的 VM 对象 | 否 | 上层虚拟化管理对象，不作为 OS 大类。 |
| VM 进程、tap 设备、vhost 设备和相关 socket | 是 | 分别归入 RT-04、RT-17、RT-11、RT-12 等 OS 类。 |
| `/var/log/messages` 中的历史记录 | 否 | 持久化历史日志。 |
| 当前 dmesg ring buffer | 是 | RT-20，易失事件缓冲。 |
| pstore 中已跨重启保存的 panic 记录 | 否 | 已具备跨重启保存语义，不是重启前易失现场本体。 |
| 普通文件内容 | 否 | 持久化数据。 |
| 已删除但仍被进程打开的文件 | 是 | RT-07，打开引用关系。 |
| 网络配置文件 | 否 | 持久化配置。 |
| 当前地址、路由、邻居表 | 是 | RT-11，网络接口与路由状态。 |
| 当前 TCP 连接和 socket 队列 | 是 | RT-12，socket 与传输协议状态。 |
| 防火墙配置模板 | 否 | 持久化期望状态。 |
| 当前 conntrack 表和 NAT 映射 | 是 | RT-13，数据面运行状态。 |

## 完整性说明

本文的大类覆盖 Linux 服务器 OS 层面在重启前具有现场意义的运行时信息。对于一个候选信息，先判断它是否属于 Linux OS 层，再判断它是否会在重启后消失、重置、重新生成或失去原现场意义；满足这两个条件后，再按其主对象归入上述大类。

本文不声称 Linux 上所有可读取的数据都值得保存；它定义的是“重启前 OS 现场”的完整分类边界。后续文档可以在这些稳定大类下继续拆二级对象、字段、采集入口、风险和报告视图，但不应绕过本文重新创建上层业务化大类。
