# 工作目录结构规范（安装后）

本文档定义 PureWorker 在用户选择工作目录后的标准落盘结构。

## 目标

- 所有运行时数据优先落在用户选择的工作目录
- 数据库、日志、Skills 插件包、教学工作文件分层管理
- 兼容旧版本（历史 AppData 数据）

## 根目录约定

假设用户选择的工作目录为 `<workspace>`：

```text
<workspace>/
├─ .pureworker/
│  ├─ db/
│  │  └─ pureworker.db
│  └─ skills/
│     └─ builtin/
│        ├─ builtin-skills.json
│        ├─ README.md
│        ├─ office-read-write/
│        │  ├─ SKILL.md
│        │  └─ package.json
│        └─ ... 其余内置 skills
├─ .agents/
│  └─ skills/                # 第三方/项目级 skills 安装目录
├─ logs/
│  └─ startup.log
├─ students/                 # 学生相关工作文件
├─ archives/                 # 归档目录
├─ templates/                # 模板目录
├─ exports/                  # 导出目录
└─ imports/                  # 导入目录
```

## 目录职责

- `.pureworker/db`：SQLite 主库与附属文件（`-wal/-shm`）
- `logs`：应用启动与运行日志
- `.pureworker/skills/builtin`：系统内置 Skills 的文件化插件包（初始化时自动解包）
- `.agents/skills`：用户安装/项目级 Skills
- `students/archives/templates/exports/imports`：教学工作文件目录

## 兼容策略

- 首次选择工作目录后，后续启动优先从工作目录读取数据库与日志
- 若检测到历史 AppData 数据库且工作目录数据库不存在，启动阶段自动迁移
- 内置 Skills 会在工作目录初始化时自动解包，后续启动按最新内置模板同步
- 已有业务设置（如 `workspace_path`）继续保留，不破坏旧版本行为
