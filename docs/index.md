# 文档索引

## 文档定位

本文是 RebootSnap 的文档地图，帮助人类和 AI 快速找到权威入口、理解文档状态，并确认新增文档是否已经接入项目导航。

新增、移动或删除文档时，必须同步更新本文和 `docs/project-map.yml`。

## 核心入口

| 文档 | 状态 | 用途 |
| --- | --- | --- |
| `AGENTS.md` | 生效 | AI 协作入口和操作边界 |
| `README.md` | 生效 | 仓库简介和最小文档入口 |
| `CHANGELOG.md` | 生效 | 项目变更历史 |
| `docs/index.md` | 生效 | 文档地图和导航一致性规则 |
| `docs/project-governance.md` | 生效 | 人与 AI 共治规则 |
| `docs/change-management.md` | 生效 | 提交信息和变更记录规范 |
| `docs/project-map.yml` | 生效 | 机器可读项目地图 |
| `docs/glossary.md` | 生效 | 稳定术语表 |

## 设计文档

| 文档 | 状态 | 用途 |
| --- | --- | --- |
| `docs/linux-runtime-info-categories.md` | 生效 | Linux OS 运行时信息大类定义 |
| `docs/decisions/README.md` | 生效 | 设计决策记录目录 |

## 模板

| 文档 | 状态 | 用途 |
| --- | --- | --- |
| `docs/templates/changelog-entry.md` | 生效 | 变更记录条目模板 |
| `docs/templates/decision-record.md` | 生效 | 设计决策记录模板 |

## 维护规则

- 本文列出的路径必须真实存在。
- `README.md` 至少链接本文、变更记录、变更管理规范、共治规则和运行时信息大类定义。
- `docs/project-map.yml` 必须覆盖本文的核心入口、设计文档、模板和校验命令。
- 新文档如果不在本文中出现，视为未接入项目治理结构。
