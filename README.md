# PureWorker

PureWorker 是一个面向教师场景的本地优先桌面 AI 助手（Tauri + Rust + React/TypeScript），用于支持教务管理、作业/考评结构化处理、家校沟通与评语生成等核心工作流。

> 当前仓库以方案设计与开发计划为主（v1.0），正在按里程碑推进实现。

## 项目目标

- 本地优先处理教师与学生数据，降低敏感信息外泄风险
- 通过 Agent 编排实现“输入素材 → AI处理 → 教师确认 → 导出成品”闭环
- 提供可追溯、可恢复、可审计的长任务执行机制

## 一期 MVP 范围（对齐 PRD v1.0）

1. **系统设置与个性化（模块 5）**
2. **基础教务管理（模块 3）**
3. **作业/考评与题库（模块 4.2）**
4. **班务与家校沟通（模块 4.3）**

## 技术栈

- Desktop: Tauri 2.x（Windows/macOS）
- Frontend: React + TypeScript
- Backend: Rust（Rig 编排 + Tool/LLM 接入）
- Data: SQLite（结构化 + FTS）+ 本地文件系统记忆库

## 仓库结构

```text
pure-worker/
├─ doc/
│  ├─ prd-v1.0.md
│  ├─ tech-solution-v1.0.md
│  └─ development-plan-v1.0.md
└─ README.md
```

## 文档入口

- 产品需求：`doc/prd-v1.0.md`
- 技术方案：`doc/tech-solution-v1.0.md`
- 细粒度开发计划：`doc/development-plan-v1.0.md`
- 工作目录结构规范：`doc/workspace-directory-structure.md`

## 开发原则

- Local-first（本地优先）
- Human-in-the-loop（教师确认后生效）
- Security-by-default（最小权限、密钥托管、审计可追溯）
- White-box execution（步骤级进度可视）

## 快速开始（当前阶段）

当前仓库处于方案与计划阶段，建议先阅读：

1. `doc/tech-solution-v1.0.md`
2. `doc/development-plan-v1.0.md`（第 1 步含国内源环境配置）

后续代码落地后将补充：
- 本地开发启动命令
- 测试命令
- 构建与打包说明

## 状态

- [x] PRD v1.0
- [x] 技术方案 v1.0
- [x] 开发计划 v1.0
- [ ] 工程代码初始化
- [ ] MVP 功能实现

## License

本项目采用 **GNU Affero General Public License v3.0 (AGPL-3.0)**。

详见仓库根目录 `LICENSE` 文件。

---
