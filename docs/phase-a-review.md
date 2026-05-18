# Phase A 实现审核报告

> **注意：本文为历史性审核文档。** 审核对象是 Phase A 代码（commit c2f7792），文中大部分问题已在后续提交中修复。当前代码状态见文档末尾的 Phase B/C/D 更新脚注。阅读本文时请以脚注中的实现状态为准。

## 定位

本文是对 `feat(collector): implement Phase A framework` (c2f7792) 的全面代码审核。
审核维度：社区 crate 优先度、Rust 1.95 原生特性优先度、代码风格与命名惯用法。

审核后不直接修改代码。本文作为设计级治理文档，经确认稳定后再进入实现。

---

## 审核结论摘要

| 问题类别 | 数量 | 最高优先级 |
|---|---|---|
| 缺少社区 crate（手写本应由 crate 处理的逻辑） | 5 | **Critical** |
| async-trait 非必要（Rust 1.95 有更优方案） | 1 | **Critical** |
| 非 Rust 惯用法命名和设计模式 | 9 | High |
| 阻塞 I/O 在 async 上下文 | 6 | High |
| 用 String 做错误类型（失去 ? 传播、错误区分、堆栈链） | 8 | High |
| 重复解析 /proc（procfs crate 未被充分利用） | 4 | Medium |
| 重复代码模式 | 9 | Medium |
| 多余字段/死代码 | 3 | Low |

---

## 一、缺少社区 crate

### 1.1 错误类型：全代码用 `Result<T, String>` 而非 proper error type

**涉及**：`src/collector/mod.rs:12-29`、所有 collector 的 `collect()`、`src/output.rs` 全部方法、`src/main.rs:436` (`create_tar_gz`)

用 `String` 做错误类型导致：
- 无法用 `?` 运算符在跨抽象层传播（`From` trait 不可实现）
- 调用者无法按错误种类分支处理（须 match 字符串内容）
- 无 backtrace、无 source chain
- 与 `std::error::Error` 生态不兼容（`color-eyre`、`anyhow::Context`）

**修复方向**：
- 用 `thiserror` 派生三个 error enum：
  - `CollectError`（采集器错误：超时、截断、权限拒绝、接口不可用）
  - `OutputError`（输出模块错误：IO、序列化、大小超限）
  - `ArchiveError`（打包错误）
- `Cargo.toml` 新增 `thiserror = "2"`
- ADR 0003 crate 表补 `thiserror`

### 1.2 日志：全代码用 `eprintln!` 而非 logging crate

**涉及**：`src/main.rs:230, 286, 369, 408, 425-426`

`eprintln!` 无日志级别、无时间戳、无法重定向到文件、无法按级别过滤。
在目标机器（不稳定的重启前环境）上，stderr 可能丢失。

**修复方向**：
- 用 `tracing` + `tracing-subscriber`（tokio 生态原生，`zbus` 已依赖 `tracing`）
- `Cargo.toml` 新增 `tracing = "0.1"`、`tracing-subscriber = "0.3"`
- ADR 0003 crate 表补两项
- 所有 `eprintln!` 替换为 `tracing::error!`/`tracing::warn!`/`tracing::info!`

### 1.3 /proc 解析：procfs crate 未被充分利用

**涉及**：`src/collector/boot.rs:84-93`、`src/collector/cpu.rs:72-84`、`src/collector/memory.rs:85-117`

`procfs = "0.18"` 已在 Cargo.toml 中。但仅有 `process.rs` 使用 `procfs::process::all_processes()`。
另外三个 collector 仍用 `std::fs::read_to_string` 读原始字符串：

- `boot.rs` 读 `/proc/uptime` → 手工 parse f64，而 `procfs::Uptime` 返回 `(f64, f64)` 结构化
- `memory.rs` 手写 19 行 `parse_meminfo()` 用 `splitn(2, ':')` 解析，而 `procfs::Meminfo` 已提供所有字段的类型化访问

