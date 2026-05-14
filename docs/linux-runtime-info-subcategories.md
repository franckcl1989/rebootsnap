# Linux OS 运行时信息二级分类

## 文档定位

本文在 `docs/linux-runtime-info-categories.md` 的 21 个 Linux OS 运行时信息大类之下，定义可稳定引用的二级分类。

本文只回答一个问题：每个大类内部可以进一步拆成哪些 OS 运行时信息类别。本文不定义采集命令、采集接口、字段结构、采集优先级、存储格式、报告样式或产品交互。

本文中的二级分类是后续采集项、数据模型和报告视图的设计锚点。后续实现可以把多个采集项挂到同一个二级分类下，但不应绕过本文直接创建与 `RT-01` 到 `RT-21` 平行的新上层分类。

## 继承边界

本文继承 `docs/linux-runtime-info-categories.md` 中的全部边界：

- 分类对象是 Linux 内核、初始化系统、系统守护进程和 OS 子系统维护的运行态。
- 容器、Pod、VM、数据库实例、Web 服务和业务系统不作为二级分类对象；它们在 Linux 层面的表现应拆回进程、namespace、cgroup、挂载、网络、设备等 OS 对象。
- 持久化配置、历史日志、静态资产清单和应用业务数据不因被采集或被引用而成为 Linux OS 运行时信息。
- 同一底层对象允许被多个分类引用，但主归属必须按对象本体确定；交叉关系只作为上下文。

如果本文和大类定义发生冲突，以 `docs/linux-runtime-info-categories.md` 的大类边界为准。

## 审核依据

本文的二级分类按以下官方或一手资料校准边界，但不把这些资料中的接口路径、命令或字段直接提升为分类规则：

