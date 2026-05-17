# 0003 - 固定 collector 实现技术栈

- **状态**：生效
- **日期**：2026-05-15
- **范围**：`collector`

## 背景

- `docs/collector-security-governance.md` 和 `docs/collector-testing-governance.md` 已经固定了采集安全约束和测试治理基线，ADR 0002 的安全与测试治理门槛已落地。
- collector 需要在不可预测的主机上运行：主机可能不稳定、资源紧张、重启窗口有限。
- collector 不得引入运行时依赖（动态链接库、外部命令、守护进程），必须是自包含的单二进制文件。
- 所有采集必须通过原生 OS 接口（procfs、sysfs、netlink、D-Bus、syscall）完成，不得包装外部命令。

## 决策

### 语言

选择 **Rust**，不使用其他语言。

理由：

- 内存安全可静态分析，避免 C/C++ 在紧张主机上的 UB 风险，无需额外 ASan/UBSan CI 验证。
- 零成本抽象，无 GC，适合低资源占用场景。
- `x86_64-unknown-linux-musl` target 可生成纯静态二进制，无 glibc 版本约束。
- 生态中已有纯 Rust 实现的 procfs 解析器、D-Bus 客户端、netlink 协议栈和 netfilter 通信库。

### 异步运行时

使用 **tokio**（multi-threaded runtime），无手动线程数配置。

- worker 线程数由 `std::thread::available_parallelism()` 自动决定。
- 所有采集函数返回 `async`，独立任务可被 tokio 自动并行调度。
- 逐项超时通过 `tokio::time::timeout` 实现。
- procfs/sysfs 文件读取本身是同步操作（虚拟文件系统不阻塞），在 async 上下文中直接调用，不阻塞 worker。

### 配置原则

**零参数**。collector 不接受配置文件、环境变量调优参数和 CLI 调参标志。

唯一输入是输出目录路径（默认当前目录）。所有内部参数直接引用 `docs/collector-security-governance.md` 中的硬编码值：

- 逐项超时：2 秒（常规项）、10 秒（量大项）
- 输出大小上限：64 MiB/项
- 条目截断上限：50000
- 递归深度上限：3
- 全局超时：300 秒
- 输出文件权限：0600 或 0640（owner root）

这些值不因主机规格而变化。它们被选为在最低配置服务器上也不会引发风险的上限，在大型主机上同样适用。

### 分发方式

`x86_64-unknown-linux-musl` target 编译纯静态二进制，零运行时依赖。

首轮目标平台 Rocky Linux 8.x / systemd / 内核 4.18.x / x86_64。其他平台不拒绝运行，但不承诺行为一致（按 `docs/collector-testing-governance.md` 记录 `unsupported platform`）。

### Crate 依赖

