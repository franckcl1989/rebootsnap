# Collector 安全治理

## 文档定位

本文是 RebootSnap collector 实现必须遵守的安全治理规范。本文在 ADR 0002（`docs/decisions/0002-human-ai-governance.md`）要求的采集安全治理门槛上展开，是 collector 安全边界的唯一权威来源。

本文不替代 `docs/linux-runtime-info-collection-decision.md` 中的各分类决策，而是从安全视角统一解释和加固那些分散的约束。

## 只读采集原则

collector 默认不得执行任何会改变 OS 状态的操作。具体禁止：

- 写入 sysctl 或修改内核参数。
- 刷新或失效缓存（sync、drop_caches、nscd -i 等）。
- 触发文件系统修复、一致性检查或 fsck。
- 重启、停止、暂停或 reload 任何服务。
- 加载或卸载内核模块。
- 附着新的 kprobe、uprobe、tracepoint、BPF program 或其他动态观测对象。
- 创建新的 cgroup、namespace、IPC 对象或临时文件系统挂载点。
- 执行可能产生副作用的设备探测、固件加载或驱动绑定操作。

collector 只能通过以下已有接口读取状态：

- procfs（`/proc/`）
- sysfs（`/sys/`）
- devfs（`/dev/`）
- netlink socket（rtnetlink、genetlink 只读操作）
- service manager D-Bus（只读查询）
- 系统守护进程已有的只读查询接口

读取操作本身不应显著改变系统行为，但允许以下已知副作用：

- 访问已经在内存中的 procfs/sysfs 节点可能触发少量内核内存分配，这在紧张环境下是可以接受的。
- 读取 `/proc/<pid>/` 下的映射文件（如 `smaps`）可能在进程规模极大的情况下产生可观测的 CPU 开销，因此这类采集项被分类为 `选采` 而非 `必采`。

**反例**：以下操作即使在故障排查中常用，collector 也不得执行：

- `echo 3 > /proc/sys/vm/drop_caches`
- `sysctl -w net.ipv4.ip_forward=0`
- `systemctl restart <service>`
- `modprobe <module>`
- `bpftool prog load/attach`
- `mount -t tmpfs /run/user/...`
- `fsck /dev/sda1`

## 敏感信息禁止清单

以下内容在任何情况下都不得由 collector 采集：

- 环境变量值（仅记录进程级环境变量存在性，不记录键名和值）。
- 密钥 payload（SSH private key、TLS private key、keyring 中的密钥材料、Kerberos ticket 内容）。
- 认证材料内容（sudo credential cache 内容、SSH agent 中的 key、PAM 认证 token）。
- 应用业务 payload（共享内存、消息队列、tmpfs 文件中承载的业务数据；只记录对象身份、大小、权限和引用关系）。
- 普通文件内容（已删除但仍被进程打开的文件、memfd 的内容不采集；只记录引用关系、大小和 flags）。
- 用户隐私数据（无显式授权时不采集用户的主目录文件列表、Shell 历史、编辑器临时文件）。

可以采集但需要在元数据中标明敏感的字段见下文「数据记录策略」。

### 安全标注

对落入敏感范围但并非禁止采集的元数据，collector 实现中应使用固定的 `sensitive` 标签标注，供下游报告层和脱敏层识别：

| 敏感类别 | 示例 | collector 行为 |
| --- | --- | --- |
| 主机身份 | hostname | 原始值记录 |
| 网络终结点 | IP 地址、端口号 | 原始值记录 |
| 用户标识 | UID、用户名、loginuid | 原始值记录 |
| 文件路径 | cmdline、exe、cwd、root、fd 目标路径 | 原始值记录 |
| 设备标识 | 设备路径、序列号 | 原始值记录 |

collector 自身不做脱敏。脱敏是报告层或下游消费者的职责，详见「数据记录策略」。

## 数据记录策略

collector 记录所有可采集字段的原始值，不在采集阶段做脱敏、掩码、哈希或截断（除输出大小上限外）。

理由：

- 原始值保留现场真实性。脱敏是不可逆操作，在采集阶段执行会永久丢失证据。
- 报告层可以基于 `sensitive` 标签决定呈现粒度（原始 / 掩码 / 省略），不同审计或分享场景可以有不同的脱敏策略。
- 避免 collector 引入脱敏 bug 导致静默丢失关键故障证据。

约束：

- collector 生成的数据文件必须存放在受权限保护的目录中（所有者 root，权限 0600 或 0640）。
- 每个采集项的输出中，带有 `sensitive` 标签的字段必须标记为敏感，使下游消费者无需语义理解即可识别。
- 如果未来需要为低权限 collector 提供脱敏输出，不应在 collector 中内嵌脱敏逻辑，而应通过独立的 post-processing 工具或报告层实现。

## 权限模型

### 最小运行权限

collector 期望以 root 运行，但应探测当前有效权限并在不足时按降级路径继续。

