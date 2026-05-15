# 设计决策记录

## 文档定位

本文是 RebootSnap 设计决策记录目录。

当一次变更建立长期设计约束、取舍多个可行方案，或影响后续实现路径时，应新增设计决策记录。普通说明、临时计划和已在领域文档中完整表达的细节，不需要单独写决策记录。

## 记录规则

- 文件名使用 `NNNN-short-title.md`，从 `0001` 递增。
- 每条记录必须说明状态、背景、决策、影响和验证。
- 决策记录不替代 `CHANGELOG.md`；新增或修改决策记录仍需写入变更记录。
- 决策记录采用 `docs/templates/decision-record.md` 模板。

## 当前记录

| 记录 | 状态 | 范围 | 用途 |
| --- | --- | --- | --- |
| `docs/decisions/0001-runtime-info-boundary.md` | 生效 | `runtime-taxonomy` | 固定 Linux OS 运行时信息边界和分类取舍 |
| `docs/decisions/0002-human-ai-governance.md` | 生效 | `governance`、`change-management` | 固定人与 AI 共治执行模型和阶段门槛 |
| `docs/decisions/0003-implementation-tech-stack.md` | 生效 | `collector` | 固定 collector 实现语言、异步 runtime、crate 依赖和零配置原则 |
| `docs/decisions/0004-output-format.md` | 生效 | `collector` | 固定 collector 输出目录结构、文件格式和 AI 消费路径 |
