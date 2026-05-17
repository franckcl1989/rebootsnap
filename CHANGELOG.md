# 变更记录

本文记录 RebootSnap 的重要设计、文档和实现变更。目标是让人类读者可以快速理解项目演进，也让 AI 工具可以稳定解析变更意图、范围和影响。

提交信息和变更记录格式的唯一规范来源见 [变更管理规范](docs/change-management.md)。

## 2026-05-17 - FsRoots 路径抽象：为 mock 测试引入文件系统路径重定向

- **类型**：实现
- **范围**：`src/fs.rs`、`src/collector/mod.rs`、全部 21 个 collector、`src/main.rs`
- **提交信息**：`feat(fs): add FsRoots path indirection for mock testing support`

### 变更内容

- 新增 `src/fs.rs`：`FsRoots` struct，包含 `proc`、`sys`、`dev`、`etc`、`run` 五个 `PathBuf` 字段，以及 `resolve(path) -> PathBuf` 前缀映射方法
- `ProbeOutcome` 新增 `roots: FsRoots` 字段，生产环境为 `FsRoots::default()`（映射到真实 `/proc`、`/sys`、`/dev` 等）
- `probe_files()` 使用 `roots.exists(f)` 替代 `Path::new(f).exists()`，支持 mock 重定向
- `CollectionTask::probe()` 签名新增 `roots: &FsRoots` 参数
- 所有 21 个 collector 的 `collect()` 内部所有 `std::fs` 路径调用通过 `probe.roots.resolve(...)` 包装

### 设计影响

- 生产行为零变化：`FsRoots::default()` 将 `/proc/...` 映射回真实 `/proc/...`
- 测试可注入 mock `FsRoots`（指向 `tempfile::TempDir`），实现全部 21 个 collector 的 fixture 测试
- procfs crate（`process.rs`、`memory.rs`）不受 FsRoots 控制，mock 测试需单独处理

### 验证

- `cargo build --release` 编译通过（零 warning），`cargo clippy` 零 warning
- `scripts/verify.sh` exit 0
- `cargo run --release -- /tmp` 生成 22 文件 tar.gz，输出正确

## 2026-05-17 - 第三轮审计修复：文档 precision、dmesg 自描述、分隔符统一

- **类型**：修复 / 文档
- **范围**：`src/collector/events.rs`、`docs/decisions/0004-output-format.md`、`docs/collector-architecture.md`、`CHANGELOG.md`
- **提交信息**：`fix(collector): add collection marker to dmesg output, fix doc precision`

### 变更内容

- `events.rs`：增加 `EventsMarker` record struct，dmesg 输出前置 `{"collection":"RT-20","source":"/dev/kmsg"}\n` 自描述 JSON 行
- `events.rs`：错误消息去除硬编码 `(permission denied)`，改为通用 `unreadable`
- ADR 0004 AI 消费路径文本：`sockets.jsonl`→`sockets.json`、`fds.jsonl`→`fds.json`
- `architecture.md` RT-02：路径从模糊 `/proc/sys/` 改为完整 `/proc/sys/kernel/ostype` 等精确路径
- `CHANGELOG.md`：移除全部残留 `---` 分隔符，条目间统一使用空行分隔

### 设计影响

- dmesg.txt 现在与所有其他 collector 输出一致，携带 `collection` 自描述字段
- RT 映射表所有路径均为可复现的精确路径

### 验证

- `cargo build --release` 编译通过（零 warning），`cargo clippy` 零 warning
- `scripts/verify.sh` exit 0
- dmesg.txt 首行为 `{"collection":"RT-20","source":"/dev/kmsg"}`（非 root 环境仅含标记行，预期行为）

## 2026-05-17 - 第二轮审计修复：probe 降级传播、架构表同步

- **类型**：修复 / 文档
- **范围**：`src/collector/process.rs`、`docs/collector-architecture.md`
- **提交信息**：`fix(collector): merge probe degradation in process, sync architecture RT table`

### 变更内容

- `process.rs`：最终状态合并 `probe.degraded`（probe 阶段标记的降级不再被静默丢弃）
- `docs/collector-architecture.md` RT 映射表 6 处同步：
  - RT-02：移除误标的 `+ procfs`（kernel.rs 仅用 std::fs）
  - RT-06：补标 `+ procfs`（memory.rs 使用 `procfs::Meminfo`）
  - RT-11：补漏 `/proc/net/ipv6_route`
  - RT-12：补漏 `/proc/net/raw6`、`/proc/net/snmp6`
  - RT-13：补漏 `/proc/sys/net/nf_conntrack_max`
  - RT-20：补漏 `/proc/sys/kernel/printk`

### 设计影响

- RT 映射表现在与实际代码实现完全一致，可作为接口权威参考
- 所有 21 个 collector 的 probe 降级信息均正确传播到最终 CollectionOutcome

### 验证

- `cargo build --release` 编译通过（零 warning），`cargo clippy` 零 warning
- `scripts/verify.sh` exit 0
- `cargo run --release -- /tmp` 生成 22 文件 tar.gz

## 2026-05-17 - 多维度审计修复：ADR 对齐、依赖清理、代码规范化

- **类型**：修复 / 文档 / 工程化
- **范围**：`src/collector/`、`Cargo.toml`、`docs/decisions/0003-*`、`docs/decisions/0004-*`、`docs/collector-architecture.md`、`CHANGELOG.md`
- **提交信息**：`fix(collector): align ADR 0004 filenames, remove unused deps, fix code drift`

### 变更内容

- ADR 0004 文件名对齐：`fd.json`→`fds.json`、`mount.json`→`mounts.json`、`session.json`→`sessions.json`、`device.json`→`devices.json`、`cache.json`→`caches.json`
- RT-20 `events.json`→`dmesg.txt`，改为 `text_writer` 输出原始内核环形缓冲区
- `events.rs`：`/dev/kmsg` 加入 probe，不可读时标记 `Degraded`
- `netdev.rs`：`/proc/net/route` 等 4 个文件加入 probe，`raw()`→`read_raw()`
- `tmpfs.rs`：`count_dir` 从模块级移至 `collect()` 局部作用域
- `systemd.rs`：合并 `probe.degraded` 到最终状态
- Fd/Mount/Socket 逐项超时从 2s/5s 改为 10s（与架构文档一致）
- `Cargo.toml`：移除 3 个未使用依赖 `nix`、`rtnetlink`、`neli`（净减少编译时间）
- ADR 0003：依赖表拆分为「当前生效」和「暂缓」两组，标注替代方案
- ADR 0004：文件清单同步为实际输出名称
- `docs/collector-architecture.md`：RT 表接口/crate/格式列同步为实际实现；代码示例更新为 21 variant
- `docs/phase-a-review.md`：添加「历史性审核文档」标题注释
- `CHANGELOG.md`：统一条目间分隔符（移除孤立 `---`）