| 用途 | Crate | 选型理由 |
| --- | --- | --- |
| 异步运行时 | `tokio` 1.x (features: rt-multi-thread, fs, time, macros) | 多线程异步 runtime，所有异步采集、超时控制和并发调度。 |
| /proc 解析（进程、meminfo、/proc/net/* 等） | `procfs` 0.18 | 纯 Rust，活跃维护，覆盖全部 /proc 文件语法解析。 |
| systemd D-Bus 查询 | `zbus` 5.x | 纯 Rust，异步原生，零 C 依赖。 |
| 网口、地址、路由、邻居查询 | `rtnetlink` 0.21 + `netlink-packet-route` | 纯 Rust netlink 协议栈，原生 tokio 后端。 |
| conntrack 表查询 | `conntrack` crate | 社区存在且活跃维护，通过 netlink 读取连接跟踪表。备选路径 `/proc/net/nf_conntrack` 文本解析兜底。 |
| nftables 规则集导出 | `neli` 0.7 构建 nftables netlink 消息 | `netlink-packet-netfilter` 已废弃约 3 年，社区无可用的 nftables crate。`neli` 是纯 Rust 泛型 netlink 库，用于构建 `NFNL_SUBSYS_NFTABLES` 消息。这不是从零手写 raw socket，而是使用已有 netlink 基础库构建上层协议消息。 |
| qdisc/tc 查询 | `rtnetlink` tc 消息类型 + `neli` 兜底 | 优先 `rtnetlink` 已有封装，不足时用 `neli` 发送 `RTM_GETQDISC`。 |
| dmesg 读取 | `nix::sys::syslog` | klogctl syscall 的标准封装。 |
| 序列化 | `serde` 1.x + `serde_json` 1.x | 所有结构化输出。 |
| gzip 压缩 | `flate2` 1.x | 纯 Rust DEFLATE 实现，tar.gz 归档必需的压缩层。 |
| tar 打包 | `tar` 0.4 | 目录输出打包为单文件分发，配合 `flate2` 生成 `.tar.gz`。 |
| 时间格式化 | `chrono` 0.4 | 输出目录名和 manifest 时间戳的本地时区格式化。 |
| 错误类型 | `thiserror` 2.x | 派生 `std::error::Error` 实现，替代手写 `Result<T, String>`。 |
| 日志 | `tracing` 0.1 + `tracing-subscriber` 0.3 | 结构化异步日志，替代 `eprintln!` / `println!`。 |
| 临时文件 | `tempfile` 3.x | 原子临时文件创建 + `persist()` 重命名，消除 TOCTOU race 和 panic 安全问题。 |

`procfs`、`zbus`、`neli`、`serde`、`serde_json`、`flate2`、`tar`、`chrono`、`thiserror`、`tracing`、`tracing-subscriber` 和 `tempfile` 均为纯 Rust，不依赖 C 代码。`nix` 和 `rtnetlink` 在构建树中依赖 `libc` crate 进行 syscall 绑定，`conntrack` 通过 `neli` 间接依赖 `libc`，但 musl 目标下全部链接到 musl libc 并静态打进二进制，不产生运行时动态库依赖。

## 影响

- collector 源码全部为 Rust，不得包含 C/C++ 代码或 FFI 调用非 Rust 库。
- 新增依赖前必须验证该 crate 为纯 Rust 且活跃维护，不得引入运行期 C 库依赖。
- `Cargo.toml` 中所有依赖均使用 `[dependencies]` 段声明（无需平台限定：collector 仅面向 Linux，跨平台构建非目标场景）。
- CI 使用 `x86_64-unknown-linux-musl` target 编译，输出单个静态 ELF。
- collector 不得通过 `std::process::Command` 调用任何外部命令；所有数据来源必须是 procfs、sysfs、netlink socket、D-Bus 连接或 syscall。
- 配置参数不从文件、环境变量或命令行读取；修改硬编码值必须先更新 `docs/collector-security-governance.md` 中对应的约束值。

## 备选方案

| 方案 | 未采用原因 |
| --- | --- |
| Go | 静态二进制分发成熟，但 netlink 操作依赖 C 或非标准 syscall 封装，D-Bus 生态不如 Rust；GC 暂停在资源紧张场景不可控。 |
| Python | 无法生成单静态二进制；进程启动和内存占用不适合资源紧张主机。 |
| Shell | 无法满足有界执行和超时控制约束；text-parsing 脆弱。 |
| C/C++ | 内存安全需额外 sanitizer 和 CI 验证，增加治理成本；缺少统一的异步 runtime 和 netlink/D-Bus 生态。 |
| Rust + 单线程 tokio | 单线程无法利用多核并行采集独立子系统；且在能力探测和 D-Bus 连接等 I/O 密集阶段会串行等待。 |

## 验证

- 人工核对所有选定 crate 均为纯 Rust、活跃维护且不引入运行期 C 依赖。
- 人工确认 `netlink-packet-netfilter` 自 2023 年 7 月起未更新，社区无替代品，`neli` 是构建 nftables netlink 消息的最小可行方案。
- 人工审核硬编码参数值与 `docs/collector-security-governance.md` 中的约束值一致。
- 本决策不引入新的外部服务、构建系统或发布流程依赖。
