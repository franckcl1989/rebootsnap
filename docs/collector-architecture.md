# Collector 架构

## 文档定位

本文是 RebootSnap collector 实现的总体架构文档。本文在上游设计决策（ADR 0003 技术栈、ADR 0004 输出格式）的约束下，定义模块结构、运行时模型、数据流和实现顺序。

本文不替代上游治理文档（`docs/collector-security-governance.md`、`docs/collector-testing-governance.md`）中的约束，也不重复 `docs/decisions/0003-implementation-tech-stack.md` 和 `docs/decisions/0004-output-format.md` 中的决策理由。本文是这些上游决策的架构级落实。

## 模块结构

```
src/
  main.rs               # 入口：初始化 tracing → 创建输出目录 → probe → spawn collect → manifest → summary → tar.gz
  error.rs              # 错误类型（CollectError / OutputError / ArchiveError，thiserror 派生）
  output.rs             # 输出目录创建、JSON/JSONL/Text 流式写入（tempfile 临时文件 + persist 原子重命名）、截断控制
  manifest.rs           # manifest.json 构建（预留模块，当前内联于 main.rs）
  summary.rs            # summary.json 指标计算（预留模块，当前内联于 main.rs）
  archive.rs            # tar.gz 打包与清理（预留模块，当前内联于 main.rs）

  collector/
    mod.rs              # CollectionTask enum 定义 + ProbeOutcome/CollectionOutcome/CollectionStatus 类型 + 串行 probe 分发
    boot.rs             # RT-01 启动实例身份
    kernel.rs           # RT-02 内核状态与参数
    systemd.rs          # RT-03 init/systemd 控制面
    process.rs          # RT-04 进程与线程
    cpu.rs              # RT-05 CPU、调度与中断
    memory.rs           # RT-06 内存、虚拟内存与 swap
    fd.rs               # RT-07 打开句柄与内核引用
    filesystem.rs       # RT-22 磁盘取证（挂载点容量、块设备清单）
    tmpfs.rs            # RT-08 临时文件系统
    mount.rs            # RT-09 VFS 与挂载
    block.rs            # RT-10 块设备与存储 I/O
    netdev.rs           # RT-11 网络接口与路由
    socket.rs           # RT-12 socket 与连接
    netfilter.rs        # RT-13 包过滤与流量控制
    ipc_ns_cg.rs        # RT-14 IPC、namespace 与 cgroup
    session.rs          # RT-15 用户登录与会话
    security.rs         # RT-16 安全策略与凭据
    device.rs           # RT-17 设备与驱动
    power.rs            # RT-18 电源与硬件健康
    time.rs             # RT-19 时间与计划任务
    events.rs           # RT-20 易失事件缓冲
    cache.rs            # RT-21 OS 缓存与解析器
```

`collector/mod.rs` 的职责：

- 定义 `CollectionTask` enum，每个变体包装一个 collector struct。
- 定义 `ProbeOutcome`、`CollectionOutcome`、`CollectionStatus` 返回类型。
- 通过 enum dispatch (`match self`) 提供统一的 `id()`、`filename()`、`item_timeout()`、`probe()` 接口。

所有 collector struct 均为 unit struct（零大小）。不使用 trait object 或动态分发。
并发采集由 `main.rs` 直接调用各 struct 的 `collect()` 方法并通过 tokio `JoinSet` 调度。

## CollectionTask enum 与返回类型

```rust
/// 所有 collector 的编译期注册表（22 变体，RT-01 至 RT-22 全覆盖）。
pub enum CollectionTask {
    Block(block::Block), Boot(boot::Boot), Cache(cache::Cache), Cpu(cpu::Cpu),
    Device(device::Device), Events(events::Events), Fd(fd::Fd),
    Filesystem(filesystem::Filesystem), IpcNsCg(ipc_ns_cg::IpcNsCg),
    Kernel(kernel::Kernel), Memory(memory::Memory),
    Mount(mount::Mount), Netdev(netdev::Netdev), Netfilter(netfilter::Netfilter),
    Power(power::Power), Process(process::Process), Security(security::Security),
    Session(session::Session), Socket(socket::Socket), Systemd(systemd::Systemd),
    Time(time::Time), Tmpfs(tmpfs::Tmpfs),
}

impl CollectionTask {
    pub fn id(&self) -> &'static str { ... }
    pub fn filename(&self) -> &'static str { ... }
    pub fn item_timeout(&self) -> Duration { ... }
    pub async fn probe(&self) -> ProbeOutcome { ... }
    pub async fn collect(&self, output: &OutputDir, probe: &ProbeOutcome) -> CollectionOutcome { ... }
}

/// 一次能力探测的结果。
pub struct ProbeOutcome {
    pub available: bool,
    pub degraded: Vec<String>,
    pub reason: Option<String>,
    pub roots: FsRoots,
}

/// 一次采集的结果。除 status/duration/file_size 外，还携带 summary 构建所需的提取值，
/// 避免下游从磁盘重读已采集数据。
pub struct CollectionOutcome {
    pub status: CollectionStatus,
    pub duration: Duration,
    pub file_size: u64,
    pub items_total: Option<u64>,
    pub items_collected: Option<u64>,
    // summary 提取值（仅相关 collector 填充）
    pub mem_total_kb: Option<i64>,
    pub mem_available_kb: Option<i64>,
    pub hostname: Option<String>,
    pub kernel_version: Option<String>,
    pub boot_id: Option<String>,
    pub uptime_seconds: Option<u64>,
}

pub enum CollectionStatus {
    Ok,
    Truncated { reason: String },
    Partial { degrading: Vec<String> },
    Unsupported { reason: String },
    PermissionDenied,
    Failed { reason: String },
    TimedOut,
}
```