### 设计影响

- 所有输出文件名现在与 ADR 0004 完全一致，工具消费者可按固定名称索引
- `Cargo.toml` 中无任何未使用依赖，依赖声明与代码实际引用互相验证可自动化
- `CollectionTask` 超时表与架构文档保持一致：常规 2s、量大 10s（RT-04/06/07/09/11/12）、systemd/netfilter 5s
- 代码风格规范化：所有 collector 辅助函数均在 `collect()` 局部作用域内定义

### 验证

- `cargo build --release` 编译通过（零 warning），依赖树精简（−3 unused crates）
- `cargo clippy` 零 warning
- `scripts/verify.sh` exit 0
- `cargo run --release -- /tmp` 生成 22 文件 tar.gz（RT-20 degraded 于非 root 环境，预期行为）
- manifest.json 中 RT-20 正确标记 `degraded` + `missing: ["/dev/kmsg"]`

## 2026-05-17 - Phase C+D：实现全部 17 个采集器，完成 RT-01 至 RT-21 全覆盖

- **类型**：实现
- **范围**：`src/collector/` （17 个新文件）、`src/collector/mod.rs`、`src/main.rs`、`Cargo.toml`
- **提交信息**：`feat(collector): implement Phase C+D all 17 remaining collectors`

### 变更内容

- Phase C（netlink 通路）：新增 `socket.rs`（RT-12）、`netdev.rs`（RT-11，sysfs 回退）、`netfilter.rs`（RT-13）
- Phase D（批量收尾）：新增 13 个 collector — `block.rs`（RT-10）、`cache.rs`（RT-21）、`device.rs`（RT-17）、`events.rs`（RT-20）、`fd.rs`（RT-07）、`ipc_ns_cg.rs`（RT-14）、`kernel.rs`（RT-02）、`mount.rs`（RT-09）、`power.rs`（RT-18）、`security.rs`（RT-16）、`session.rs`（RT-15）、`time.rs`（RT-19）、`tmpfs.rs`（RT-08）
- `memory.rs`：使用 `procfs::Meminfo::current()` 类型安全提取 MemTotal / MemAvailable
- `mod.rs`：`CollectionTask` enum 扩展至 21 变体，新增 `collect()` 方法实现 unified dispatch
- `main.rs`：重构为循环驱动模式（`tasks.into_iter().enumerate()`），消除 20 个独立 spawn block
- `main.rs`：manifest probe 改为 `HashMap` 动态查找，不再硬编码 p0..p7 索引
- `Cargo.toml`：移除未使用的直接 `futures` 和 `netlink-packet-route` 依赖
- `events.rs`：`/dev/kmsg` 直读替代 nix klogctl（避免 feature flag 复杂度）

### 设计影响

- RT-01 至 RT-21 全部 21 个大类已实现，每个 collector 遵循统一的 probe→collect 模式
- `all_tasks()` 返回 21 元素数组，新增 collector 只需追加元素和匹配分支
- 主循环采用 `into_iter()` 消费式遍历 + `JoinSet` 并发，probe 结果通过 `Vec<(&str, ProbeOutcome)>` 索引匹配
- RT-11（netdev）回退为 sysfs 读取（`/sys/class/net/*`），因 rtnetlink 0.21 与 netlink-packet-route 0.30 API 变更较大
- 架构文档中 Phase C/D 已全部实现，Phase E（测试）为下一阶段

### 验证

- `cargo build --release` 编译通过（零 warning），`cargo clippy` 零 warning
- `cargo run --release -- /tmp` 生成 23 文件 tar.gz（21 collector + manifest + summary）
- `scripts/verify.sh` exit 0
- 输出大小 72 KiB（tar.gz），全链路 runtime 验证通过

## 2026-05-17 - Phase B：实现 RT-03 systemd D-Bus 采集器

- **类型**：实现
- **范围**：`src/collector/systemd.rs`、`src/collector/mod.rs`、`src/main.rs`、`src/collector/process.rs`、`src/output.rs`、`docs/phase-a-review.md`
- **提交信息**：`feat(collector): implement RT-03 systemd Phase B D-Bus collector`

### 变更内容

- 新增 `src/collector/systemd.rs`，通过 `zbus` 连接 D-Bus system bus 调用 `org.freedesktop.systemd1.Manager` 接口。
- 采集 `ListUnits()` 全部 unit 的 name/description/load/active/sub 状态。
- 采集 `ListJobs()` 当前待处理 job（id/unit/type/state）。
- 采集 `ListInhibitors()` 活动 inhibitor lock（what/who/why/mode/uid/pid）。
- 采集 Manager 属性：Version、Architecture、Features、Virtualization。
- probe 阶段通过 D-Bus `ListNames` 检查 `org.freedesktop.systemd1` 是否存在，确认 systemd 环境。
- 输出为 `systemd.json`（JSON 格式，单文件）。
- 在 `CollectionTask` enum 中注册 `Systemd` 变体，`all_tasks()` 中新增对应条目。
- Systemd 逐项超时设为 5 秒（D-Bus 调用较 procfs 偏慢，非 systemd 环境快速降级）。
- Manifest probe 中 `systemd` 字段从硬编码 `unavailable` 改为反映实际 probe 结果。
- 修复 7 个 clippy 警告（`process.rs` 冗余 cast、`systemd.rs` `map_or`→`is_ok_and`、`output.rs` 可折叠 `if`/`is_some_and`/`Error::other`）。
- 修复 `cleanup_tmp()` 遗留文件检测模式：`extension() == "tmp"` 不匹配 tempfile crate 的 `.tmpXXXXXX` 命名，改用 `file_name().starts_with(".tmp")`。
- `systemd.rs` 字段 `virtualisation` 改为 `virtualization`，与 systemd D-Bus 属性名一致。
- `docs/phase-a-review.md` 添加 Phase B 更新脚注，标注 RT-03 已实现、`ProbeSummary` 已重构移除。

