# 变更管理规范

## 文档定位

本文是 RebootSnap 的提交信息和变更记录格式的唯一规范来源。

当本文和 `CHANGELOG.md` 中的历史条目不一致时，以本文为准。`CHANGELOG.md` 只记录已经发生的变更，不再承载格式规则。

本文目标有两个：

- 人类读者可以在不读 diff 的情况下快速理解一次变更的意图、范围、影响和验证状态。
- AI 工具可以稳定解析提交信息和变更记录，提取类型、范围、约束变化和验证结论。

## 提交信息格式

提交信息采用轻量 Conventional Commits 风格，首行必须使用以下格式：

```text
type(scope): summary
```

首行硬性规则：

- `type` 必须来自本文的固定类型表，不允许自造类型。
- `scope` 必须填写，使用小写字母、数字和短横线，指向稳定模块、文档或设计域。
- `summary` 使用英文祈使句或名词化短句，说明本次提交的核心结果。
- `summary` 不超过 72 个字符，不以句号结尾，不使用 `update things`、`misc changes`、`fix stuff` 这类泛化表达。
- 一个提交只表达一个主要意图。若变更跨多个文件，`scope` 选择共同的稳定上层领域。

首行结构可按以下正则校验。`summary` 长度限制和禁用结尾句号仍按上方硬性规则执行：

```text
^(docs|design|feat|fix|refactor|test|chore|build|ci|perf|style|revert)\([a-z0-9]([a-z0-9-]*[a-z0-9])?\): [A-Za-z0-9]([ -~]*[A-Za-z0-9)])?$
```

固定类型表：

| type | 使用场景 |
| --- | --- |
| `docs` | 文档内容、文档索引、格式规范变更 |
| `design` | 架构、分类、边界、约束等设计决策变更 |
| `feat` | 新增用户可见功能或新的可用能力 |
| `fix` | 修复错误行为、错误文档或错误约束 |
| `refactor` | 不改变外部行为的结构调整 |
| `test` | 新增或修改测试、验证脚本、测试数据 |
| `chore` | 仓库维护、元数据、非功能性例行事务 |
| `build` | 构建系统、依赖、打包流程变更 |
| `ci` | CI/CD、自动化检查、发布流水线变更 |
| `perf` | 性能优化且不改变功能语义 |
| `style` | 格式化、排版、命名等不改变语义的调整 |
| `revert` | 回滚已有提交 |

推荐 scope 示例：

- `runtime-taxonomy`
- `change-management`
- `docs-index`
- `collector`
- `report`

提交正文在以下情况必须填写：

- 变更建立、修改或废弃了设计约束。
- 变更跨越多个模块，单行信息不足以说明边界。
- 变更包含不明显的取舍、兼容性影响或后续工作约束。
- 未运行常规测试，需要说明原因。
- 变更存在破坏性影响或迁移要求。

提交正文格式：

```text
Context:
- Why this change is needed.

Changes:
- What changed.

Impact:
- Which behavior, contract, design constraint, or follow-up work is affected.

Verification:
- Which checks, tests, or reviews were performed.
```

如果存在破坏性影响，正文末尾必须增加：

```text
BREAKING CHANGE: describe the incompatible change and required migration.
```

## 执行与强制

仓库使用同一套脚本同时约束人类提交和 AI 生成提交。

- `scripts/validate-change-management.sh` 是可执行校验入口。
- `scripts/verify.sh` 是提交前的统一验证入口。
- `.githooks/commit-msg` 在本地提交时调用校验脚本。
- `.github/workflows/change-management.yml` 在 push 和 pull request 中重复校验，防止本地 hook 被跳过。

本地仓库必须启用版本化 hook：

```text
scripts/setup-dev.sh
```

强制规则：

- 提交首行必须符合本文的 `type(scope): summary` 规则。
- 如果提交正文非空，必须包含 `Context`、`Changes`、`Impact`、`Verification` 四个小节。
- `CHANGELOG.md` 必须随提交一起暂存。
- 暂存区中的 `CHANGELOG.md` 顶部条目 `提交信息` 必须和本次提交首行完全一致。
- 暂存区中的 `CHANGELOG.md` 顶部条目必须包含固定元数据字段和三个固定小节，且每个小节至少有一条 bullet。
- 提交前必须运行 `scripts/verify.sh`，确认治理入口、文档链接、项目地图和变更管理规则一致。

