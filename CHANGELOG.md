# 变更记录

本文记录 RebootSnap 的重要设计、文档和实现变更。目标是让人类读者可以快速理解项目演进，也让 AI 工具可以稳定解析变更意图、范围和影响。

提交信息和变更记录格式的唯一规范来源见 [变更管理规范](docs/change-management.md)。

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
- 新增 `docs/glossary.md`，固定 RebootSnap、人机共治和运行时信息相关稳定术语。
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