### 设计影响

- RT-03 全部 14 个三级子项均为 `必采`，`ListUnits`/`ListJobs`/`ListInhibitors` 覆盖了 unit 状态、job 队列和 inhibitor 三大类。
- Systemd 是按技术风险递增顺序实现的 Phase B 第一个 collector，验证了 `zbus` 异步 D-Bus 链路的可行性。
- 个别 D-Bus 调用失败（如 `ListInhibitors` 因非 root 权限被拒绝）时，采集器标记 `Degraded` 并继续输出已获取数据，不阻止整体采集。
- 非 systemd 环境（如容器、WSL）在 probe 阶段返回 `available: false`，采集安全跳过。

### 验证

- `cargo build --release` 编译通过（零 warning），`cargo clippy` 零 warning。
- `cargo run --release -- /tmp` 运行成功，生成 7 文件 tar.gz（含 `systemd.json` 80 KiB、573 unit、0 job、0 inhibitor）。
- `systemd.json` 包含 Manager 版本 259.5-0ubuntu3、x86-64 架构和 features 字符串。
- Manifest probe `systemd` 字段为 `available`（probe 确认 systemd 在总线）。
- `scripts/verify.sh` exit 0。

## 2026-05-17 - 审核修复：接入 error type、补齐校验覆盖、消除主循环重复

- **类型**：修复 / 重构
- **范围**：`src/error.rs`、`src/output.rs`、`src/main.rs`、全部 `src/collector/*.rs`、`src/collector/mod.rs`、`Cargo.lock`、`docs/decisions/0003-implementation-tech-stack.md`、`docs/collector-architecture.md`、`docs/change-management.md`、`scripts/verify.sh`、`scripts/validate-change-management.sh`、`README.md`、`CHANGELOG.md`
- **提交信息**：`fix(governance): wire error types, DRY main, fix verify coverage`

### 变更内容

- 移除 `OutputError` 和 `ArchiveError` 的 `#[allow(dead_code)]`，接入 `output.rs`（全部 `Result<T, String>` → `Result<T, OutputError>`）和 `main.rs` 的 `create_tar_gz`（→ `Result<(), ArchiveError>`）。
- 四个 collector 的 `collect()` 中捕获实际 error（`e.to_string()`）替代硬编码失败原因。
- 新增 `OutputDir::from_existing()` 工厂方法，`main.rs` 中 4 处 `OutputDir { root }` 直接构造替换为 `from_existing()`。
- `main.rs` 提取 `all_tasks()` 函数统一注册 4 个 `CollectionTask`，消除 task_infos 与 probe 循环之间的手动重复声明。
- 移除 `ProbeOutcome`、`CollectionStatus` 上多余的 `#[allow(dead_code)]`（已被 4 个 collector 和 `main.rs` 使用）。
- `output::SIZE_LIMIT` 改为 `pub`，`process.rs` 截断原因判断从魔术数字 `64*1024*1024` 改为引用该常量。
- 重建 `Cargo.lock`，消除已移除 `async-trait` 的直接依赖残留。
- ADR 0003 crate 表格补齐 `tokio` 行。
- `docs/collector-architecture.md`：修正 `create()` 参数名 `root`→`base_dir` 和返回值类型 `Result<Self, OutputError>`；补充 `from_existing()` 文档。
- `memory.rs` 硬编码文件路径改为引用 `FILES` 数组索引，与 `boot.rs`/`cpu.rs` 模式一致。
- `main.rs` 中 `try_into().unwrap()` 改为 `match` 显式处理 probe 数量变化。
- `output.rs` 中 `expect()` 改为 `match` 返回 `OutputError`，消除 panic 路径。
- `docs/change-management.md` 和 `scripts/validate-change-management.sh` 中提交信息 regex 修正：字符类 `[A-Za-z0-9)]` 中误入的 `)` 移除。

### 设计影响

- 不改变现有设计约束。`OutputError`/`ArchiveError` 接入是对 Phase A 审核中 `thiserror` 落地路径的补全，不引入新的 API、配置项或外部依赖。
- `OutputDir::from_existing()` 是语义澄清而非行为变更：调用者必须已通过 `create()` 确保目录存在。
- `all_tasks()` 函数使新增 collector 时只需修改一处数组。

### 验证

- `cargo build` 编译通过（零 warning）。
- `cargo run --release -- /tmp` 运行成功，tar.gz 归档正确。
- `scripts/verify.sh` exit 0。

## 2026-05-17 - 审核修复：probe 结果落地、文档漂移消除

- **类型**：修复
- **范围**：全部 `src/collector/*.rs`、`docs/collector-architecture.md`、`docs/decisions/0003-*.md`、`docs/decisions/0004-*.md`、`CHANGELOG.md`
- **提交信息**：`fix(collector): use probe outcome in collect, fix doc drift`

### 变更内容

- 所有 collector 的 `collect()` 现在检查 probe 结果：`!probe.available` 时立即返回 Failed；`probe.degraded` 非空时标记 Degraded。
- process.rs 修复错误处理：`write_line` 错误不再覆盖 `items_total`，正确区分截断/写入错误。
- 提取公共 `probe_files()` 辅助函数，消除三份重复的 probe 逻辑。
- 统一 boot.rs probe 深度：从"读文件检查可读性"改为与 cpu.rs/memory.rs 一致的"仅检查文件存在"。
- 修复 `docs/collector-architecture.md` 6 处漂移：`ProbeResult`→`ProbeOutcome`、`required` 概念消除、`async fn json_writer`→同步、`{filename}.tmp`→`NamedTempFile`、Phase A 文件清单、`Collector` trait→`CollectionTask` enum。
- 修复 `docs/decisions/0003-*.md` 平台限定要求与实际 `[dependencies]` 段一致。
- 修复 `docs/decisions/0004-*.md` tar 命令行引用改为 crate 描述。

### 设计影响

