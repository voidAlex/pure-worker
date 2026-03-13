# Skills 执行层 — 未实现功能清单

**创建时间**: 2026-03-13  
**状态**: 实现中  
**关联文档**: `doc/tech-solution-v1.0.md` §7.1-7.3, §14.5.5.1

## 概述

PureWorker 的 Skills 系统目前只完成了**管理层**（注册表 CRUD、IPC 命令、健康检查、Python 环境管理），但缺少完整的**运行执行层**。本文档列举所有需要实现的功能，按优先级排序。

## 已完成（管理层 ✅）

| 功能 | 文件 | 说明 |
|------|------|------|
| 技能注册 CRUD | `services/skill.rs` | 列表、查询、创建、更新、软删除 |
| IPC 命令 | `commands/skill.rs` | 6 个 IPC 命令已注册到 `lib.rs` |
| 数据模型 | `models/skill.rs` | SkillRecord (16 字段)、输入/输出结构 |
| 数据库表 | `migrations/0001_init.sql` + `0007_m5_settings.sql` | skill_registry 表 16 列 |
| 健康检查 | `services/skill.rs` | 内置技能/Python 环境路径检测 |
| Python 环境管理 | `services/uv_manager.rs` | uv 检测、虚拟环境创建、依赖安装 |
| uv 安装/修复 | `commands/uv_manager.rs` | 4 个 IPC 命令 |

## 未实现（运行执行层 ❌）

### P0: 统一工具协议（UnifiedTool Trait）

**技术方案参考**: §14.5.5.1

**目标**: 定义所有技能（内置 + Python + 第三方）的统一调用接口。

**需要实现**:
- `UnifiedTool` trait：`name()`, `description()`, `input_schema()`, `invoke()`
- `ToolResult` 结构：`success`, `data`, `error`, `degraded_to`
- `ToolAuditInfo` 结构：`tool_name`, `invoke_id`, `risk_level`, `duration_ms`
- JSON Schema 输入/输出规范（使用 `schemars = "1.0"`）

**新增文件**: `services/unified_tool.rs`

---

### P1: Skill 执行引擎（SkillExecutor）

**目标**: 统一技能调度，根据类型分发到 Rust 内置实现或 Python 子进程。

**需要实现**:
- `SkillExecutor::execute()` — 分发器，按 `skill_type` 路由
- `execute_builtin()` — 调用已注册的 `UnifiedTool` 实现
- `execute_python()` — 通过 `tokio::process::Command` 启动 Python 子进程
  - 使用 UvManager 创建的虚拟环境中的 Python 解释器
  - 传入 JSON 参数（`--input '{...}'`）
  - 捕获 stdout/stderr，解析 JSON 输出
  - 超时控制（默认 60 秒）
- 每次调用写入审计日志（AuditService）

**新增文件**: `services/skill_executor.rs`

**依赖**: P0（UnifiedTool trait）

---

### P2: 默认技能数据（Seed Data）

**目标**: 在数据库中预注册 5 个内置技能。

**需要实现**:
- SQL 迁移 `0009_skill_seed_data.sql`
- 5 个内置技能记录：
  1. `office.read_write` — Word/Excel 读写
  2. `ocr.extract` — OCR 文字提取
  3. `image.preprocess` — 图片预处理
  4. `math.compute` — 数学表达式计算
  5. `export.render` — 导出渲染
- 使用 `INSERT OR IGNORE` 确保幂等（尊重 UNIQUE(name, version) 约束）

**新增文件**: `migrations/0009_skill_seed_data.sql`

---

### P3: Skill 自动发现（SkillDiscovery）

**技术方案参考**: §7.1

**目标**: 自动扫描约定目录，解析 SKILL.md 并注册技能。

**需要实现**:
- 扫描路径：`./.agents/skills/`（项目级）、`~/.agents/skills/`（用户级）
- 解析 `SKILL.md` 文件：YAML frontmatter（name, description 必填）+ Markdown body
- 项目级覆盖用户级（同名技能以项目级为准）
- 发现的 Python 技能自动注册到 `skill_registry`
- 增量注册（已存在则跳过）

**新增文件**: `services/skill_discovery.rs`

**依赖**: P0（UnifiedTool trait 定义技能元数据格式）

---

### P4: LLM/Agent 集成

**目标**: 将 Skills 桥接到 Rig Agent 的 Tool 系统，使 LLM 可以通过 Function Call 调用技能。

**需要实现**:
- `SkillToolAdapter<T: UnifiedTool>` — 实现 `rig::tool::Tool` trait 的适配器
- 映射关系：
  - `UnifiedTool::name()` → `Tool::NAME`
  - `UnifiedTool::input_schema()` → `ToolDefinition::parameters`
  - `UnifiedTool::invoke()` → `Tool::call()`
- 扩展 `LlmProviderService::create_agent` 支持 `.tool()` 注册

**当前状态**: `create_agent()` 方法只调用 `.preamble().temperature().build()`，没有 `.tool()` 调用

**新增文件**: `services/skill_tool_adapter.rs`

**依赖**: P0（UnifiedTool trait）、rig-core 0.32.0 的 Tool trait

---

### P5: 5 个内置 Skills

**目标**: 用 Rust 原生实现 5 个内置技能。

| 技能 | 说明 | 依赖 crate |
|------|------|-----------|
| `office.read_write` | Word/Excel 读写和模板渲染 | `docx-rs 0.4`, `rust_xlsxwriter 0.79`, `calamine 0.26` |
| `ocr.extract` | OCR 文字提取（复用现有 OcrService） | 现有 `services/ocr.rs` |
| `image.preprocess` | 图片灰度化、去噪、纠偏、缩放 | `image 0.25` |
| `math.compute` | 数学表达式求值 | 新增 `meval` 或手写解析器 |
| `export.render` | 导出渲染（docx/xlsx 格式） | `docx-rs 0.4`, `rust_xlsxwriter 0.79` |

**新增文件**:
- `services/builtin_skills/mod.rs`
- `services/builtin_skills/office_read_write.rs`
- `services/builtin_skills/ocr_extract.rs`
- `services/builtin_skills/image_preprocess.rs`
- `services/builtin_skills/math_compute.rs`
- `services/builtin_skills/export_render.rs`

**依赖**: P0（每个技能实现 UnifiedTool trait）

---

### P6: Skill Store（技能商店）

**技术方案参考**: §7.3

**目标**: 支持第三方技能的安装、版本管理和审计。

**需要实现**:
- `install_from_git()` — 从 Git 仓库克隆并安装技能
- `update_skill()` — 更新到指定版本
- `uninstall_skill()` — 软删除注册记录
- 安装过程：克隆 → 解析 SKILL.md → 创建 Python venv → 安装依赖 → 注册
- 所有操作写入审计日志
- 安装来源限制白名单

**新增文件**: `services/skill_store.rs`, `commands/skill_store.rs`

---

## 安全约束（来自技术方案）

- AI/Skills/MCP **禁止删除文件**
- **禁止技能安装系统级包**
- **禁止写入白名单外路径**
- 所有安装与执行写入审计日志
- 子进程执行：禁交互、超时、资源限制
- 安装来源限制白名单域名

## 实施优先级

```
P0 (UnifiedTool) → P1 (Executor) → P2 (Seed Data) → P4 (Rig Integration) → P5 (Built-in Skills) → P3 (Discovery) → P6 (Store)
```

P3（自动发现）和 P6（技能商店）可在 P2 之后并行实施。