**修复方向**：
- boot.rs：用 `procfs::kernel::*` 或直接读但以 `procfs` 类型包装
- memory.rs：删除 `parse_meminfo()`，用 `procfs::Meminfo::new()` 获取类型化数据
- cpu.rs：`/proc/loadavg` 用 `procfs::LoadAverage`，其余可保持原样（`/proc/stat` 巨大但 procfs 有 `KernelStats`）

### 1.4 临时文件：手工 temp + rename 而非 tempfile crate

**涉及**：`src/output.rs:31-68, 94-111, 166-183, 197-212`

手写的 "写 .tmp → rename" 有三个问题：
- TOCTOU race：`self.tmp_path.exists()` 检查与 `fs::write` 之间文件可能被创建（行166, 252）
- 无 panic 安全清理：若写入中 panic，`.tmp` 残留
- `OpenOptions::create(true).append(true)` 可同时处理创建和追加，无需分支

**修复方向**：
- 用 `tempfile::NamedTempFile` 替代手写，或
- 至少移除 `exists()` 检查，统一用 `OpenOptions::create(true).append(true).open()`

**注意**：`tempfile::NamedTempFile` 默认在 `/tmp` 创建临时文件，`persist()` 到输出目录时若跨文件系统则失败（`EXDEV`）。
需用 `tempfile::Builder::new().tempfile_in(output_dir)` 确保临时文件和目标在同一文件系统。

### 1.5 时间格式化：chrono 不在 ADR 0003 crate 表中

**涉及**：`Cargo.toml:24`、`src/output.rs:7`、`src/main.rs:9`

`chrono` 在实现时被添加但 ADR 0003 未列。纯 Rust，合理，但需补齐文档。

**修复方向**：
- ADR 0003 crate 表补 `chrono`

---

## 二、async-trait 非必要

### 2.1 Rust 1.95 的 async trait 现状

调研结论：
- `async fn` in trait definition（RPITIT）已 stable（Rust 1.75），但仅支持**静态分发**。
- `dyn Trait` + `async fn`（dynamic dispatch）在 Rust 1.95 **尚未 stable**（需 nightly `#![feature(async_fn_in_dyn_trait)]`）。
- 如果继续使用 `Vec<Box<dyn Collector>>` 进行动态分发，`async-trait` crate 仍然**不可避免**。

### 2.2 替代方案：放弃 dyn dispatch，改用 enum dispatch

当前架构用 `Vec<Box<dyn Collector>>` 并在 tokio task 中分发。
这同时引入了：
- `async-trait` proc macro 依赖
- `Box` heap allocation（对零大小类型是纯浪费）
- trait object 的虚函数调用开销（微小但非零）

**更优 Rust 方案**：

用 enum 包装每个 collector 变体，`match` 实现静态分发：

```rust
enum CollectionTask {
    Boot(Boot),
    Cpu(Cpu),
    Memory(Memory),
    Process(Process),
}

impl CollectionTask {
    fn id(&self) -> &'static str {
        match self {
            CollectionTask::Boot(_) => "RT-01",
            CollectionTask::Cpu(_) => "RT-05",
            CollectionTask::Memory(_) => "RT-06",
            CollectionTask::Process(_) => "RT-04",
        }
    }
    // probe(), collect() 同理
}
```

优势：
- 零 heap allocation（所有 collector 是 unit struct，总大小为一个 discriminant byte）
- 无 `async-trait` crate 依赖
- 无虚函数调用开销
- `match` 穷尽性由编译器强制（新增 RT 大类时编译器提醒漏了 match arm）
- 编译时 monomorphization → 更好的优化

**对架构文档的影响**：
- `docs/collector-architecture.md` 中的 `Collector` trait 定义需更新为 enum 方案
- `pub trait Collector` 可保留为文档概念说明，但实现上走 enum dispatch
- ADR 0003 crate 表移除 `async-trait`、新增 `thiserror`、`tracing`、`tracing-subscriber`、`chrono`

