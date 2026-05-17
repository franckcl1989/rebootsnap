# 0004 - 固定 collector 输出格式

- **状态**：生效
- **日期**：2026-05-15
- **范围**：`collector`

## 背景

- `docs/collector-security-governance.md` 固定了原始值记录策略：collector 不做脱敏，全部原始值输出。
- collector 的输出必须同时被人类（直接阅读或 grep）、工具（程序化解析）和 AI 代理（智能分析）消费。
- 采集数据量差异悬殊：RT-01 只有数百字节，RT-04 进程列表可达数十万条记录。
- collector 可能在全局超时后非正常退出，已有输出必须有效可读。
- 敏感字段已在 collector 中以 `sensitive` 标签标记，下游需要据此决定呈现粒度。

## 决策

### 目录式输出

使用目录结构输出，而非单一 JSON 文件。

输出根目录为 `rebootsnap-{timestamp}/`，其中 `{timestamp}` 为采集启动时的本地时间（`YYYYMMDD-HHMMSS` 格式）。目录内每个 RT 大类一个文件，另加 `manifest.json` 和 `summary.json` 两个导航文件。

```
rebootsnap-{timestamp}/
  manifest.json       # 采集范围索引
  summary.json        # 关键指标浓缩
  boot.json           # RT-01
  kernel.json         # RT-02
  systemd.json        # RT-03
  processes.jsonl     # RT-04（大表）
  cpu.json            # RT-05
  memory.json         # RT-06
  fds.json            # RT-07
  tmpfs.json          # RT-08
  mounts.json         # RT-09
  block.json          # RT-10
  netdev.json         # RT-11
  sockets.json        # RT-12
  netfilter.json      # RT-13
  ipc_ns_cg.json      # RT-14
  sessions.json       # RT-15
  security.json       # RT-16
  devices.json        # RT-17
  power.json          # RT-18
  time.json           # RT-19
  dmesg.txt           # RT-20（文本缓冲）
  caches.json         # RT-21
```

最终目录用 `tar` + `flate2` crate 打包为 `rebootsnap-{timestamp}.tar.gz` 单文件分发（非 shell 命令调用）。原始目录在打包后清理。

### 三种文件格式

| 格式 | 适用场景 | 约定 |
| --- | --- | --- |
| JSON | 条目数固定且较少的结构化数据 | 单根对象，顶层带 `collection` 字段标识 RT 大类。 |
| JSONL | 条目数不定、可能被截断的列表数据 | 每行一个自描述 JSON 对象，行间独立。截断时最后一行标记 `{"truncated":true,"reason":"size_limit"}`。 |
| .txt | 日志/缓冲等时序文本 | 保留原始行序和格式，字节截断。 |

### manifest.json — 导航层

面向工具和 AI 的第一入口，回答「这次采集了哪些内容、哪些有问题」。

顶层结构：

```json
{
  "rebootsnap_version": "0.1.0",
  "collected_at": "2026-05-15T14:30:00+08:00",
  "host": {
    "hostname": "web01",
    "kernel": "4.18.0-553.el8.x86_64",
    "boot_id": "abc-def-123",
    "uptime_seconds": 864000
  },
  "probe": {
    "systemd": "available",
    "netlink": "available",
    "conntrack": "denied",
    "hidepid": "none",
    "capabilities": ["CAP_NET_ADMIN"]
  },
  "items": [
    {
      "id": "RT-01",
      "status": "ok",
      "file": "boot.json",
      "size_bytes": 512,
      "duration_ms": 12
    },
    {
      "id": "RT-04",
      "status": "truncated",
      "file": "processes.jsonl",
      "size_bytes": 52428800,
      "duration_ms": 850,
      "total_items": 52340,
      "collected_items": 50000,
      "truncation": "item_count_limit"
    },
    {
      "id": "RT-13",
      "status": "degraded",
      "file": "netfilter.json",
      "missing": ["nftables_rules"],
      "reason": "CAP_NET_ADMIN required for nftables netlink"
    }
  ],
  "global_duration_ms": 45200,
  "exit_reason": "completed"
}
```

字段约定：

- `status` 枚举：`ok`（完整采集）、`truncated`（因上限截断）、`degraded`（部分子项不可用）、`failed`（完全失败）、`timed_out`（超时）。
- `items` 按 RT 编号升序排列，便于工具遍历。
- 若一个 item 产生多个文件（如 RT-20 输出 `dmesg.txt` 和 `journal-tail.txt`），用 `files`（复数，字符串数组）替代 `file` 字段。
- 若全局超时退出，`exit_reason` 为 `global_timeout`，`items` 末尾记录未完成的大类。

### summary.json — 指标层

面向人类和 AI 的快速概览，不做判断，只输出从采集数据中提取的关键指标。