- `collect()` 必须检查 probe 结果并据此决定提前返回或标记 Degraded；`_probe` 前缀不可再使用。
- 新增 collector 时，probe 阶段应使用 `probe_files()` 辅助函数，或手动构造 `ProbeOutcome` 并保证 `available`/`degraded` 语义正确。
- 临时文件清理逻辑依赖 `NamedTempFile::Drop` 自动删除；`cleanup_tmp()` 仅作为 SIGKILL 遗留文件清理的兜底保障。

### 验证

- `cargo build` 编译通过（1 条预期 dead_code 警告）。
- `cargo run --release -- /tmp` 运行成功，输出与修复前一致。
- `scripts/verify.sh` exit 0。

## 2026-05-17 - Phase A 重构：enum dispatch、社区 crate 与 Rust 惯用法对齐

- **类型**：重构
- **范围**：全部 `src/`、`Cargo.toml`、`docs/decisions/0003-implementation-tech-stack.md`、`docs/collector-architecture.md`、`docs/phase-a-review.md`、`CHANGELOG.md`
- **提交信息**：`refactor(collector): enum dispatch, community crates, idiomatic Rust`

### 变更内容

- 移除 `async-trait` crate 依赖，用 `CollectionTask` enum 替代 `Box<dyn Collector>` trait object 动态分发。
- 新增 `thiserror`、`tracing`、`tracing-subscriber`、`tempfile` crate 依赖，补录 `chrono` 到 ADR 0003 crate 表。
- 用 `tempfile::NamedTempFile` 替代手写 temp+rename，消除 TOCTOU race 和 panic 安全问题。
- 用 `tracing` + `tracing-subscriber::fmt` 替代全部 `eprintln!` / `println!` 日志调用。
- 创建 `src/error.rs`，定义 `CollectError`、`OutputError`、`ArchiveError`（`thiserror` 派生，为后续 `?` 传播铺设基础）。
- `CollectionOutcome` 新增 `mem_total_kb`、`mem_available_kb`、`hostname`、`kernel_version`、`boot_id`、`uptime_seconds` 字段，避免 `main.rs` 从磁盘重读已采集数据。
- 所有 naming 去除 `-Collector` / `-Collect` 后缀（`Boot`、`Cpu`、`Memory`、`Process`）。
- `ProbeResult` → `ProbeOutcome`、`CollectResult` → `CollectionOutcome`。
- 消除魔术 sentinel 值：`pid: i32`（-1 sentinel）→ `pid: Option<i32>`，`name: String`（"?" sentinel）→ `name: Option<String>`。
- 删除 unit struct 的空 `new()` 构造函数。
- 删除 `CollectionOutcome` 的冗余 `error_reason: Option<String>` 字段。
- `memory.rs` 手写 `parse_meminfo` 删除，改用独立解析但保留类型化输出。
- `process.rs` `proc_entry` match 替换为 `procfs::process::all_processes().flatten().collect()`。
- 更新 `docs/decisions/0003-implementation-tech-stack.md` crate 表。
- 更新 `docs/collector-architecture.md` trait 定义模块为 enum 方案。

### 设计影响

- 新增 collector 时需在 `CollectionTask` enum 中添加变体，并在 `id()`/`filename()`/`item_timeout()`/`probe()` match 中添加分支。编译器强制穷尽。
- 所有临时文件通过 `NamedTempFile::new_in(output_dir)` 创建，`persist()` 原子 rename；drop 时自动删除未 persist 的临时文件。
- 所有日志通过 `tracing` 输出，`main()` 入口调用 `tracing_subscriber::fmt::init()` 初始化。
- `CollectionOutcome` 携带 summary 构建所需的提取值；manifest host 字段从 `Boot` outcome 提取，summary 内存值从 `Memory` outcome 提取。

### 验证

- `cargo build --release` 编译通过（1 条预期 dead_code 警告：`JsonlWriter::byte_count`）。
- `cargo run --release -- /tmp` 运行成功，tar.gz 归档正确，输出文件数与 Phase A 重构前一致。
- `scripts/verify.sh` exit 0。
- 人工审核 `manifest.json`、`summary.json` 字段与重构前一致（除 `pid`/`name` sentinel 值消失和 `collection` 字段改用 `&'static str`）。

## 2026-05-15 - 实现 Phase A：框架、输出模块和四个基础采集器

- **类型**：实现
- **范围**：`Cargo.toml`、`src/main.rs`、`src/collector/mod.rs`、`src/collector/boot.rs`、`src/collector/cpu.rs`、`src/collector/memory.rs`、`src/collector/process.rs`、`src/output.rs`
- **提交信息**：`feat(collector): implement Phase A framework, output module, and four base collectors`

### 变更内容

- 创建 Cargo 项目，引入 `tokio`、`serde`、`serde_json`、`async-trait`、`procfs`、`nix`、`zbus`、`rtnetlink`、`netlink-packet-route`、`neli`、`flate2`、`tar`、`chrono` 依赖。
- 实现 `Collector` async trait（`ProbeResult`/`CollectResult` 类型、`probe`/`collect` 方法、`item_timeout` 逐项超时默认值）。
- 实现 `OutputDir` + `JsonWriter`/`JsonlWriter`/`TextWriter` 输出模块（临时文件 + 原子 rename、64 MiB 大小截断上限、50000 条目截断上限）。
- 实现 RT-01 启动身份采集器（boot_id/uptime/kernel/cmdline/hostname）。
- 实现 RT-04 进程与线程采集器（全进程枚举，JSONL 输出，procfs 解析 pid/ppid/name/state/uid/threads/cmdline）。
- 实现 RT-05 CPU 与调度采集器（/proc/stat/loadavg/pressure/ interrupts/softirqs 原始内容）。
- 实现 RT-06 内存采集器（meminfo 结构化解析 + vmstat/zoneinfo 原始内容）。
- 实现 `main.rs` 编排逻辑：串行 probe → 并发 collect → manifest.json → summary.json → tar.gz 打包 → 清理源目录。
- release profile 启用 LTO + single codegen unit + opt-level=s + strip。

### 设计影响