---

## 三、非 Rust 惯用法命名和设计模式

### 3.1 `Collector` 后缀命名（Java 接口实现模式）

**涉及**：`BootCollector`(boot.rs:9)、`CpuCollector`(cpu.rs:9)、`MemoryCollector`(memory.rs:9)、`ProcessCollector`(process.rs:10)

在 Java/C# 中，`class FooCollector implements ICollector` 是常态。
在 Rust 中，模块路径已提供语境。`collector::boot::Boot` 比 `collector::boot::BootCollector` 更简洁且不损失信息量 (`use collector::boot::Boot`)。

**修复**：`s/BootCollector/Boot/g`、`s/CpuCollector/Cpu/g` 等。

### 3.2 `CollectResult`/`ProbeResult` 动词+名词复合命名

**涉及**：`src/collector/mod.rs:12, 23`

保留 `Result` 后缀在 Rust 中有与 `std::result::Result` 歧义的风险。

**修复方向**（与 enum dispatch 同时考虑）：
- `ProbeResult` → `ProbeOutcome`
- `CollectResult` → `CollectionOutcome`
- `ProbeStatus`/`CollectStatus` → 不变

### 3.3 Unit struct 的空 `new()` 构造函数

**涉及**：`boot.rs:12-15`、`cpu.rs:12-15`、`memory.rs:12-15`、`process.rs:12-15`

```rust
pub struct BootCollector;
impl BootCollector {
    pub fn new() -> Self { BootCollector }
}
```

在 Rust 中 struct literal 本身即是构造：
```rust
let collector = BootCollector;  // 不需要 BootCollector::new()
```
`new()` 是有用状态/验证逻辑时的惯例；对空 struct 是纯样板代码。

**修复**：删除 `new()` 方法，直接用 `Boot` 作为值。若需要 `Default` 则 derive。

### 3.4 嵌套 match 金字塔（应采用 let-else + ?）

**涉及**：`boot.rs:104-134`、`cpu.rs:89-119`、`memory.rs:122-152`

三者结构完全一致：
```rust
match output.json_writer(...).await {
    Ok(writer) => match writer.commit(&record).await {
        Ok((size, _)) => { CollectResult { ... } }
        Err(e) => CollectResult { ... },
    },
    Err(e) => CollectResult { ... },
}
```

**修复**（配合 proper error type 用 `?` 后）：
```rust
let writer = output.json_writer(...).await?;
let (size, _) = writer.commit(&record).await?;
CollectionOutcome { status: CollectionStatus::Ok, ... }
```

### 3.5 `hardcoded collection: "RT-XX".to_string()` 在每行 JSON 中

**涉及**：boot.rs:96、cpu.rs:81、memory.rs:115、process.rs:125

每条 JSON 记录都嵌入 `"collection": "RT-XX"`。collector ID 已在 manifest.json 中记录，
数据文件按 collector 分文件，文件名已标识来源。嵌入 `collection` 字段造成每个 JSON 对象增加 ~20 bytes。

**修复**：保留 `collection` 字段（ADR 0004 已定义为强制），但改为 `&'static str` 静态常量而非 `.to_string()` 堆分配。

### 3.6 魔术 sentinel 值

**涉及**：`process.rs:106`（`pid: i32` 用 `-1` 做 sentinel）、`process.rs:113`（`name: String` 用 `"?"` 做 sentinel）

C 风格的 sentinel 值（-1、"?"）强迫所有下游消费者识别魔术值。
在 Rust 中应用 `Option<i32>`、`Option<String>`。

**修复**：`pid: Option<i32>`、`name` 不设 "?" fallback，不可读进程返回 `None`。

### 3.7 `Box::new()` 包装零大小类型

**涉及**：`main.rs:237-241`

`BootCollector` 等均为 unit struct（大小 0 bytes）。`Box::new(BootCollector)` 分配堆内存仅存指针。