## 运行时模型

### 异步调度

- tokio `#[tokio::main]` multi-threaded runtime。
- worker 线程数自动：`available_parallelism()`（tokio 默认行为）。
- 不人为设定线程数上限或比例。

### 采集并发

`main` 中的执行序列：

1. **probe 阶段（串行，同步等待）**：遍历所有注册的 Collector，调用 `probe()` 并汇总 `ProbeOutcome`。probe 只做接口存在性检查（`Path::exists()`），成本极低，总耗时应在 1 秒内完成。

2. **collect 阶段（并发，按需调度）**：每个 collector 作为独立 tokio task 启动，外层包装 `tokio::time::timeout(item_timeout, task)`。所有 task 并发提交给 tokio runtime。tokio 自动在可用 worker 线程间调度：

```
task1: RT-01 [procfs]     ──────── (12ms)
task2: RT-02 [procfs]     ─────── (45ms)
task3: RT-03 [D-Bus]      ────────── (120ms)
task4: RT-04 [procfs]     ────────────────────── (850ms)
task5: RT-05 [procfs]     ─── (8ms)
task6: RT-06 [procfs]     ── (5ms)
task7: RT-11 [netlink]    ───────── (300ms)
...
                            ← tokio 自动调度到 N 个 worker →
```

3. **全局超时**：整个 collect 阶段包装在 `tokio::time::timeout(global_timeout, all_collect_futures)` 中。超时后尚未完成的 task 被取消，其临时文件随 `NamedTempFile` drop 自动删除。已完成的容器输出的临时文件已 `persist()` 为最终文件。未完成的 task 在 manifest 中标记为 `timed_out`。

4. **选采调度**：若未来引入可选 collector，在所有必采项完成后，若全局时间和文件大小预算仍有余量，按注册顺序逐个启动。选采项不计入全局完整性判断。

5. **浓缩阶段（串行）**：所有采集完成后，遍历已有输出，计算 `summary.json` 指标，写入 `manifest.json`。

6. **打包阶段**：通过 `tar` + `flate2` crate 构建 `rebootsnap-{timestamp}.tar.gz` 归档，完成后删除原始目录。

### I/O 并发特性

procfs 和 sysfs 是虚拟文件系统，`read` 不产生真实磁盘 I/O，在 async 上下文中直接调用 `tokio::fs::read_to_string` 或不阻塞 worker 的同步 `std::fs::read_to_string` 均可。netlink 和 D-Bus 涉及 socket 读写，由对应 crate 管理 I/O 复用。

同一输出文件不会被两个 task 同时写入——每个 Collector 对应唯一文件。

## 输出模块

`output.rs` 提供 `OutputDir` 结构体：

```rust
pub struct OutputDir {
    root: PathBuf,
}

impl OutputDir {
    /// 创建输出根目录（如 rebootsnap-20260515-143000/）。
    pub async fn create(base_dir: &Path) -> Result<Self, OutputError>;

    /// 基于已存在的目录构造 OutputDir（需先通过 create() 创建）。
    pub fn from_existing(root: PathBuf) -> Self;

    /// 打开一个 JSON 文件写入器。collector 先序列化到内存 buffer；
    /// 若 buffer 未超 64 MiB，写入 tempfile 并 persist 为最终文件名；
    /// 若超限，丢弃 buffer，记录为 truncated，不产生文件。
    pub fn json_writer(&self, filename: &str) -> Result<JsonWriter>;

    /// 打开一个 JSONL 流式写入器。每行独立 JSON 对象。
    /// 内部以 NamedTempFile 写入，完成后 persist。写入过程中追踪字节数，
    /// 达到 64 MiB 时写入截断标记行，停止接受后续行。
    pub fn jsonl_writer(&self, filename: &str) -> Result<JsonlWriter>;

}
```