- `Collector` trait 的 `collect()` 接收 `probe: &ProbeResult` 以避免重复探测。
- 所有 Writer 使用 `{filename}.tmp` → atomic rename 模式，超时取消不会产生半截文件。
- 输出归档为 `rebootsnap-{timestamp}.tar.gz`（目录内文件加目录前缀）。
- 所有硬编码参数（超时、大小上限、条目上限）来自 `docs/collector-security-governance.md`。

### 验证

- `cargo build --release` 编译通过（10 条预期 dead_code 警告，对应 Phase B/C/D 预留类型）。
- `cargo run --release -- /tmp` 运行成功，生成 tar.gz 归档，包含 manifest.json、summary.json 和四个数据文件。
- 人工审核 manifest.json 结构符合 ADR 0004，summary.json 包含正向指标（主机名、uptime、进程数、内存、loadavg），进程 JSONL 格式正确（含 `collection` 自描述字段）。
- 人工审核 tar 归档文件加目录前缀，提取时产生独立子目录。

## 2026-05-15 - 固定 collector 实现技术栈、输出格式和总体架构

- **类型**：设计 / 文档
- **范围**：`docs/decisions/0003-implementation-tech-stack.md`、`docs/decisions/0004-output-format.md`、`docs/collector-architecture.md`、`docs/decisions/README.md`、`docs/index.md`、`docs/project-map.yml`、`scripts/verify.sh`、`README.md`、`CHANGELOG.md`
- **提交信息**：`design(collector): define tech stack, output format and architecture`

### 变更内容

- 新增 `docs/decisions/0003-implementation-tech-stack.md`，固定 Rust 语言、tokio 多线程异步 runtime、零配置原则、`x86_64-unknown-linux-musl` 静态单二进制分发和全部 crate 依赖选型（含 `procfs`、`zbus`、`rtnetlink`、`conntrack`、`neli`、`nix`、`serde`、`serde_json`、`flate2`、`tar`、`async-trait`）。
- 新增 `docs/decisions/0004-output-format.md`，固定目录式输出结构、`manifest.json` + `summary.json` 双导航文件、JSON/JSONL/.txt 三种文件格式、JSON 截断策略（buffer-then-commit）、自描述数据对象、截断标注约定和多文件 item 约定。
- 新增 `docs/collector-architecture.md`，定义模块结构、Collector trait（含 `ProbeResult` 注入 `collect()`、`CollectResult` 完整字段、逐项超时 `item_timeout()`）、多线程并发模型、临时文件原子 rename 写入、五个实现阶段和 RT 大类到 OS 接口与 crate 的完整映射。
- 上一轮提交后全角度审核发现 5 个设计级问题和 7 个文档级问题，本轮全部修复（crate 表补全、probe-collect 耦合、JSON 截断语义、超时取消安全、summary 截断标注、措辞澄清）。
- 更新 `docs/decisions/README.md`、`docs/index.md`、`docs/project-map.yml`、`scripts/verify.sh` 和 `README.md`，将三篇新文档接入稳定入口和自动校验。

### 设计影响

- collector 实现必须使用 Rust + tokio 多线程，不接受其他语言或运行时。
- collector 不接受配置参数；所有参数均为硬编码常量或动态自适应。
- 输出格式固定为本文定义的目录结构、文件命名和自描述约定；下游工具按 `manifest.json` 索引遍历文件。
- 实现顺序固定为五个阶段：procfs 通路 → systemd 通路 → netlink 通路 → 批量收尾 → 测试体系。
- 新增第三方 crate 依赖前必须确认该 crate 为纯 Rust 且不引入运行期 C 库依赖。

### 验证

- 运行 `scripts/verify.sh` 校验治理入口、文档导航、项目地图、Markdown 链接、变更记录结构和 whitespace，确认所有新增文件入口和导航一致。
- 人工审核三篇新文档与 ADR 0001、ADR 0002、采集安全治理和测试治理无矛盾。
- 人工确认全部选定 crate 均为纯 Rust 且活跃维护，唯一例外是 nftables 需用 `neli` 构建 netlink 消息（因 `netlink-packet-netfilter` 已废弃约 3 年）。
- 人工审核输出格式满足人类（grep 直接看）、工具（按索引遍历）和 AI（manifest → summary → 定向钻入）三种消费路径。

## 2026-05-15 - 固定默认采集策略

- **类型**：设计 / 文档
- **范围**：`docs/linux-runtime-info-collection-decision.md`、`docs/index.md`、`CHANGELOG.md`
- **提交信息**：`design(runtime-taxonomy): define default collection policy`

### 变更内容

- 在最终采集决策文档中固定默认采集约束，明确只读、有界执行、低资源占用、高证据密度、渐进采集和敏感最小化原则。
- 为 `RT-01` 到 `RT-21` 填入首轮默认采集决策，区分 `必采`、`选采` 和 `暂缓`。
- 将采集成本高、易产生大量输出、可能阻塞或默认复盘价值不足的项目排除在默认最小证据包之外。
- 更新 `docs/index.md`，将最终采集决策文档状态从框架提议改为生效设计入口。

### 设计影响

- 默认 collector 必须面向主机不稳定、资源紧张和重启窗口有限的场景设计，不能为了追求覆盖面牺牲稳定性和有界执行。
- 后续 collector 实现必须先实现 `必采` 最小证据包；`选采` 项必须受能力探测、触发条件、剩余预算和显式配置约束；`暂缓` 项不得进入默认实现。
- 后续不得默认采集环境变量值、密钥 payload、认证材料内容、应用业务 payload、普通文件内容或用户隐私数据。

### 验证

- 运行 `scripts/verify.sh` 校验治理入口、文档导航、项目地图、Markdown 链接、变更记录结构和 whitespace。
- 人工复核首轮决策符合低资源、低扰动、限时完成和故障复盘价值优先的设计要求。

## 2026-05-15 - 对齐三级候选项依据边界

- **类型**：修复 / 设计 / 文档
- **范围**：`docs/linux-runtime-info-categories.md`、`docs/linux-runtime-info-subcategories.md`、`docs/linux-runtime-info-collection-items.md`、`CHANGELOG.md`
- **提交信息**：`fix(runtime-taxonomy): align collection item evidence`

### 变更内容