```json
{
  "collection_time": "2026-05-15T14:30:00+08:00",
  "boot_time": "2026-05-05T14:30:00+08:00",
  "uptime_seconds": 864000,
  "indicators": {
    "load_1_5_15": [4.2, 3.1, 2.8],
    "iowait_pct": 18.0,
    "steal_pct": 0.0,
    "cpu_pressure_10s": 45.2,
    "procs_total": 52340,
    "procs_zombie": 3,
    "procs_running": 12,
    "procs_blocked": 3,
    "memory_total_gb": 64.0,
    "memory_available_gb": 2.1,
    "memory_pressure_10s": 68.0,
    "swap_used_gb": 8.5,
    "oom_kill_count": 12,
    "io_pressure_10s": 22.0,
    "tcp_established": 8600,
    "tcp_timewait": 3400,
    "listen_ports": [22, 80, 443, 3306],
    "conntrack_usage_pct": 92.0,
    "block_errors": 340
  },
  "top_consumers": {
    "cpu": [{"pid": 4523, "name": "mysqld", "pct": 85.0}],
    "memory": [{"pid": 4523, "name": "mysqld", "rss_gb": 24.0}],
    "fds": [{"pid": 4523, "name": "mysqld", "count": 4500}]
  },
  "filesystems_ro": ["/data"],
  "degraded_items": {
    "nftables_rules": "CAP_NET_ADMIN denied"
  }
}
```

`indicators` 中的字段全部为可选——如果对应接口不可用，字段缺省，不输出 `null`。AI 从字段存在与否即可判断对应能力是否可用。

若某个指标来源于被截断的采集数据（例如 `procs_total` 来自被条目截断的 RT-04），该指标输出截断后的采集值，并追加同名 `_truncated: true` 字段。例如当进程列表被截断时输出 `"procs_total": 50000, "procs_total_truncated": true`。

`top_consumers` 列出 CPU、内存、fd 三个维度各自的前 3 个进程。

### 自描述数据对象

每个 JSON 和 JSONL 文件中的顶层对象或每行对象都必须包含 `collection` 字段，值为对应的 `RT-NN` 标识。工具和 AI 无需外部 schema 文件即可识别数据来源。

示例（`memory.json` 顶层）：

```json
{
  "collection": "RT-06",
  "meminfo": {
    "MemTotal_kB": 67108864,
    ...
  },
  ...
}
```

示例（`processes.jsonl` 中的一行）：

```json
{"collection":"RT-04","pid":1,"ppid":0,"name":"systemd","state":"S","...":""}
```

### AI 消费路径

AI 代理的标准分析路径：

1. 解压归档，读 `manifest.json` → 了解采集范围、降级情况和全局耗时。
2. 读 `summary.json` → 获取关键指标，判断是否存在需要关注的异常方向。
 3. 根据指标定向进入对应 RT 文件 → 例如 OOM 钻入 `memory.json` 和 `processes.jsonl`，网络异常钻入 `sockets.json` 和 `netfilter.json`。
4. 交叉关联 → `pid` 从 `processes.jsonl` 关联到 `fds.json` 和 `sockets.json`，`inode` 从 `sockets.json` 关联到 `processes.jsonl`。

## 影响

- collector 实现必须按本文的文件清单逐一输出，文件名稳定不变。
- 新增采集项或移动采集项到不同 RT 大类时，本文的文件清单必须同步更新。
- `manifest.json` 和 `summary.json` 的字段一旦由 collector 发布，不得在次版本内删除或重命名。
- JSONL 文件的行顺序不保证稳定；依赖行顺序的消费者必须自行按字段排序。
- 下游工具（报告层、脱敏层）应通过 `manifest.json` 中的 `items[].file` 字段获取文件列表，不应硬编码文件名或遍历目录。
- `sensitive` 标签由 `docs/collector-security-governance.md` 定义，本文不重复定义脱敏策略。

## 备选方案

| 方案 | 未采用原因 |
| --- | --- |
| 单 JSON 文件 | 大主机上 JSON 体积轻松超过 100 MB，解析器必须全量加载；采集中断导致文件不合法；人类阅读不友好。 |
| YAML | 解析性能差于 JSON；AI 工具对 JSON 原生支持更好；非结构化文本仍需要单独处理。 |
| 数据库（SQLite） | 引入 C 依赖，违反纯静态二进制约束；AI 不能直接 grep；分发需要配套读取工具。 |
| 每个进程一个文件 | 文件系统压力大（50000 个 inode），打包和解压开销高。 |
| 不要 summary.json | AI 每次分析都需要遍历全部文件才能形成概览，重复计算成本高。`summary.json` 是总结不是判断，不会引入偏见。 |

## 验证

- 人工审核输出结构满足三种消费者需求：人类（grep 直接看 `.txt` 和 `.jsonl`）、工具（按 `manifest.json` 索引遍历）、AI（`manifest` → `summary` → 定向钻入）。
- 人工审核 JSONL 截断语义：最后一行标记截断原因、已采集和总条目数，解析器可据此判断完整性。
- 人工确认 `summary.json` 中不包含判断性结论，全部为事实性指标。
- 本决策不改变现有采集分类、安全约束和测试治理基线。