本地 hook 可以被 `git commit --no-verify` 绕过。因此远端仓库应启用分支保护，并要求 `Change Management` workflow 通过后才能合并到主分支。

示例：

```text
docs(change-management): codify commit and changelog formats

Context:
- The repository needs one stable source for change-management rules.

Changes:
- Add a dedicated change-management document.
- Link the changelog and README to the rule source.

Impact:
- Future commits and changelog entries must follow the fixed formats.

Verification:
- Reviewed the document structure and checked repository links.
```

## 变更记录格式

变更记录写在 `CHANGELOG.md`。每次有面向项目历史有意义的变更，都应在文件顶部新增一条记录，最新变更在前。

每条记录必须使用以下结构：

```markdown
## YYYY-MM-DD - 简短标题

- **类型**：文档 / 设计 / 实现 / 修复 / 重构 / 测试 / 工程化 / 构建 / 性能 / 样式 / 回滚
- **范围**：`path-or-domain`、`another-path-or-domain`
- **提交信息**：`type(scope): summary`

### 变更内容

- 具体发生了什么。

### 设计影响

- 它为后续工作建立、修改、保留或废弃了什么约束。

### 验证

- 已执行的检查、测试或人工审核结论。
```

条目硬性规则：

- 日期使用 `YYYY-MM-DD`，采用提交当天的本地日期。
- Markdown 结构和字段名必须保持一致，不重命名 `类型`、`范围`、`提交信息`、`变更内容`、`设计影响`、`验证`。
- `类型`、`范围`、`提交信息` 必须写成连续的三个 bullet 字段，避免 Markdown 渲染后合并成一个段落。
- 标题使用中文短句，直接说明结果，不写“更新”“优化”“调整”等孤立动词。
- `类型` 使用上方固定中文类型。可以用 ` / ` 连接多个类型，但必须按主要意图在前。
- `范围` 使用稳定文件路径、模块名或设计域；文件路径必须放在反引号中。
- `提交信息` 必须和实际提交首行一致。
- `变更内容` 只写已经发生的事实，不写计划、愿望或无法验证的判断。
- `设计影响` 必须说明约束变化。若没有设计影响，写明“不改变现有设计约束”及原因。
- `验证` 必须写清执行过的命令、检查或人工审核结论。未运行测试时必须写明原因。
- 每个小节至少包含一条 bullet。
- 不修改历史条目，除非是在修正明显事实错误、链接错误或格式错误。

中文类型和提交 `type` 的对应关系：

| 中文类型 | commit type |
| --- | --- |
| 文档 | `docs` |
| 设计 | `design` |
| 实现 | `feat` |
| 修复 | `fix` |
| 重构 | `refactor` |
| 测试 | `test` |
| 工程化 | `chore`、`ci` |
| 构建 | `build` |
| 性能 | `perf` |
| 样式 | `style` |
| 回滚 | `revert` |

推荐写法：

```markdown
## 2026-05-14 - 固化变更管理规范

- **类型**：文档 / 工程化
- **范围**：`docs/change-management.md`、`CHANGELOG.md`、`README.md`
- **提交信息**：`docs(change-management): codify commit and changelog formats`

### 变更内容

- 新增 `docs/change-management.md`，作为提交信息和变更记录格式的唯一规范来源。
- 更新 `CHANGELOG.md` 和 `README.md`，让规范入口稳定可见。

### 设计影响

- 后续提交和变更记录必须使用固定类型、固定字段和固定小节，便于人类审阅和 AI 解析。

### 验证

- 人工检查规范文档、变更记录和 README 链接的一致性。
```

## AI 与人工审核清单

提交前按以下清单检查：

- 提交首行能被拆成 `type`、`scope`、`summary` 三个字段。
- `type` 和变更记录 `类型` 都来自固定表。
- `scope`、`范围` 使用稳定概念或稳定路径，而不是临时口语描述。
- `CHANGELOG.md` 新条目包含日期、类型、范围、提交信息、变更内容、设计影响和验证。
- 所有影响性描述都说明事实、边界和后续约束。
- 验证结论可复现；未测试时原因明确。
- 没有使用 `一些`、`相关`、`若干`、`优化`、`完善` 等缺少对象的模糊表达。
