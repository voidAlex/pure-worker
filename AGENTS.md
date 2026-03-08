# AGENTS.md — PureWorker

**Updated:** 2026-03-08
**Commit:** da81777
**Branch:** master

> 面向教师场景的本地优先桌面 AI 助手（Tauri 2.x + Rust + React/TypeScript + SQLite）。
> 许可证：AGPL-3.0，所有贡献必须遵守。

## 项目状态

代码已落地，M1 工程底座开发中。

## 目录结构

```
pure-worker/
├── apps/desktop/          # Tauri 2 应用（React 前端 + Rust 后端）
│   ├── src/               # React + TypeScript UI（6页面 + 组件）
│   ├── src-tauri/         # Rust 后端（30+ IPC 命令）
│   └── AGENTS.md          # 应用级详细文档
├── packages/prompt-templates/  # 版本化提示词模板（待实现）
├── doc/                   # PRD、技术方案、开发计划
└── AGENTS.md              # 本文件
```

## 构建与运行命令

```bash
# 前端开发（在 apps/desktop/ 目录下执行）
pnpm install
pnpm tauri dev              # 启动 Tauri 开发模式（前端 + 后端）
pnpm tauri build            # 生产环境构建

# Rust 后端（在 apps/desktop/src-tauri/ 目录下执行）
cargo build                 # 构建 Rust 后端
cargo clippy -- -D warnings # Rust 代码静态检查（警告视为错误）
cargo fmt --check           # 检查 Rust 代码格式
cargo test                  # 运行所有 Rust 测试
cargo test <测试名称>        # 运行单个测试
cargo test -- --nocapture   # 运行测试并显示标准输出

# 前端静态检查/格式化（在 apps/desktop/ 目录下执行）
pnpm eslint src/            # TypeScript/React 静态检查
pnpm prettier --check src/  # 检查代码格式
pnpm tsc --noEmit           # 仅做类型检查，不生成产物

# 完整检查（提交前必须执行）
cargo fmt --check && cargo clippy -- -D warnings && cargo test
pnpm eslint src/ && pnpm prettier --check src/ && pnpm tsc --noEmit
```

## 环境配置

所有依赖**必须**使用国内镜像源，详见 `doc/development-plan-v1.0.md` 第 1 章。

- **Rust**：>= 1.77.2 stable，Cargo 通过 `rsproxy.cn` 加速
- **Node.js**：>= 20 LTS，优先使用 pnpm，registry 设为 `registry.npmmirror.com`
- **Python**：>= 3.10，由 `uv` 管理，PyPI 镜像 `mirrors.aliyun.com`
- **Tauri CLI**：2.x

## 代码风格 — TypeScript / React

### 格式化与静态检查
- 使用 ESLint + Prettier（工程初始化后配置文件放在项目根目录）
- 严格 TypeScript：tsconfig 中设置 `strict: true`
- 禁止 `any` — 不允许使用 `as any`、`@ts-ignore` 或 `@ts-expect-error`

### 导入规范
- 配置路径别名后使用别名导入（如 `@/components/...`）
- 导入分组顺序：1) React/第三方库 2) 内部模块 3) 类型 4) 样式
- 优先使用命名导出，避免默认导出

### 命名规范
- 组件：`PascalCase`（文件名与符号名）
- Hooks：`useCamelCase`
- 工具函数/服务：`camelCase`
- 常量：`UPPER_SNAKE_CASE`
- 类型/接口：`PascalCase`，不加 `I` 前缀

### 错误处理
- 所有面向用户的错误消息必须使用中文自然语言
- 禁止空 catch 块 — 必须处理或重新抛出
- 使用后端返回的结构化错误响应（业务错误码 + 中文提示信息）

### UI 约束（来自 PRD）
- 三栏布局，面板可折叠（左侧 220/64px，右侧 360px）
- 图标**必须**附带中文文字标签 — 禁止纯图标按钮
- 所有 AI 输出默认以草稿态渲染（需教师确认后生效）
- 通知使用底部 Toast — 禁止侵入式弹窗对话框
- 进度展示：步骤名称 + 三态指示器 + 预计剩余时间

## 代码风格 — Rust

### 格式化与静态检查
- 使用 `rustfmt` 格式化（默认配置）
- `clippy` 警告视为错误：`cargo clippy -- -D warnings`
- 生产代码中禁止 `#[allow(unused)]` — 删除无用代码

### 命名规范
- 模块/文件：`snake_case`
- 结构体/枚举/Trait：`PascalCase`
- 函数/方法：`snake_case`
- 常量：`UPPER_SNAKE_CASE`