用 enum dispatch 后彻底消除此问题。

### 3.8 `main.rs` 中独立重读 `/proc` 文件

**涉及**：`main.rs:86-116`（`read_hostname`、`read_kernel`、`read_boot_id`、`read_uptime`）

这些函数重读 BootCollector 已采集过的文件。
若 boot_id 在采集后与 manifest 写入间变化（理论上可能），数据不一致。

**修复**：manifest.host 的值从 `boot.rs` 的 `CollectionOutcome` 中提取，而非独立重读。

### 3.9 `build_summary` 从磁盘回读 memory.json

**涉及**：`main.rs:148-163`

`MemoryCollector` 刚写入 `memory.json`，`build_summary` 立即打开文件、JSON 反序列化、
字符串 key 查找提取两个值（MemTotal、MemAvailable）。完全浪费。

**修复**：MemoryCollector 的返回结构体直接提供 `mem_total_kb` 和 `mem_available_kb`
（但不放入 JSONL，仅用于 summary 构建）。

---

## 四、阻塞 I/O 在 async 上下文

### 4.1 所有 collector 在 async fn 中调用同步 fs 操作

**涉及**：`boot.rs:78-82`、`cpu.rs:76-78`、`memory.rs:79-81`、`output.rs:96, 103, 166, 198`、`main.rs:86-116, 420-429`

`std::fs::read_to_string`、`std::fs::write`、`std::fs::rename`、`serde_json::to_vec`（CPU-bound）
在 tokio async runtime 线程上执行，阻塞 worker 线程。

**修复**：
- procfs/sysfs 读用 `tokio::fs` 或 `spawn_blocking`（虽然 procfs 是虚拟文件系统不产生磁盘 I/O，但仍会阻塞 worker 在 kernel 态短暂等待）
- 序列化操作（CPU-bound）用 `tokio::task::spawn_blocking`
- tar.gz 创建用 `tokio::task::spawn_blocking`

**注意**：ADR 0003 第 34 行说"procfs/sysfs 文件读取本身是同步操作（虚拟文件系统不阻塞），在 async 上下文中直接调用，不阻塞 worker"。
此陈述对轻量 read 通常是正确的，但对大量 read（如所有进程的 cmdline）或高负载主机上可能造成可感知的 worker 阻塞。
对关键维护性操作（procfs 小文件、单次 read）直接调用是可接受的，但应在代码中标明哪些是"安全同步"。

---

## 五、重复代码

### 5.1 三个 collector 的 probe() 结构一致

**涉及**：`boot.rs:23-53`、`cpu.rs:23-51`、`memory.rs:23-50`

三个 probe 都以 `for f in &files` 循环 + `Path::exists()` 检查 + 三态判断（全不可用/全可用/部分降级）。
仅文件列表不同。可用一个辅助函数 `fn probe_files(files: &[&str]) -> ProbeResult` 消除重复。

### 5.2 三个 collector 的 collect() 返回结构一致

**涉及**：`boot.rs:104-134`、`cpu.rs:89-119`、`memory.rs:122-152`

三者相同的 `Ok(writer) => match writer.commit() { Ok((size, _)) => CollectResult { status: Ok, ... }, Err(e) => ... }, Err(e) => ...`。
用 `?` 后自然消除。

### 5.3 `build_summary` 中的 MemTotal/MemAvailable 字符串匹配

**涉及**：`main.rs:153-159`

直接用 `item["key"].as_str()` 匹配 `"MemTotal"` / `"MemAvailable"` → 硬编码字符串，易拼写错误。
用 `procfs::Meminfo` 的类型化字段可消除。

### 5.4 `probe` 参数传入 `collect()` 但从未使用

**涉及**：boot.rs:108、cpu.rs:93、memory.rs:107、process.rs:67

所有 collector 的 `collect()` 签名都接收 `probe: &ProbeResult`，但 no implementation reads it。
架构文档定义的"避免重复探测"意图未落地——实际是 collect() 内又自己读了一遍 /proc。