- 从 `RT-01-04-02` 描述中移除 `machine-id`，避免与一级边界中 `machine-id` 为持久化事实来源的排除规则冲突。
- 为 chrony、cron/cronie 和 GNU C Library NSS/nscd 补充上游入口与依据组，并将对应三级项挂接到这些依据组。
- 将系统守护进程内存队列和派生缓存相关项收窄为有明确 OS 子系统语义或已纳入依据组的守护进程状态。

### 设计影响

- 保留现有 `RT-xx`、`RT-xx-yy` 和 `RT-xx-yy-zz` ID，不新增、不删除、不重编号稳定分类。
- 后续最终采集决策不能把任意 daemon 内部队列或缓存自动视为 Linux OS 运行时信息；cron 非 Cronie 实现必须补充对应来源。

### 验证

- 运行 `scripts/verify.sh` 校验治理入口、文档导航、项目地图、Markdown 链接、变更记录结构和 whitespace。
- 人工复核三级候选项与一级排除边界、二级分类名称和依据组挂接关系一致。

## 2026-05-15 - 建立最终采集决策框架

- **类型**：文档 / 设计 / 测试
- **范围**：`docs/linux-runtime-info-collection-decision.md`、`README.md`、`docs/index.md`、`docs/project-map.yml`、`scripts/verify.sh`、`CHANGELOG.md`
- **提交信息**：`docs(runtime-taxonomy): scaffold collection decision framework`

### 变更内容

- 新增 `docs/linux-runtime-info-collection-decision.md`，作为后续从三级候选采集项筛选最终采集范围的决策文档框架。
- 固定最终采集决策的输入基线、状态枚举、决策维度、表格模板、一级大类填充入口和完整性检查规则。
- 当前只建立框架，不填入具体 `RT-xx-yy-zz` 采集项决策。
- 更新 `README.md`、`docs/index.md`、`docs/project-map.yml` 和 `scripts/verify.sh`，将框架文档接入稳定入口和自动校验。

### 设计影响

- 后续最终采集范围必须从 `docs/linux-runtime-info-collection-items.md` 中筛选，不得绕过三级候选采集项直接新增最终采集项。
- 后续具体决策必须使用固定状态：`必采`、`选采`、`暂缓`、`禁止采集`、`待定`。
- 保留为 `待定` 的项目不得进入 collector 实现，避免框架占位被误用为实现授权。

### 验证

- 运行 `scripts/verify.sh` 校验新增框架文档入口、项目地图、Markdown 链接、脚本权限、变更记录结构和 whitespace。
- 人工审核框架只定义结构和填充规则，未填入具体采集项决策。

## 2026-05-15 - 建立三级候选采集项基线

- **类型**：设计 / 文档 / 测试
- **范围**：`docs/linux-runtime-info-collection-items.md`、`docs/linux-runtime-info-subcategories.md`、`docs/glossary.md`、`README.md`、`docs/index.md`、`docs/project-map.yml`、`scripts/verify.sh`、`CHANGELOG.md`
- **提交信息**：`design(runtime-taxonomy): add collection item baseline`

### 变更内容

- 新增 `docs/linux-runtime-info-collection-items.md`，在 21 个一级大类和 138 个二级分类之下建立 `RT-xx-yy-zz` 三级候选采集项组。
- 为每个二级分类定义 2 个三级候选采集项组，共 276 个候选采集项组。
- 在三级项文档中新增二次确认依据组，让每类候选采集项同时指向官方文档入口和源码确认路径。
- 更新 `docs/linux-runtime-info-subcategories.md`，让二级分类文档指向三级候选采集项文档，并明确最终采集范围应从三级项筛选。
- 更新 `docs/glossary.md`，新增“三级分类”稳定术语。
- 更新 `README.md`、`docs/index.md`、`docs/project-map.yml` 和 `scripts/verify.sh`，将三级候选采集项文档接入稳定入口和自动校验。

### 设计影响

- `RT-xx-yy-zz` 成为后续最终采集决策的候选项基线；最终采集文档必须从这些三级项选择、合并或推迟。
- 本文仍不固定采集命令、字段结构、权限模型、脱敏规则、存储格式或报告布局；这些内容必须在后续 collector 设计和最终采集决策中定义。
- 后续如果发现必须新增三级候选采集项，应先修改三级分类基线，再更新最终采集决策，避免绕过稳定分类结构。

### 验证

- 运行 `scripts/verify.sh` 校验新增文档入口、项目地图、Markdown 链接、脚本权限、变更记录结构和 whitespace。
- 人工对照 `docs/linux-runtime-info-subcategories.md`，确认三级文档覆盖全部 21 个一级大类和 138 个二级分类。
- 人工核对每个三级候选采集项组均带有官方文档和源码二次确认依据组。

## 2026-05-15 - 固定治理基线决策记录

- **类型**：设计 / 文档 / 工程化
- **范围**：`docs/decisions/0001-runtime-info-boundary.md`、`docs/decisions/0002-human-ai-governance.md`、`docs/decisions/README.md`、`README.md`、`docs/index.md`、`docs/project-map.yml`、`docs/project-governance.md`、`docs/change-management.md`、`scripts/verify.sh`、`CHANGELOG.md`
- **提交信息**：`design(governance): record baseline decisions`

### 变更内容

- 新增 `docs/decisions/0001-runtime-info-boundary.md`，记录 Linux OS 运行时信息边界、分类 ID 稳定性和上层对象排除取舍。
- 新增 `docs/decisions/0002-human-ai-governance.md`，记录人与 AI 共治执行模型、远端保护阶段安排和采集实现前的安全与测试治理门槛。
- 更新 `docs/decisions/README.md`，将两篇决策记录接入决策目录。
- 更新 `README.md`、`docs/index.md` 和 `docs/project-map.yml`，将新增决策记录接入人类导航和机器地图。
- 更新 `docs/project-governance.md` 和 `docs/change-management.md`，明确远端分支保护在 `0.1.0` 发布后启用，当前快速推进阶段不作为阻塞项。
- 更新 `scripts/verify.sh`，将新增决策记录纳入必需文件和项目地图一致性校验。

### 设计影响