- Linux kernel documentation 中的 [filesystems](https://www.kernel.org/doc/html/latest/filesystems/index.html)、[networking](https://www.kernel.org/doc/html/latest/networking/index.html)、[cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)、[PSI](https://www.kernel.org/doc/html/latest/accounting/psi.html)、[BPF maps](https://www.kernel.org/doc/html/latest/bpf/maps.html)、[ftrace](https://www.kernel.org/doc/html/v4.19/trace/ftrace.html)、[zswap](https://www.kernel.org/doc/html/latest/admin-guide/mm/zswap.html)、[CPUFreq](https://www.kernel.org/doc/html/latest/admin-guide/pm/cpufreq.html) 和 [driver model](https://www.kernel.org/doc/html/latest/driver-api/infrastructure.html) 文档。
- Linux man-pages 中的 [proc(5)](https://man7.org/linux/man-pages/man5/proc.5.html)、[namespaces(7)](https://man7.org/linux/man-pages/man7/namespaces.7.html)、[cgroups(7)](https://man7.org/linux/man-pages/man7/cgroups.7.html)、[capabilities(7)](https://man7.org/linux/man-pages/man7/capabilities.7.html)、[inotify(7)](https://man7.org/linux/man-pages/man7/inotify.7.html) 和 [fanotify(7)](https://man7.org/linux/man-pages/man7/fanotify.7.html)。
- systemd 官方文档中的 unit、service、socket、path、timer、logind 和 inhibitor 相关手册。

## 二级分类原则

- 二级分类按 OS 对象、运行时关系和子系统状态拆分，不按命令、文件路径、工具名称、故障类型或上层产品概念拆分。
- 二级分类只描述当前运行态，不承诺该信息一定可在所有 Linux 发行版、内核版本或权限条件下采集。
- 二级分类名称应表达信息对象和运行态语义，避免使用“其他信息”“相关状态”等无法约束后续工作的名称。
- 每个二级分类必须归属于且只归属于一个 `RT-xx` 大类。若信息跨多个大类，应按主对象归类，并在后续采集或报告中建立交叉引用。
- 二级分类不等同于采集项。采集项可以更细，也可以为了效率一次覆盖多个二级分类。

## ID 规则

二级分类 ID 使用 `RT-xx-yy`：

- `RT-xx` 必须是现有大类 ID。
- `yy` 从 `01` 开始按大类内部顺序递增。
- ID 一旦被采集项、数据模型、报告或变更记录引用，即视为稳定 ID。删除、重命名或重编号稳定 ID 前必须由人类维护者确认。
- 后续新增二级分类时，追加新的 `yy` 编号，不复用已经废弃或保留的编号。

## 二级分类定义

### RT-01 启动实例、系统身份与全局基线

#### RT-01-01 启动实例身份

- 判定标准：信息用于区分当前这一次 boot 实例本身，包括本次启动的唯一性、生命周期和重启前后不可继承的身份。

#### RT-01-02 全局运行时间关系

- 判定标准：信息描述当前 boot 内的 uptime、启动时间、运行时长和与其他时间基准之间的全局关系。

#### RT-01-03 运行内核与启动参数生效基线

- 判定标准：信息描述当前正在运行的内核实例、实际生效的启动参数和由启动参数形成的全局 OS 基线。

#### RT-01-04 主机命名与系统域运行态

- 判定标准：信息描述当前 hostname、domainname、machine role 等 OS 层命名和系统身份的运行时生效结果。

### RT-02 内核状态、运行时参数与动态内核对象

#### RT-02-01 内核运行时参数

- 判定标准：信息描述当前内核实例中可以在运行中读取或修改的全局参数、开关和限制。

#### RT-02-02 已加载模块与模块运行参数

- 判定标准：信息描述当前已进入内核地址空间的模块、模块引用关系和模块级运行参数。

#### RT-02-03 内核完整性、taint 与热修补状态

- 判定标准：信息描述当前内核实例的 taint、livepatch、kexec、crashkernel、watchdog 和 lockup detector 等全局健康或完整性状态。

#### RT-02-04 动态探针、追踪与性能事件对象

- 判定标准：信息描述运行中附着到内核或用户态边界的 kprobe、uprobe、ftrace、perf event、dynamic debug 等动态观测对象。

#### RT-02-05 eBPF 程序、映射与链接对象

- 判定标准：信息描述当前内核中的 eBPF program、map、link 和引用关系，不包含其在具体数据面上的包处理效果。

### RT-03 初始化系统、服务管理与本机控制面状态

#### RT-03-01 unit 生命周期与依赖状态

- 判定标准：信息描述 service、target、mount、device、scope、slice 等 unit 的 active/sub 状态和依赖关系的当前结果。

#### RT-03-02 job、事务与启动关闭队列

- 判定标准：信息描述服务管理器当前排队、执行、阻塞或失败的 job、transaction、shutdown 和 reboot 相关工作。

#### RT-03-03 transient unit、scope 与 slice 运行态

- 判定标准：信息描述运行中动态创建的 transient unit、scope、slice 及其与进程和资源控制对象的关系。

#### RT-03-04 服务主进程、重启计数与失败状态

- 判定标准：信息描述服务管理器视角下的主进程、控制进程、退出结果、重启计数、失败原因和 watchdog 状态。

#### RT-03-05 socket、path 与 timer unit 运行态

- 判定标准：信息描述由服务管理器维护的 socket activation、path activation 和 timer unit 的当前等待、触发和关联状态。

#### RT-03-06 inhibitor、会话控制与本机管理锁

- 判定标准：信息描述当前阻止或延迟关机、重启、睡眠、会话切换等本机控制动作的运行时锁和持有者。

#### RT-03-07 system bus 名称所有权与控制面连接

- 判定标准：信息描述系统总线、本机 OS 控制面和系统守护进程之间的当前连接、名称所有权和服务暴露状态。

### RT-04 进程、线程与执行上下文

#### RT-04-01 进程身份与生命周期状态

- 判定标准：信息描述当前进程的 PID、状态、启动关系、退出等待、僵尸状态和生命周期阶段。

#### RT-04-02 父子关系、进程组、会话与控制终端引用

- 判定标准：信息描述进程树、父子关系、进程组、session、控制终端引用和作业控制相关运行态。

#### RT-04-03 可执行镜像、启动参数与执行目录视图

- 判定标准：信息描述进程当前可执行入口、命令行、环境、当前工作目录、root 目录和执行视图。

#### RT-04-04 线程集合与线程执行状态

- 判定标准：信息描述一个进程内的线程、TID、线程状态、阻塞点、等待原因和线程级执行属性。

#### RT-04-05 进程级调度属性与 CPU 亲和性

- 判定标准：信息描述进程或线程自身携带的调度策略、优先级、nice、实时属性和 CPU affinity。

#### RT-04-06 资源限制、运行计数与退出上下文

- 判定标准：信息描述进程级 rlimit、资源使用计数、信号状态、退出码等待关系和可由父进程回收的上下文。

#### RT-04-07 隔离、资源控制与路径视图引用

- 判定标准：信息描述进程当前引用的 namespace、cgroup、挂载视图、root 视图和其他隔离边界引用关系。

### RT-05 CPU、调度、中断与负载状态

#### RT-05-01 CPU 在线状态与执行容量

- 判定标准：信息描述 CPU 当前 online/offline、可调度容量、isolated/nohz 等影响执行资源可用性的运行态。

#### RT-05-02 调度队列、负载与运行压力

- 判定标准：信息描述 run queue、load average、可运行任务积压和调度器当前执行压力。

#### RT-05-03 调度统计、迁移与上下文切换

- 判定标准：信息描述任务调度、抢占、迁移、上下文切换和调度延迟相关的当前计数或状态。

#### RT-05-04 中断、软中断与 backlog

- 判定标准：信息描述 IRQ、softirq、tasklet、NAPI 等 CPU 侧异步执行路径的当前分布、亲和性和积压。

#### RT-05-05 CPU wait、steal、iowait 与 PSI

- 判定标准：信息描述 CPU 侧等待、被抢占、I/O 等待和 pressure stall information 所反映的即时压力。

#### RT-05-06 调度域与 CPU 侧均衡状态

- 判定标准：信息描述调度域、负载均衡、CPU mask 和跨 CPU 调度组织关系的当前生效状态。

### RT-06 内存、虚拟内存、swap 与内存压力状态

#### RT-06-01 物理内存占用与可用性

- 判定标准：信息描述物理内存总量中当前 free、available、used、reserved、dirty 页和 writeback 页等占用与可用状态。

#### RT-06-02 zone、node、NUMA 与页分配状态

- 判定标准：信息描述内存 zone、NUMA node、watermark、页分配、页迁移和本次 boot 中的内存拓扑运行态。

#### RT-06-03 进程内存映射与进程级内存占用

- 判定标准：信息描述进程地址空间、映射摘要、RSS/PSS、匿名页、文件映射和进程级内存使用关系。

#### RT-06-04 swap、zswap、zram 与交换工作集

- 判定标准：信息描述 swap 设备、交换使用、换入换出、zswap、zram 和被换出的当前工作集状态。

#### RT-06-05 页回收、规整、碎片与 THP 状态

- 判定标准：信息描述内存回收、direct reclaim、compaction、fragmentation、THP、hugepage 和 KSM 相关运行态。

#### RT-06-06 OOM、内存压力与内存 stall

- 判定标准：信息描述 OOM killer、内存分配失败、内存 PSI、压力触发器和内存紧张相关的当前状态。

### RT-07 打开句柄、文件引用与内核同步对象

#### RT-07-01 fd 表与打开文件引用

- 判定标准：信息描述进程持有的 fd 表、打开文件对象、文件偏移、打开标志和引用计数关系。

#### RT-07-02 已删除文件、匿名文件与 memfd 引用

- 判定标准：信息描述仍被进程持有但路径已消失、未具名或以内存文件形式存在的文件引用。

#### RT-07-03 pipe、eventfd、timerfd、signalfd 与 pidfd

- 判定标准：信息描述通过 fd 暴露的匿名内核对象及其当前等待、计数、触发或目标关系。

#### RT-07-04 epoll、inotify、fanotify 与 io_uring 对象

- 判定标准：信息描述进程持有的事件复用、文件通知和异步 I/O 内核对象的当前引用和队列状态。

#### RT-07-05 文件锁、lease、futex 与同步关系

- 判定标准：信息描述 flock、POSIX lock、lease、futex 等内核维护的同步、互斥、等待和持有关系。

#### RT-07-06 socket fd 引用

- 判定标准：信息描述 socket 作为进程 fd 被持有的引用关系，不描述 socket 协议状态本身。

### RT-08 临时文件系统与运行时目录状态

#### RT-08-01 tmpfs 与易失文件系统实例

- 判定标准：信息描述由内存或重启清理策略承载的临时文件系统实例、容量和当前占用。

#### RT-08-02 运行时目录与守护进程状态目录

- 判定标准：信息描述系统和守护进程在当前 boot 中创建的运行时目录、所有权、权限和生命周期。

#### RT-08-03 PID 文件、锁文件与协调文件

- 判定标准：信息描述用于 OS 或守护进程协调的 PID 文件、锁文件、状态标记和当前引用关系。

#### RT-08-04 Unix socket 文件与运行时入口

- 判定标准：信息描述出现在临时目录中的 Unix socket 文件、控制入口和服务间本机通信端点。

#### RT-08-05 tmpfs 共享内存目录项与临时挂载点

- 判定标准：信息描述临时文件系统中的共享内存路径、目录项、权限、所有权、引用状态、临时挂载点目录项和本次 boot 中生成的路径对象。

#### RT-08-06 残留、失效与清理策略运行态

- 判定标准：信息描述临时目录中残留、失效、被引用或等待清理的运行时文件和目录状态。

### RT-09 VFS、文件系统与挂载状态

#### RT-09-01 挂载拓扑与挂载选项

- 判定标准：信息描述当前 VFS 挂载树、挂载点、挂载选项、只读状态和实际暴露的路径空间。

#### RT-09-02 mount namespace 视图与传播关系

- 判定标准：信息描述不同 mount namespace 中的挂载视图、共享传播、私有传播和 bind 关系。

#### RT-09-03 superblock 与文件系统运行状态

- 判定标准：信息描述 superblock、文件系统实例、运行时特征开关和文件系统级健康状态。

#### RT-09-04 journal、quota、dirty inode 与写回状态

- 判定标准：信息描述文件系统 journal、quota 当前状态、dirty inode、写回积压和提交现场。

#### RT-09-05 overlay、FUSE、bind 与组合挂载关系

- 判定标准：信息描述 overlay、FUSE、bind mount 等将多个文件系统视图组合成当前路径空间的运行态。

#### RT-09-06 文件系统错误、只读切换与保护状态

- 判定标准：信息描述文件系统错误、自动只读切换、recovery 现场和保护性降级状态。

### RT-10 块设备、存储映射与 I/O 路径状态

#### RT-10-01 块设备身份、可用性与队列状态

- 判定标准：信息描述当前块设备对象、可用性、大小暴露、打开关系和请求队列基础状态。

#### RT-10-02 I/O scheduler、request queue 与拥塞状态

- 判定标准：信息描述 I/O scheduler、request queue、队列深度、合并、调度和 block layer 拥塞。

#### RT-10-03 device-mapper、dm-crypt 与 LVM 映射

- 判定标准：信息描述 device-mapper 目标、dm-crypt、LVM 逻辑卷和当前映射表运行态。

#### RT-10-04 mdraid、multipath 与路径选择状态

- 判定标准：信息描述 mdraid、multipath、路径可用性、路径切换、降级和重建相关运行态。

#### RT-10-05 loop、zram 与虚拟块设备

- 判定标准：信息描述由文件、内存压缩或内核虚拟化机制形成的块设备及其当前后端关系。

#### RT-10-06 NVMe、SCSI、iSCSI、NBD 与存储传输运行态

- 判定标准：信息描述 NVMe、SCSI、iSCSI、NBD、NVMe-oF 等存储传输层的控制器、会话、命令队列、错误和路径运行状态。

#### RT-10-07 I/O 错误、延迟、计数器与压力

- 判定标准：信息描述块层 I/O 错误、延迟、重试、统计计数和 I/O pressure stall information。

### RT-11 网络接口、链路、地址与路由状态

#### RT-11-01 接口身份、链路状态与队列

- 判定标准：信息描述网络接口对象、link up/down、MTU、MAC、carrier、队列、offload 特性和接口运行标志。

#### RT-11-02 二层虚拟设备与链路组合

- 判定标准：信息描述 bridge、bond、vlan、macvlan、ipvlan、veth、tap、tun 等内核网络设备的当前关系。

#### RT-11-03 IP 地址、作用域与临时地址

- 判定标准：信息描述接口上的 IPv4/IPv6 地址、作用域、生命周期、临时地址和地址选择上下文。

#### RT-11-04 路由表、策略路由与路径选择结果

- 判定标准：信息描述路由表、规则、策略路由和内核当前三层路径选择状态。

#### RT-11-05 邻居表、ARP、NDP 与二层解析缓存

- 判定标准：信息描述邻居项、ARP、NDP、二层解析状态、可达性和失效时间。

#### RT-11-06 网络命名空间内接口视图

- 判定标准：信息描述不同 network namespace 中接口、地址、路由和邻居状态的可见性差异。

#### RT-11-07 接口统计、错误、丢包与链路压力

- 判定标准：信息描述接口级收发计数、错误、丢包、队列积压、重传侧信号和链路侧压力。

#### RT-11-08 隧道、封装与内核网络端点状态

- 判定标准：信息描述 vxlan、geneve、gre、ipip、sit、wireguard 等由内核维护的隧道、封装和端点运行态。

### RT-12 socket、传输协议与连接状态

#### RT-12-01 监听 socket 与绑定端点

- 判定标准：信息描述当前监听 socket、绑定地址、端口、协议族、backlog 和可接受连接状态。

#### RT-12-02 TCP、MPTCP、SCTP 等连接与传输状态机

- 判定标准：信息描述 TCP、MPTCP、SCTP 等传输协议连接的当前状态机、端点、窗口、重传和拥塞控制状态。

#### RT-12-03 UDP、raw、netlink 与 Unix socket 协议状态

- 判定标准：信息描述无连接 socket、内核通信 socket 和 Unix domain socket 在协议层的运行态。

#### RT-12-04 socket buffer 与发送接收队列

- 判定标准：信息描述 socket send/receive buffer、队列长度、积压、内存占用和等待关系。

#### RT-12-05 TIME_WAIT、orphan、SYN backlog 与异常连接集合

- 判定标准：信息描述短生命周期、半连接、孤儿连接和异常连接集合的当前状态。

#### RT-12-06 协议栈计数器与拥塞控制运行态

- 判定标准：信息描述传输协议栈计数器、拥塞算法当前状态和协议层异常计数。

#### RT-12-07 socket inode 与进程关联

- 判定标准：信息描述 socket inode、协议对象和持有进程之间的运行时关联，不替代 fd 引用分类。

### RT-13 包过滤、NAT、连接跟踪与流量控制状态

#### RT-13-01 过滤规则集与规则命中状态

- 判定标准：信息描述 nftables、iptables、ip6tables、ebtables 等当前实际生效的过滤规则和命中计数。

#### RT-13-02 set、map、flowtable 与规则辅助对象

- 判定标准：信息描述数据面规则依赖的 set、map、flowtable、动态集合和其当前成员状态。

#### RT-13-03 conntrack、NAT 映射与连接跟踪表

- 判定标准：信息描述连接跟踪对象、NAT 映射、状态转换、超时和表容量压力。

#### RT-13-04 IPVS 与内核负载均衡状态

- 判定标准：信息描述 IPVS virtual service、real server、调度状态、连接表和健康状态。

#### RT-13-05 qdisc、class、filter 与流量队列

- 判定标准：信息描述 tc qdisc、class、filter、整形、限速和排队现场。

#### RT-13-06 XDP、tc 附着点与数据面程序引用

- 判定标准：信息描述 XDP、tc 等网络数据面附着点、附着顺序、命中计数和引用的内核程序对象。

#### RT-13-07 XFRM、IPsec state 与 policy

- 判定标准：信息描述 XFRM/IPsec 的 state、policy、安全关联和当前包处理约束。

#### RT-13-08 包标记、限流与数据面计数器

- 判定标准：信息描述 packet mark、rate limit、hashlimit、recent、配额和其他数据面临时计数状态。

### RT-14 IPC、namespace 与 cgroup 状态

#### RT-14-01 SysV IPC 与 POSIX IPC 对象

- 判定标准：信息描述消息队列、共享内存、信号量、POSIX message queue 等 IPC 对象的身份、权限、容量和引用。

#### RT-14-02 namespace 实例与引用关系

- 判定标准：信息描述 PID、network、mount、UTS、IPC、user、cgroup、time namespace 的实例身份和被进程引用关系。

#### RT-14-03 user namespace ID 映射与权限边界

- 判定标准：信息描述 user namespace 中 UID/GID 映射、能力边界和跨 namespace 权限解释关系。

#### RT-14-04 cgroup 层级、controller 与生效配置

- 判定标准：信息描述 cgroup v1/v2 层级、controller 启用状态、资源控制配置和继承关系。

#### RT-14-05 cgroup 成员关系与资源限制

- 判定标准：信息描述进程或线程在 cgroup 中的成员关系，以及 CPU、memory、I/O、pids 等限制的当前效果。

#### RT-14-06 cgroup 统计、事件与 pressure

- 判定标准：信息描述 cgroup 级使用统计、事件计数、pressure、OOM 事件和限流事件。

#### RT-14-07 IPC 等待、引用与清理状态

- 判定标准：信息描述 IPC 对象上的等待者、持有者、引用计数和待清理状态，不包含应用 payload。

### RT-15 用户登录、会话与临时认证状态

#### RT-15-01 当前登录用户、session 与 seat

- 判定标准：信息描述当前登录用户、systemd-logind session、seat、session 类型和会话生命周期。

#### RT-15-02 TTY、PTY 与控制终端关系

- 判定标准：信息描述 TTY/PTY 设备、控制终端、前台进程组和交互式会话绑定关系。

#### RT-15-03 SSH 与远程登录运行态

- 判定标准：信息描述 SSH 等远程登录产生的当前会话、伪终端、认证上下文和进程关系。

#### RT-15-04 PAM session、loginuid 与 utmp 当前记录

- 判定标准：信息描述 PAM session、loginuid、utmp 当前会话记录和登录审计上下文。

#### RT-15-05 用户级 systemd manager 与 session bus

- 判定标准：信息描述用户级 service manager、user bus、用户会话服务和其与登录会话的关联。

#### RT-15-06 sudo、Kerberos 与临时认证材料

- 判定标准：信息描述 sudo credential cache、Kerberos credential cache 和其他用户临时认证材料的当前有效状态。

### RT-16 安全策略、凭据与访问控制运行态

#### RT-16-01 LSM 模式、策略加载与约束状态

- 判定标准：信息描述 SELinux、AppArmor、Landlock、BPF LSM 等 LSM 的当前模式、策略加载状态和实际约束状态。

#### RT-16-02 进程凭据、安全标签与访问主体

- 判定标准：信息描述进程当前 UID、GID、supplementary group、安全标签和访问主体身份。

#### RT-16-03 capability、seccomp、securebits 与 no_new_privs

- 判定标准：信息描述进程 capability 集合、seccomp filter、securebits、ambient capability 和 no_new_privs 运行态。

#### RT-16-04 keyring、临时密钥与内核密钥引用

- 判定标准：信息描述内核 keyring、session keyring、临时密钥、密钥引用和有效期状态。

#### RT-16-05 audit 规则、状态与运行计数

- 判定标准：信息描述 audit 当前规则、启用状态、backlog 限制和运行计数，不包含易失事件缓冲内容。

#### RT-16-06 随机性、熵池与安全初始化状态

- 判定标准：信息描述 kernel random、entropy、CRNG 初始化和安全随机性相关当前状态。

#### RT-16-07 IMA、EVM 与完整性度量运行态

- 判定标准：信息描述 IMA/EVM 的当前策略、度量状态、校验状态和访问控制影响。

### RT-17 设备、驱动、总线与 udev 运行状态

#### RT-17-01 设备枚举、设备节点与可见性

- 判定标准：信息描述 OS 当前发现的设备对象、设备节点、主次设备号和用户态可见性。

#### RT-17-02 总线拓扑与设备层级

- 判定标准：信息描述 PCI、USB、SCSI、NVMe、virtio 等总线拓扑、设备层级和枚举状态。

#### RT-17-03 驱动绑定、probe 与 deferred probe

- 判定标准：信息描述设备与驱动的当前绑定、probe 结果、deferred probe 和解绑状态。

#### RT-17-04 固件加载与驱动初始化结果

- 判定标准：信息描述运行中固件加载、驱动初始化、失败原因和设备准备状态。

#### RT-17-05 udev database、规则结果与设备属性

- 判定标准：信息描述 udev 当前数据库、规则处理结果、设备属性和用户态可见的设备状态，不包含尚未处理的易失事件队列。

#### RT-17-06 设备错误、reset 与运行时属性

- 判定标准：信息描述设备错误计数、reset 状态、运行时 sysfs 属性和驱动暴露的当前状态。

### RT-18 电源、频率、温度与硬件健康运行状态

#### RT-18-01 CPU 频率、governor 与调频状态

- 判定标准：信息描述 CPU frequency governor、当前频率、频率限制和调频策略的运行效果。

#### RT-18-02 cpuidle、runtime PM 与电源管理状态

- 判定标准：信息描述 CPU idle、设备 runtime power management、电源域和省电状态。

#### RT-18-03 thermal zone、cooling device 与节流

- 判定标准：信息描述 thermal zone、cooling device、温度、热告警和硬件或内核触发的节流状态。

#### RT-18-04 风扇、电源供应与平台电源输入

- 判定标准：信息描述风扇、电源供应、电池、AC 输入和平台电源当前状态。

#### RT-18-05 EDAC、MCE、RAS 与硬件错误

- 判定标准：信息描述 ECC、EDAC、MCE、RAS、内存硬件错误和平台报告的当前硬件健康状态。

#### RT-18-06 平台健康、节能限制与降级状态

- 判定标准：信息描述由固件、平台或 OS 暴露的健康计数、功耗限制、降级和保护状态。

### RT-19 时间、时钟源、定时器与计划任务状态

#### RT-19-01 wall clock、monotonic 与 boottime 关系

- 判定标准：信息描述当前 wall clock、monotonic、boottime、uptime 之间的时间关系和偏移。

#### RT-19-02 clocksource、clockevent 与时钟精度状态

- 判定标准：信息描述当前 clocksource、clockevent、时钟稳定性、精度和切换状态。

#### RT-19-03 NTP、PTP 与时间同步状态

- 判定标准：信息描述 NTP、PTP、时钟偏移、同步源、同步质量和当前校时状态。

#### RT-19-04 time namespace 与时间偏移视图

- 判定标准：信息描述 time namespace 中的时间偏移、可见时间关系和被进程引用状态。

#### RT-19-05 内核 timer、hrtimer 与超时对象

- 判定标准：信息描述内核 timer、hrtimer、超时对象和等待触发的时间事件。

#### RT-19-06 workqueue 与异步任务积压

- 判定标准：信息描述 workqueue、worker、delayed work、pending work 和异步内核任务执行积压。

#### RT-19-07 cron、systemd timer 与当前任务实例

- 判定标准：信息描述计划任务在当前 boot 中已经触发、正在运行、等待运行或错过触发的实例状态。

### RT-20 易失事件缓冲、追踪缓冲与内存日志状态

#### RT-20-01 kernel ring buffer 与 dmesg 现场

- 判定标准：信息描述内核环形日志缓冲中的当前事件、顺序、覆盖状态和未持久化片段。

#### RT-20-02 volatile journald 与内存日志

- 判定标准：信息描述未持久化的 journald 事件、内存日志队列和当前 boot 内日志现场。

#### RT-20-03 audit backlog 与易失审计队列

- 判定标准：信息描述 audit 事件 backlog、丢弃、限流和等待写出的易失审计事件状态。

#### RT-20-04 trace、ftrace 与 perf buffer

- 判定标准：信息描述 trace buffer、ftrace buffer、perf buffer 中当前保存的事件和覆盖状态。

#### RT-20-05 BPF ring buffer 与动态观测输出

- 判定标准：信息描述 BPF ring buffer、perf event output 等动态观测路径中的易失输出状态。

#### RT-20-06 驱动内部错误缓冲与设备事件队列

- 判定标准：信息描述驱动、设备子系统或 udev 路径在内存中维护的错误缓冲、未处理事件队列和未落盘现场。

#### RT-20-07 系统守护进程内存事件队列

- 判定标准：信息描述系统守护进程尚未持久化或尚未处理的内存事件队列和状态缓冲。

#### RT-20-08 rate limit、抑制与丢失事件状态

- 判定标准：信息描述内核或系统守护进程因限流、抑制、丢弃而造成的事件缺口和当前计数。

### RT-21 OS 缓存、解析器与派生运行状态

#### RT-21-01 DNS resolver 缓存

- 判定标准：信息描述系统 DNS 解析器、systemd-resolved、nscd 等维护的 DNS 缓存和当前解析结果。

#### RT-21-02 NSS、用户组与名称服务缓存

- 判定标准：信息描述 NSS、用户、组、主机名、服务名等 OS 名称解析缓存和失效状态。

#### RT-21-03 page cache 与文件数据缓存占用

- 判定标准：信息描述 page cache 中由文件访问派生出的缓存占用、冷热关系和回收压力，不包含业务内容语义。

#### RT-21-04 dentry、inode 与 slab cache

- 判定标准：信息描述 dentry cache、inode cache、slab cache 和内核对象缓存的当前占用与回收状态。

#### RT-21-05 负查找、路径解析与派生索引缓存

- 判定标准：信息描述负目录项、路径解析缓存和由 OS 子系统派生出的临时索引状态。

#### RT-21-06 网络路径派生缓存与协议辅助缓存

- 判定标准：信息描述由网络路径选择、协议栈计算或策略计算派生出的临时缓存结果；邻居表对象本体仍归 RT-11。

#### RT-21-07 设备属性、udev 派生属性与硬件视图缓存

- 判定标准：信息描述设备属性缓存、udev 派生属性和 OS 为快速呈现设备视图维护的缓存。

#### RT-21-08 系统守护进程派生缓存

- 判定标准：信息描述系统守护进程为了加速 OS 级解析、寻址、访问或状态展示而维护的临时缓存。

## 相邻边界规则

| 边界问题 | 主归属 |
| --- | --- |
| 进程调度属性与系统调度压力 | 进程自身的策略、优先级和 affinity 归 RT-04；调度队列、负载和 CPU pressure 归 RT-05。 |
| 进程内存占用与内存子系统压力 | 进程地址空间和内存占用归 RT-06；进程身份和执行上下文归 RT-04；cgroup memory 限制归 RT-14。 |
| socket fd 与协议连接 | socket 作为 fd 的持有关系归 RT-07；监听、连接、队列和协议状态归 RT-12。 |
| network namespace 与网络状态 | namespace 实例身份归 RT-14；namespace 内接口、地址、路由和邻居视图归 RT-11。 |
| cgroup 资源限制与资源压力 | cgroup 层级、controller、成员和限制归 RT-14；系统级 CPU、内存和 I/O 压力分别归 RT-05、RT-06、RT-10。 |
| timer unit 与内核 timer | 服务管理器中的 timer unit 生命周期归 RT-03；内核 timer、hrtimer、workqueue 和当前计划任务实例归 RT-19。 |
| audit 规则与 audit 事件队列 | audit 规则、启用状态和运行计数归 RT-16；audit backlog 和易失事件队列归 RT-20。 |
| tmpfs 共享内存目录项与 IPC 对象 | 临时文件系统中的路径、目录项和权限归 RT-08；SysV/POSIX IPC 对象身份、容量、等待和引用关系归 RT-14。 |
| udev database 与 udev 事件队列 | udev database、规则处理结果和设备属性归 RT-17；尚未处理或未持久化的 udev 事件队列归 RT-20。 |
| BPF 对象与 BPF 生效位置 | eBPF program、map、link 对象本体归 RT-02；网络数据面附着效果归 RT-13；BPF LSM 的访问控制效果归 RT-16。 |
| 网络路径缓存与邻居表 | 邻居项、ARP、NDP 等网络对象本体归 RT-11；由路径选择或协议栈计算派生出的临时缓存归 RT-21。 |
| 缓存对象与缓存内容 | OS 缓存对象、命中和占用关系归 RT-21；其中承载的应用业务 payload 不因此纳入运行时信息分类。 |

## 完整性校验

| 大类 | 二级分类数量 |
| --- | --- |
| RT-01 | 4 |
| RT-02 | 5 |
| RT-03 | 7 |
| RT-04 | 7 |
| RT-05 | 6 |
| RT-06 | 6 |
| RT-07 | 6 |
| RT-08 | 6 |
| RT-09 | 6 |
| RT-10 | 7 |
| RT-11 | 8 |
| RT-12 | 7 |
| RT-13 | 8 |
| RT-14 | 7 |
| RT-15 | 6 |
| RT-16 | 7 |
| RT-17 | 6 |
| RT-18 | 6 |
| RT-19 | 7 |
| RT-20 | 8 |
| RT-21 | 8 |

本文覆盖 `RT-01` 到 `RT-21` 的全部大类，并为每个大类建立至少 4 个二级分类。后续新增采集项时，应先选择最具体的二级分类；如果无法归入现有二级分类，应先判断是二级分类缺口、一级大类缺口，还是候选信息本身不属于 Linux OS 运行时信息。