**修复**：collect() 中检查 probe result 的 `ProbeStatus`。若 probe 阶段就确定某文件不可用，collect 应跳过并标记 `Degraded`。

### 5.5 `cleanup_tmp` 中的 `.flatten()` 吞没错误

**涉及**：`output.rs:73`

`for entry in dir.flatten()` 对 `Result<DirEntry, io::Error>` 调用 `.flatten()`，将 `Err` 转换为 `None`，静默丢弃错误。
若某目录条目不可读，应至少 warn 日志。

**修复**：显式 `match entry` 并 `tracing::warn!` 记录跳过原因。

### 5.6 `OutputDir` 在 main 中通过结构体字面量绕过 `create()`

**涉及**：`main.rs:264-266`

```rust
let output = OutputDir { root: manifest_dir.clone() };
```

绕过 `OutputDir::create()`，不验证目录存在、不记录创建路径。
任何同事阅读代码时必须先理解"前面已调用过 create"才能信任此构造。

**修复**：`create()` 返回 `OutputDir`，main 中只持有 Arc。或给 `OutputDir` 加上工厂方法 `from_existing(root)` 明确意图。

### 5.7 `process.rs` 中 `proc_entry = match entry { Ok(p) => Some(p), Err(_) => None }` 等价于 `.ok()`

**涉及**：`process.rs:99-102`

```rust
let proc_entry = match entry {
    Ok(p) => Some(p),
    Err(_) => None,
};
```

完全等价于 `entry.ok()`。多余 4 行代码。

### 5.8 `tokio::time::Instant` 使用

**涉及**：boot.rs:71、cpu.rs:86、memory.rs:120、process.rs:68

Rust 社区惯例：在 tokio 的 multi-threaded runtime 中，`std::time::Instant` 不受 `tokio::time::pause()` 影响
（生产代码不用 pause），因此直接使用 `std::time::Instant` 更简单。

**修复**：`tokio::time::Instant` → `std::time::Instant`。同时消除 `use tokio::time::Instant`。

---

## 六、死代码和多余字段

### 6.1 `started_at` 字段存储但从未读取

**涉及**：`output.rs:90, 119, 218`

三个 Writer 结构体都存 `started_at` field，从未读取。可用于 duration 计算但当前未使用。

**修复**：移除。若后续需要记录 write latency，再加回来。

### 6.2 `ProbeSummary` 追踪不存在的 RT-03/RT-11/RT-12/RT-13

**涉及**：`main.rs:200-217`

当前只实现了 RT-01/04/05/06，但 `ProbeSummary::update` 匹配 "RT-03"、"RT-11"、"RT-12"、"RT-13"。

**修复**：保留匹配逻辑（Phase B/C 会用到），但加 `#[allow(dead_code)]` 标注。

> **Phase B 更新 (2026-05-17)**: RT-03 已在 Phase B 实现（`systemd.rs`），`ProbeSummary` 已重构移除。RT-11/RT-12/RT-13 仍待 Phase C。
>
> **Phase C+D 更新 (2026-05-17)**: RT-02、RT-07 至 RT-21 全部 17 个剩余 collector 已实现。RT-01 至 RT-21 全部 21 个大类已完成。Phase E（测试）为下一阶段。
>
> **0.1.1 更新 (2026-05-18)**: RT-22 (filesystem) 作为第 22 个 collector 新增，`CollectionTask` 变体从 21 增至 22。`TextWriter` 已移除（RT-20 改用 JSON）。

### 6.3 `TextWriter` 全结构未使用

**涉及**：`output.rs:213-280`

`TextWriter` 定义完整但无 collector 使用（RT-20 dmesg 是 Phase D 的工作）。

