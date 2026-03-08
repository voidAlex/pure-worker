# PureWorker Desktop App

**Generated:** 2026-03-08
**Commit:** da81777
**Branch:** master

## 概述

Tauri 2.x 桌面应用，前端 React + TypeScript，后端 Rust + SQLite。

## 目录结构

```
apps/desktop/
├── src/                    # React 前端
│   ├── pages/              # 页面组件（6个）
│   ├── components/         # UI 组件
│   │   ├── layout/         # 布局组件（AppLayout, Sidebar, AiPanel, StatusBar）
│   │   └── shared/         # 共享组件（Toast, ConfirmDialog, EmptyState）
│   ├── hooks/              # 自定义 Hooks
│   ├── App.tsx             # 根组件（路由 + QueryClient）
│   ├── main.tsx            # 入口文件
│   └── bindings.ts         # Tauri 自动生成的 IPC 绑定（19KB）
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── commands/       # IPC 命令处理器（15个文件，671行）
│   │   ├── services/       # 业务服务层（11个文件，1904行）
│   │   ├── models/         # 数据模型（12个文件）
│   │   ├── database/       # 数据库初始化与迁移
│   │   ├── error.rs        # 错误类型定义（AppError）
│   │   ├── lib.rs          # 库入口，注册 30+ IPC 命令
│   │   └── main.rs         # 二进制入口
│   ├── migrations/         # SQL 迁移脚本
│   ├── capabilities/       # Tauri 权限配置
│   └── Cargo.toml          # Rust 依赖
├── package.json            # 前端依赖与脚本
├── tsconfig.json           # TypeScript 配置（strict mode）
├── eslint.config.js        # ESLint 配置
└── .prettierrc             # Prettier 配置
```

## 入口点

| 文件 | 说明 |
|------|------|
| `src/main.tsx` | React 入口，`startApp()` 挂载到 `#root` |
| `src/App.tsx` | 根组件，配置 TanStack Query + React Router |
| `src-tauri/src/main.rs` | Rust 二进制入口，调用 `pure_worker_lib::run()` |
| `src-tauri/src/lib.rs` | Tauri 应用初始化，数据库连接池，IPC 命令注册 |

## IPC 命令（30+）

按业务域划分：

| 域 | 命令 | 文件 |
|----|------|------|
| **设置** | `get_app_settings` | commands/settings.rs |
| **教师档案** | `get_teacher_profile` | commands/profile.rs |
| **班级** | `list/create/update/delete_classroom`, `get_classroom` | commands/classroom.rs |
| **学生** | `list/create/update/delete_student`, `get_student`, `get_student_profile_360` | commands/student.rs |
| **学生标签** | `list/add/remove/update_student_tag` | commands/student_tag.rs |
| **成绩** | `list/create/update/delete_score_record` | commands/score_record.rs |
| **观察记录** | `list/create/update/delete_observation_note` | commands/observation_note.rs |
| **家校沟通** | `list/create/update/delete_parent_communication` | commands/parent_communication.rs |
| **日程事件** | `list/get/create/update/delete_schedule_event` | commands/schedule_event.rs |
| **日程文件** | `list/create/delete_schedule_file` | commands/schedule_file.rs |
| **导入** | `import_students` | commands/student_import.rs |
| **审批** | `list_pending_approvals` | commands/approval.rs |
| **导出** | `health_check` | commands/export.rs |
| **任务** | `list_tasks` | commands/task.rs |

## 前端路由

| 路径 | 页面 | 说明 |
|------|------|------|
| `/` | DashboardPage | 仪表盘 |
| `/classes` | ClassesPage | 班级管理 |
| `/students` | StudentsPage | 学生列表 |
| `/students/:id` | StudentDetailPage | 学生详情（360档案） |
| `/import` | ImportPage | 学生导入 |
| `/schedule` | SchedulePage | 日程管理 |

## 数据库配置（SQLite）

连接池初始化必须执行（见 `src-tauri/src/database/mod.rs`）：
- `journal_mode = WAL`
- `synchronous = NORMAL`
- `busy_timeout = 5000ms`
- `foreign_keys = ON`（每连接生效）
- `max_connections = 5`

## 开发命令

```bash
# 前端开发
pnpm dev                    # Vite 开发服务器
pnpm build                  # tsc && vite build
pnpm lint                   # ESLint 检查
pnpm format                 # Prettier 格式化
pnpm typecheck              # TypeScript 类型检查

# Tauri 开发
pnpm tauri dev              # 启动完整应用（前端 + 后端）
pnpm tauri build            # 生产构建

# Rust 后端
cd src-tauri
cargo build                 # 构建
cargo clippy -- -D warnings # 静态检查
cargo fmt --check           # 格式检查
cargo test                  # 运行测试
```

## 关键约定

### 前端
- 路径别名：`@/` → `src/`
- 状态管理：Zustand
- 数据请求：TanStack Query
- 样式：Tailwind CSS 4.x
- 图标：Lucide React

### 后端
- 错误处理：`thiserror` 定义 `AppError`
- 数据库：`sqlx` + SQLite + 宏
- IPC 绑定：`tauri-specta` 自动生成 TypeScript

### TypeScript 绑定
`src/bindings.ts` 由 `tauri-specta` 自动生成，开发模式下每次启动自动更新。

## 注意事项

- **bindings.ts 体积大**（19KB）：包含所有 IPC 命令的类型绑定，暂不拆分
- **测试缺失**：前端无测试脚本，依赖 Rust 测试（`cargo test`）
- **CI/CD 缺失**：项目尚未配置 GitHub Actions