- 后续采集项、数据模型和报告视图必须优先挂靠到既有 `RT-xx-yy` 二级分类；新增或重编号稳定分类 ID 仍需人类确认。
- 后续 AI 代理应把远端保护状态表述为 `0.1.0` 发布后启用的待确认治理项，而不是当前阶段的本地阻塞项。
- 首个会执行主机采集动作的 collector 实现前，必须先补充采集安全治理和测试治理，覆盖只读采集原则、敏感信息边界、脱敏策略、发行版兼容范围和可复现测试样本。

### 验证

- 运行 `scripts/verify.sh` 校验新增决策记录入口、文档导航、项目地图、Markdown 链接、脚本权限、变更记录结构和 whitespace。

## 2026-05-14 - 收紧 AI 治理执行闭环

- **类型**：工程化 / 文档 / 测试
- **范围**：`AGENTS.md`、`docs/project-governance.md`、`docs/change-management.md`、`docs/decisions/README.md`、`scripts/validate-change-management.sh`、`scripts/verify.sh`、`scripts/setup-dev.sh`、`.githooks/commit-msg`、`CHANGELOG.md`
- **提交信息**：`chore(governance): tighten AI governance workflow`

### 变更内容

- 更新 `AGENTS.md`，新增 AI 执行闭环，要求 AI 在修改前判断人类确认边界，修改后区分自动验证、人工审核和无法本地确认的远端治理状态。
- 更新 `docs/project-governance.md`，明确脚本通过不等同于语义正确，事实准确性、设计取舍和远端仓库设置仍需按来源显式审核。
- 更新 `docs/change-management.md`，说明本地 hook、`scripts/verify.sh`、暂存区提交校验和 CI 的各自自动化覆盖范围。
- 更新 `.githooks/commit-msg`，让本地提交时先运行 `scripts/verify.sh`，再校验暂存区提交信息与变更记录匹配关系。
- 更新 `scripts/validate-change-management.sh`，对 `CHANGELOG.md` 的全部历史条目执行结构、固定字段、固定小节、类型值和提交信息格式校验；新增 `export LC_ALL=C` 避免 locale 影响字符类匹配。
- 更新 `scripts/verify.sh`，新增 `export LC_ALL=C` 确保校验行为跨 locale 一致。
- 更新 `docs/decisions/README.md`，让当前基线设计同时指向大类和二级分类文档。

### 设计影响

- 后续 AI 代理必须按“加载上下文、判断确认边界、同步稳定入口、运行验证、说明审核依据”的闭环执行任务。
- 变更管理的自动化边界被明确为结构和一致性校验；设计语义、事实来源和远端分支保护状态必须单独说明，不能由脚本通过代替。
- 本地提交行为与文档声明对齐：启用 hook 后，提交会自动运行 `scripts/verify.sh` 和暂存区变更管理校验。

### 验证

- 运行 `scripts/verify.sh` 校验治理入口、文档链接、项目地图、变更记录结构和 whitespace。

## 2026-05-14 - 建立 Linux OS 运行时信息二级分类

- **类型**：设计 / 文档 / 测试
- **范围**：`docs/linux-runtime-info-subcategories.md`、`docs/linux-runtime-info-categories.md`、`README.md`、`docs/index.md`、`docs/project-map.yml`、`docs/glossary.md`、`scripts/verify.sh`、`CHANGELOG.md`
- **提交信息**：`design(runtime-taxonomy): add runtime subcategory baseline`

### 变更内容

- 新增 `docs/linux-runtime-info-subcategories.md`，在 `RT-01` 到 `RT-21` 大类下建立 `RT-xx-yy` 二级分类。
- 为每个二级分类定义一句稳定判定标准，避免提前绑定采集命令、文件路径、字段结构或报告形态。
- 在二级分类文档中补充官方资料审核依据，覆盖 Linux kernel documentation、Linux man-pages 和 systemd 官方文档。
- 收紧 tmpfs 共享内存目录项、IPC 对象、udev database、udev 事件队列、BPF 对象、生效位置、workqueue 和网络路径派生缓存的相邻边界。
- 更新 `docs/linux-runtime-info-categories.md`，让大类文档指向二级分类文档。
- 更新 `README.md`、`docs/index.md`、`docs/project-map.yml` 和 `docs/glossary.md`，将二级分类接入稳定文档入口和术语表。
- 更新 `scripts/verify.sh`，将二级分类文档纳入必需文件和项目地图一致性校验。

### 设计影响

- 后续采集项、数据模型和报告视图应优先挂靠到 `RT-xx-yy` 二级分类，不应直接绕过现有大类和二级分类创建新的上层对象模型。
- `RT-xx-yy` 一旦被后续设计或实现引用，即成为稳定 ID；删除、重命名或重编号前必须由人类维护者确认。
- 二级分类仍然是 OS 运行时信息类别，不等同于采集项；采集命令、字段结构和优先级应留到后续采集设计文档定义。
- 共享内存、udev、BPF、timer/workqueue、网络路径缓存等跨系统面对象必须按主对象归属分类，并通过交叉引用表达上下文。

### 验证

- 运行 `scripts/verify.sh` 校验新增文档入口、项目地图、Markdown 链接、脚本权限和变更管理结构。
- 人工对照 Linux kernel documentation、Linux man-pages 和 systemd 官方文档，核对二级分类的对象边界和相邻归属。

## 2026-05-14 - 建立人与 AI 共治治理基础设施

- **类型**：工程化 / 文档 / 设计
- **范围**：`AGENTS.md`、`docs/project-governance.md`、`docs/index.md`、`docs/glossary.md`、`docs/project-map.yml`、`docs/change-management.md`、`CHANGELOG.md`、`README.md`、`docs/templates/changelog-entry.md`、`docs/templates/decision-record.md`、`docs/decisions/README.md`、`scripts/setup-dev.sh`、`scripts/verify.sh`、`scripts/validate-change-management.sh`、`.githooks/commit-msg`、`.github/workflows/change-management.yml`
- **提交信息**：`chore(governance): establish human-ai governance controls`

### 变更内容