**修复**：保留，加 `#[allow(dead_code)]`。
>
> **0.1.1 更新 (2026-05-18)**：`TextWriter` struct/impl 和 `text_writer()` 方法已在 v0.1.1 移除（72 行死代码）。RT-20 dmesg 改用 JSON 格式输出（`json_writer` + `EventsRecord` struct）。

---

## 七、需更新的上游文档和 ADR

### 7.1 ADR 0003 crate 表变更

| 操作 | Crate | 理由 |
|---|---|---|---|
| 新增 | `thiserror` 2.x | 派生 error enum，全代码 `Result<T, String>` → proper error types |
| 新增 | `tracing` 0.1 | 替代 `eprintln!`，与非阻塞输出和日志过滤兼容 |
| 新增 | `tracing-subscriber` 0.3 | `tracing` 的终端 subscriber |
| 新增 | `chrono` 0.4 | 输出目录和 manifest 的时间格式化（已使用，补文档） |
| 新增 | `tempfile` 3.x | 原子的临时文件创建 + persist 重命名，消除手写 TOCTOU 和 panic 安全问题 |
| 移除 | `async-trait` 0.1 | 改用 enum dispatch 后不再需要 |

### 7.2 架构文档变更

- `docs/collector-architecture.md` 第 41-113 行（`Collector` trait 定义、模块结构）
  - `Vec<Box<dyn Collector>>` → `CollectionTask` enum
  - `#[async_trait]` 删除
  - 模块结构图中 `collect/` 内的文件名需反映命名变更
  - 第 44 行 `boot.rs` 保留、`kernel.rs` 保留、其余不变
- 第 88-113 行 trait 代码块需替换为 enum dispatch 代码块

### 7.3 CHANGELOG 新条目

需在 `CHANGELOG.md` 顶部新增条目：

```
## 2026-05-15 - Phase A 审核：社区 crate、async 惯用法与代码风格重构

- 类型：设计 / 重构
- 范围：本文 + `docs/decisions/0003-implementation-tech-stack.md` + `docs/collector-architecture.md`
- 提交信息：`refactor(collector): replace async-trait with enum dispatch, adopt proper error types and logging`
```

---

## 八、实现优先级和顺序

| 优先级 | 任务 | 涉及文件 |
|---|---|---|
| P0 | 定义 error enum（`thiserror`） | `src/error.rs`(新)、Cargo.toml |
| P0 | enum dispatch 替代 async-trait | `src/collector/registry.rs`(新)、所有 collector + main |
| P1 | 用 `procfs` 替代手写 /proc 解析 | `src/collector/boot.rs`、`src/collector/memory.rs` |
| P1 | `tracing` 替代 `eprintln!` / `println!` | `src/main.rs`、Cargo.toml |
| P1 | 修复魔术 sentinel 值 | `src/collector/process.rs` |
| P2 | 修复命名 | 全局 `s/BootCollector/Boot/g` 等 |
| P2 | 修复阻塞 I/O（tokio::fs / spawn_blocking） | 所有 collector + main |
| P3 | 修复 main 中重读 /proc 文件 | `src/main.rs` |
| P3 | 修复 `build_summary` 磁盘回读 | `src/main.rs` + `src/collector/memory.rs` |
| P3 | 更新 ADR 0003 + 架构文档 | `docs/decisions/0003-*`、`docs/collector-architecture.md` |

---

## 九、确认清单（审核闭环）

以下事项需逐项确认后进入实现：

- [ ] 所有新增 crate（`thiserror`、`tracing`、`tracing-subscriber`、`chrono`、`tempfile`）均为纯 Rust、musl 静态链接兼容、零 C 编译依赖
- [ ] enum dispatch 方案不引入额外复杂度（match 臂数 = collector 种类数，每类约 100-200 行）
- [ ] 命名变更后与 `docs/project-map.yml`、`docs/index.md` 中的入口语义一致
- [ ] ADR 0003 移除 `async-trait` 不违反任何既有决策
- [ ] CHANGELOG 新条目格式符合 `docs/change-management.md` 规范