### 错误处理
- 使用 `thiserror` 定义错误类型
- 使用 `?` 传播错误 — 生产代码路径禁止 `.unwrap()`
- IPC 命令返回 `Result<T, AppError>`，附带结构化错误码

### 数据库（SQLite）
- 连接初始化**必须**执行：`WAL`、`synchronous=NORMAL`、`busy_timeout=5000`、`foreign_keys=ON`
- `foreign_keys=ON` 必须逐连接设置（在连接池钩子中），不能仅在迁移脚本中设置一次
- 所有表使用软删除（`is_deleted INTEGER NOT NULL DEFAULT 0`）
- 所有业务查询**必须**包含 `WHERE is_deleted=0`
- 主键：TEXT 类型（UUID），不使用自增整数
- 时间戳：TEXT 类型，ISO 8601 格式

### 文件操作
- 所有文件 I/O 通过 Rust 服务层完成（`FileService`、`MemoryService`、`DocService`）
- **禁止**使用 shell 命令执行文件操作
- AI/Skills/MCP **禁止**删除文件（不提供 delete/remove/rm 能力）

## 架构原则

1. **本地优先**：学生数据保存在本地磁盘，默认不上传云端。
2. **人审闭环**：所有 AI 输出均为草稿，教师确认后才可采纳/导出。
3. **前后端职责分离**：前端仅负责渲染与交互。所有 LLM 调用、Agent 编排和工具执行均在 Rust 后端通过 Rig 完成。
4. **默认安全**：API 密钥存入系统密钥链（禁止明文存储）。文件访问受路径白名单限制。所有关键操作写入审计日志。
5. **白盒执行**：长任务提供步骤级进度可视化。
6. **禁删红线**：AI、Skills 和 MCP 工具不允许删除文件。

## IPC 约束

- 文件拖拽：前端仅发送文件路径/句柄元数据，**禁止**传输 Base64 编码内容
- 二进制数据保留在本地文件系统 — IPC 仅传输轻量参数（job_id、路径、配置项）
- IPC 命令按权限域划分（设置、档案、任务、审批、导出）

## Agent 与工具设计

- 每个 Agent 单次请求仅暴露 5-8 个工具（防止 LLM 认知过载）
- 不同 Agent 使用隔离的工具集 — 禁止将 30+ 个工具全部暴露
- 对 LLM 暴露宏观业务工具；细粒度工具保留在 Rust 编排层内部
- 提示词模板：版本化文件存放在 `packages/prompt-templates/`，配有变量白名单

## 安全规则

- 高危操作（发送、覆盖写入、批量修改）必须经过用户明确确认
- 外发前默认启用脱敏（可配置开关）
- 路径白名单：仅允许访问工作区白名单目录，越权访问直接拒绝
- 安装操作（如 uv install）必须由用户主动触发 — 禁止静默安装
- 安装来源限制在白名单域名（默认：`astral.sh`）
- 所有安装与修复操作记录到审计日志

## 关键参考文档

| 文档 | 路径 | 说明 |
|------|------|------|
| 产品需求 | `doc/prd-v1.0.md` | 功能需求、UI 规范 |
| 技术方案 | `doc/tech-solution-v1.0.md` | 架构设计、技术选型 |
| 开发计划 | `doc/development-plan-v1.0.md` | 里程碑任务分解 |

- 里程碑顺序：M1（工程底座）→ M2（教务管理）→ M3（家校沟通）→ M4（作业考评）→ M5（扩展收口）

## Git 规范

- 分支策略与提交规范在工程初始化时定义（任务 E-004）
- 禁止提交：`.env`、`*.key`、`*.pem`、`credentials.json`、`secrets.*`
- 禁止提交：`node_modules/`、`target/`、`dist/`、`*.db`

## 要求

- 给用户回答问题必须使用中文，完成工作后的交付总结也必须用中文
- 用户发出疑问时，必须分析后给用户解决方案的选择，严禁未经确认直接修改文档或代码
- 用户让开始编码前，必须找到当前计划中需要用户澄清的问题，待用户确认后才可开始编码
- 如果用户让排查问题，排查顺序是日志--源码，严禁不通过排查日志和源码直接进行修改，如果缺少问题依据，则需要在关键路径增加日志输出，并告诉用户再次复现；如果是前端无法自行获取日志，则要引导用户告知日志
- 代码必须添加方法级和文件级的中文注释
- 每次对话完后，在结尾加一个“喵~”