- 新增 `docs/change-management.md`，作为提交信息和变更记录格式的唯一规范来源。
- 新增 `AGENTS.md`，定义 AI 进入仓库后的必读顺序、工作规则、人类确认边界和输出要求。
- 新增 `docs/project-governance.md`，定义人与 AI 共治目标、事实来源优先级、职责边界和质量门槛。
- 新增 `docs/index.md`，作为文档地图和导航一致性规则。
- 新增 `docs/glossary.md`，固定 RebootSnap、人与 AI 共治和运行时信息相关稳定术语。
- 新增 `docs/project-map.yml`，提供机器可读的治理入口、规范入口、模板、校验命令和稳定术语。
- 新增 `docs/templates/changelog-entry.md` 和 `docs/templates/decision-record.md`，固定常用治理模板。
- 新增 `docs/decisions/README.md`，建立后续设计决策记录目录。
- 新增 `scripts/setup-dev.sh`，让新 clone 的仓库可以一条命令启用本地 Git hook。
- 将提交信息固定为 `type(scope): summary`，并定义固定 `type` 表、必填 `scope`、`summary` 长度和正文触发条件。
- 将 `CHANGELOG.md` 条目固定为日期、类型、范围、提交信息、变更内容、设计影响和验证的结构。
- 新增 `scripts/validate-change-management.sh`，用同一套脚本校验提交信息和变更记录。
- 新增 `scripts/verify.sh`，统一校验治理入口、文档导航、项目地图、Markdown 链接、脚本权限和变更管理规则。
- 新增 `.githooks/commit-msg`，在本地提交时拦截不合规提交。
- 新增 `.github/workflows/change-management.yml`，在 push 和 pull request 中重复校验治理结构、提交信息和变更记录。
- CI 对已有分支的 push 和 pull request 校验新增提交范围；对新分支首次 push 只校验当前 head，避免新规则追溯阻塞旧历史提交。
- 本地提交校验读取暂存区中的 `CHANGELOG.md`，避免工作区内容和实际提交内容不一致时绕过检查。
- 校验脚本和规范文档使用一致的提交首行正则，避免规则实现和文字规范分叉。
- 更新 `README.md` 文档入口，并让 `CHANGELOG.md` 引用规范文档而不是内嵌规则。

### 设计影响

- 后续提交和变更记录必须使用固定类型、固定字段和固定小节，便于人类审阅和 AI 解析。
- `docs/change-management.md` 成为变更管理规则的单一事实来源；`CHANGELOG.md` 只记录历史变更。
- `AGENTS.md`、`docs/project-governance.md`、`docs/index.md`、`docs/project-map.yml` 和 `scripts/verify.sh` 共同构成人与 AI 共治的稳定入口。
- 后续新增文档、稳定术语、模板或治理命令时，必须同步维护文档索引和机器可读项目地图。
- 新 clone 的本地治理初始化统一通过 `scripts/setup-dev.sh` 执行，避免依赖手工记忆 `git config core.hooksPath .githooks`。
- 本地 hook 和 CI 共同执行规范；若需要远端强制，应在代码托管平台启用分支保护并要求 `Change Management` workflow 通过。

### 验证

- 运行 `scripts/verify.sh` 校验治理入口、文档导航、项目地图、Markdown 链接、脚本权限和变更管理规则。
- 运行 `scripts/setup-dev.sh` 确认本地 Git hook 可通过统一入口配置。
- 运行 `scripts/validate-change-management.sh changelog 'chore(governance): establish human-ai governance controls'` 校验变更记录结构和提交信息匹配。
- 运行 `scripts/validate-change-management.sh commit-msg /tmp/rebootsnap-commit-msg --no-staged-check` 校验示例提交信息。
- 运行错误示例 `update things`，确认校验脚本会拒绝不符合 `type(scope): summary` 的提交信息。
- 运行提交信息和变更记录不匹配的错误示例，确认校验脚本会拒绝语义脱节的提交。
- 人工审核 `commit-msg` 模式确认本地提交校验读取暂存区 `CHANGELOG.md`。
- 人工比对规范文档和校验脚本中的提交首行正则，确认两者一致。
- 人工审核 CI 新分支首次 push 分支，确认不会因旧历史提交不符合新规范而阻塞当前治理落地。
- 运行 `git diff --check` 校验文档和脚本没有 whitespace 错误。

## 2026-05-14 - 建立 Linux OS 运行时信息大类基线

- **类型**：文档 / 设计
- **范围**：`docs/linux-runtime-info-categories.md`、`README.md`、`CHANGELOG.md`
- **提交信息**：`docs(runtime-taxonomy): establish Linux OS runtime baseline`

### 变更内容

- 新增 `docs/linux-runtime-info-categories.md`，定义 RebootSnap 所说的 Linux 服务器操作系统运行时信息。
- 将文档定位收敛为“大类定义”基石文档，只回答哪些信息属于重启前 OS 运行时现场。
- 采用 Linux 系统视角，明确排除应用、容器平台、编排平台、虚拟化管理平台和业务系统的对象模型。
- 建立 7 个层次、21 个运行时信息大类，覆盖 boot、内核、init、进程、CPU、内存、fd、VFS、块设备、网络、socket、netfilter、IPC、namespace、cgroup、安全、设备、电源健康、时间、易失缓冲和 OS 缓存等系统面。
- 为每个大类统一定义 `判定标准`、`包括`、`不包括`、`边界` 和 `重启失真`。
- 增加相邻边界规则、覆盖性校验和边界判定示例，便于后续文档与 AI 工具稳定引用。
- 更新 `README.md` 文档入口。
- 新增 `CHANGELOG.md`，初步记录变更信息和格式约束。

### 设计影响

- 后续二级分类、采集项、数据模型、权限模型、脱敏策略和报告视图应以 `RT-01` 到 `RT-21` 作为稳定上层分类。
- 后续文档不应把容器、Pod、VM、数据库实例等上层对象提升为 OS 运行时一级大类；这些对象在 Linux 层面的表现应拆回进程、namespace、cgroup、挂载、网络、设备等系统对象。
- 后续新增分类时，应优先判断是否可以归入现有 21 个大类。只有当官方 Linux OS 运行态出现无法归入现有系统面的新对象时，才应考虑新增一级大类。

### 验证

- 对照 Linux kernel 官方文档、Linux man-pages 和 systemd/freedesktop 官方文档做了覆盖性核验。
- 检查 21 个大类均具备统一结构：`判定标准`、`包括`、`不包括`、`重启失真`。
- 本次为纯文档变更，未运行代码测试。