所有 Writer 内部使用 `tempfile::NamedTempFile`：写入期间数据进入输出目录内的临时文件，`Writer` 的 `commit()` 或 `finish()` 时 `persist()` 原子重命名为目标文件名。若超时取消或 panic 导致 Writer drop 而未 commit/finish，`NamedTempFile::Drop` 自动删除临时文件——保证最终文件名不存在半截文件。`cleanup_tmp()` 作为额外保障清理任何遗留文件。`TextWriter` 已在 v0.1.1 移除，dmesg 改用 JSON 格式输出。

## RT 大类与接口映射

| RT | 大类 | 主要接口 | 主要 crate | 输出格式 |
| --- | --- | --- | --- | --- |
| RT-01 | 启动实例身份 | `/proc/sys/kernel/random/boot_id`, `/proc/uptime`, `/proc/version`, `/proc/cmdline`, `/proc/sys/kernel/hostname` | std::fs | JSON |
| RT-02 | 内核状态 | `/proc/sys/kernel/ostype`, `/proc/sys/kernel/osrelease`, `/proc/modules`, `/proc/sys/kernel/tainted`, `/proc/sys/kernel/core_pattern`, `/proc/sys/kernel/panic`, `/proc/sys/kernel/printk` | std::fs | JSON |
| RT-03 | 初始化系统 | org.freedesktop.systemd1 D-Bus | `zbus` | JSON |
| RT-04 | 进程与线程 | `/proc/[pid]/*` | `procfs` | JSONL |
| RT-05 | CPU 与调度 | `/proc/stat`, `/proc/loadavg`, `/proc/pressure/cpu`, `/proc/interrupts`, `/proc/softirqs` | std::fs | JSON |
| RT-06 | 内存 | `/proc/meminfo`（含 `procfs::Meminfo` 提取 MemTotal/MemAvailable），`/proc/pressure/memory`, `/proc/vmstat`, `/proc/zoneinfo` | std::fs + `procfs` | JSON |
| RT-07 | 打开句柄 | `/proc/sys/fs/file-nr`, `/proc/sys/fs/file-max`, `/proc/sys/fs/inode-nr`, `/proc/sys/fs/inode-max`, `/proc/locks` | std::fs | JSON |
| RT-08 | 临时文件系统 | `/proc/mounts`, `/run`, `/dev/shm`, `/tmp` 目录项计数 | std::fs | JSON |
| RT-09 | VFS 与挂载 | `/proc/self/mountinfo`, `/proc/self/mounts`, `/proc/self/mountstats`, `/proc/filesystems` | std::fs | JSON |
| RT-10 | 块设备 | `/proc/diskstats`, `/proc/partitions`, `/proc/pressure/io` | std::fs | JSON |
| RT-11 | 网络接口 | `/sys/class/net/*`（接口属性 + 统计），`/proc/net/route`, `/proc/net/ipv6_route`, `/proc/net/arp`, `/proc/net/netstat` | std::fs | JSON |
| RT-12 | socket | `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, `/proc/net/udp6`, `/proc/net/unix`, `/proc/net/raw`, `/proc/net/raw6`, `/proc/net/snmp`, `/proc/net/snmp6` | std::fs | JSON |
| RT-13 | 包过滤 | `/proc/net/nf_conntrack`, `/proc/net/stat/nf_conntrack`, `/proc/sys/net/nf_conntrack_max`, `/proc/net/nf_tables_names`, `/proc/net/xfrm_stat` | std::fs | JSON |
| RT-14 | IPC/ns/cgroup | `/proc/cgroups`, `/proc/sysvipc/msg`, `/proc/sysvipc/sem`, `/proc/sysvipc/shm` | std::fs | JSON |
| RT-15 | 用户会话 | `/var/run/utmp`（文件大小与 mtime），`/etc/passwd`, `/etc/group` | std::fs | JSON |
| RT-16 | 安全策略 | `/proc/sys/kernel/random/entropy_avail`, `/proc/sys/kernel/random/poolsize`, `/proc/sys/kernel/cap_last_cap`, `/proc/sys/crypto/fips_enabled` | std::fs | JSON |
| RT-17 | 设备 | `/proc/devices`, `/proc/iomem` | std::fs | JSON |
| RT-18 | 电源 | `/sys/power/state`, `/sys/power/disk` | std::fs | JSON |
| RT-19 | 时间 | `/proc/timer_list`, `/sys/class/rtc/rtc0/date`, `/sys/class/rtc/rtc0/time` | std::fs | JSON |
| RT-20 | 易失事件缓冲 | `/dev/kmsg`（内核环形缓冲区直读），`/proc/sys/kernel/printk`（日志级别） | std::fs | JSON |
| RT-21 | OS 缓存 | `/proc/slabinfo`, `/proc/meminfo` | std::fs | JSON |
| RT-22 | 磁盘取证 | `/proc/mounts` + `statvfs()`（按挂载点容量、inode 使用量），`/sys/class/block/*/`（块设备清单） | std::fs + `libc` | JSON |

## 实现阶段与优先级

按技术风险递增，不是按 RT 编号。

### 阶段 A：骨架与 procfs 通路

**内容**：项目骨架（`main.rs`、`error.rs`、`output.rs`）+ RT-01、RT-05、RT-06、RT-04。（`manifest.rs`、`summary.rs`、`archive.rs` 逻辑当前内联于 `main.rs`，后续按模块拆分。）

**优先级理由**：

- 这四个大类全是 procfs 读取，零外部协议依赖，技术风险最低。
- RT-04 是体积最大的大类，最先实现可以最早验证 JSONL 流式写入、条目截断和超时控制在规模场景下的行为。
- RT-01、RT-05、RT-06 覆盖了 OOM、CPU 打满、负载飙高三类最高频故障的入口证据。

**产出**：第一个可运行、可出 snapshot 的 collector 二进制。

### 阶段 B：systemd 通路

**内容**：RT-03。

**优先级理由**：

- 第一个依赖外部协议的大类。打通 `zbus` 后，全异步 D-Bus 链路验证完成。
- systemd unit/job/inhibitor 状态对"服务为何挂了"、"谁在阻止重启"的解释价值无可替代。

### 阶段 C：netlink 通路 ✅ 已完成

**内容**：RT-11、RT-12、RT-13。

**优先级理由**：

- netlink 是三个外部协议路径中技术复杂度最高的。
- RT-11（网口与路由）实际实现回退至 sysfs `/sys/class/net/*` + procfs 路由/ARP 读取，因 rtnetlink 0.21 与 netlink-packet-route 0.30 API 版本变化较大。
- RT-12（socket）走 procfs `/proc/net/*`，实际上是阶段 A 的延续。
- RT-13（netfilter）走 procfs `/proc/net/nf_conntrack` 等文件读取，`neli` 路径暂缓。

### 阶段 D：批量收尾 ✅ 已完成

**内容**：RT-02、RT-07、RT-08、RT-09、RT-10、RT-14、RT-15、RT-16、RT-17、RT-18、RT-19、RT-20、RT-21。

**优先级理由**：

- 全部是 procfs/sysfs 读取，技术零风险，工程上纯铺量。
- 有阶段 A~C 积累的 `CollectionTask` enum 模板和输出模块工具，每个大类 100~200 行即可完成。

### 阶段 E：测试体系与补全

**内容**：mock fixtures、集成测试。

**优先级理由**：

- mock fixture 不阻塞功能开发，但必须在上游消费者使用 collector 前完成。
- 按 `docs/collector-testing-governance.md` 五个场景构建 fixture。

## 硬编码参数速查

| 参数 | 值 | 来源 |
| --- | --- | --- |
| 逐项超时（常规） | 2 秒 | `docs/collector-security-governance.md` |
| 逐项超时（量大项：RT-04、RT-07、RT-09、RT-11、RT-12 等） | 10 秒 | 同上，大量条目遍历容许更长 |
| 输出大小上限 | 64 MiB/项 | 同上 |
| 条目截断上限 | 50000 | 同上 |
| 递归深度上限 | 3 级 | 同上 |
| 全局超时 | 300 秒 | 同上 |
| 输出文件权限 | 0600（文件），0700（目录） | 同上 |
| top_consumers 截取数 | 3 | 本文定义 |
| 输出根目录命名模板 | `rebootsnap-{local:%Y%m%d-%H%M%S}` | 本文定义 |
| tar 后缀 | `.tar.gz` | 本文定义 |

所有参数在源码中以常量定义，不从外部读取。

## 错误分类

collector 中发生的错误分为三类：

| 类别 | 语义 | collector 行为 |
| --- | --- | --- |
| 预期降级 | 接口不可用、权限不足、发行版不匹配 | 记录原因到 manifest，继续运行 |
| 资源超限 | 超时、输出截断、条目截断 | 标记截断点，保留已采集数据，继续运行 |
| 非预期失败 | panic、未处理的 fatal error | tokio task 级别的 panic 被 catch，标记对应 RT 为 `failed`，不影响其他 task |

collector 不会因任何单个采集项失败而整体退出。唯一的整体退出路径是全局超时（优雅退出，保留已完成数据）和无法创建的输出目录（启动前致命的）。