| 采集资源 | 典型最低权限 | 无权限时的行为 |
| --- | --- | --- |
| `/proc/<pid>/` 基础信息（PID、状态、cmdline） | root 或与目标进程相同 UID | 只采当前 UID 可见进程 |
| `/proc/<pid>/fd/` | root 或目标进程 owner | 记录 `permission denied`，继续下一进程 |
| `/proc/<pid>/environ` | root 或目标进程 owner | 记录不可用，跳过 |
| `/proc/sys/` | 通常只读对所有用户 | 按需读取 |
| `/sys/` 多数节点 | 通常只读对所有用户 | 按需读取 |
| nftables/iptables/conntrack | `CAP_NET_ADMIN` | 记录不可用，降级 |
| tc qdisc | `CAP_NET_ADMIN` | 记录不可用，降级 |
| BPF program/map 枚举 | `CAP_SYS_ADMIN` 或 `CAP_BPF` | 记录不可用，降级 |
| perf event | `CAP_PERFMON` 或 `CAP_SYS_ADMIN` | 记录不可用，降级 |
| trace/ftrace buffer | root | 记录不可用，降级 |
| audit | `CAP_AUDIT_READ` | 记录不可用，降级 |
| systemd D-Bus | 通常只读对本地用户 | 探测，不可用时记录 |

### hidepid 应对

当 procfs 以 `hidepid=1` 或 `hidepid=2` 挂载时，collector 只能看到当前 UID 的进程信息。此时：

- 在报告中标明 `hidepid` 生效，记录 procfs 挂载选项。
- 不尝试 umount / remount procfs 来绕过 hidepid。

### privilege boundary

collector 自身不应提升权限：

- 不使用 setuid binary。
- 不调用 sudo、pkexec 或 su。
- 不在运行时要求用户输入密码来提权。

collector 假设调用者已经提供了足够权限；权限不足时降级记录，不中断执行。

## 有界执行

每个采集项必须满足以下有界执行约束：

- **超时**：每个逻辑采集项（对应一个或一组三级项）必须设定硬超时。超时后记录 `timed out` 和已采集部分（如有），继续下一项。推荐逐项超时在 2 秒以内，总量级采集项可在 10 秒内。
- **输出大小上限**：单个采集项的输出不得超过 64 MiB。输出超过上限时截断，标记 `truncated` 和原始大小。
- **进程/条目数上限**：遍历类采集（进程、fd、连接表、conntrack、cgroup）必须设定数量上限（推荐 50000）。超过上限时截断，记录实际条目数。
- **递归深度上限**：目录遍历类采集（运行时目录、tmpfs 目录项）必须设定递归深度上限（推荐 3 级），禁止全量递归扫描。
- **全局超时**：整个 collector 运行不得超过 300 秒。超时后优雅退出，输出已采集数据，标注哪些大类未完成。

## 安全威胁模型

collector 自身不得引入新的攻击面。实现必须遵守：

- **不监听端口**：collector 不接受任何远程连接。所有输入通过命令行参数、配置文件或环境变量。
- **不写可预测文件路径**：输出文件的路径和名称由调用者或配置指定，不使用可被竞态攻击绑定的固定路径（如 `/tmp/rebootsnap.dat`）。
- **不解析不受信输入**：collector 的输出文件不解析为输入。如果未来支持增量采集和状态对比，该功能应使用受控格式（如带有完整性校验的结构化文件），而不是解析可能被篡改的任意文件。
- **不暴露敏感接口**：collector 不提供 RPC、HTTP、Unix socket 或任何形式的本地查询接口。输出仅为一次性文件写入。
- **无子进程 shell 注入**：如果 collector 未来通过外部命令采集（非默认路径），不得使用 shell 解释器拼接命令；必须使用 exec 直接调用可执行文件并分别传递参数。
- **内存安全**：collector 实现语言应避免缓冲区溢出、use-after-free 或未初始化内存的使用。如果使用 C/C++，应启用 ASan/UBSan 并在 CI 中运行。如果使用内存安全语言（Go、Rust），应在 CI 中验证没有 `unsafe` 代码或通过 `-race` 检测。

## 与最终采集决策的关系

本文的安全约束覆盖全部采集状态（`必采`、`选采`、`暂缓`、`禁止采集`、`待定`）：

- `必采` 项必须逐条满足本文约束后才能进入实现。
- `选采` 项除满足本文约束外，还必须满足 `docs/collector-testing-governance.md` 中的条件启用测试要求。
- `暂缓` 项被重新评估为 `必采` 或 `选采` 时，必须重新按本文逐条过安全约束。

## 验证清单

collector 实现提交前，必须确保：

- 每个采集接口确认了只读语义（无写入、无副作用、无 probe、无修复）。
- 环境变量值、密钥 payload、认证材料内容、业务 payload、普通文件内容均未出现在输出中。
- 每个采集项有独立超时和全局超时。
- 输出文件权限为 0600 或 0640，所有者为 root。
- collector 自身不监听任何端口，不写固定路径文件，不调用 sudo/pkexec 提权。
- 所有标注为 `sensitive` 的字段已在输出中标记。
