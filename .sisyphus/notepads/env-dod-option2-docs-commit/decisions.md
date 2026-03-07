# Decisions (append-only)

## 2026-03-07 DoD 验收口径调整（方式 2）

### 决策背景
原 DoD 要求 `pnpm tauri info` 在第 1 章环境验收时通过，但该命令依赖 Tauri CLI 已安装且项目已初始化，在纯环境配置阶段无法执行。

### 采用方案
方式 2（调整验收口径）：将 `pnpm tauri info` 从第 1 章 DoD 延后至 M1 E-002 验收。

### 修改内容
1. **1.2 节标题**：`npm/pnpm/yarn` → `pnpm 为主，npm 备选`，移除 yarn 配置命令
2. **1.7 DoD 第 1 条**：`npm/pnpm/yarn registry 全部指向国内源` → `pnpm（及备选 npm）registry 指向国内源（不要求 yarn）`
3. **1.7 DoD 第 4 条**：`pnpm tauri info 成功执行` → 添加"延后至 M1 E-002 验收"
4. **2.1 E-002**：添加明确验收项 `pnpm tauri info 成功执行`

### 理由
- pnpm 为团队统一选择，npm 仅作备选，不强制 yarn
- `pnpm tauri info` 需要 Tauri 项目存在，属于工程初始化验收而非环境配置验收

## 2026-03-07 env-setup-record.md 口径一致化

### 修改目的
与 development-plan-v1.0.md 更新后的 DoD 口径保持一致。

### 具体修改
- 验收清单第 1 项：明确 pnpm 为主、npm 备选、不要求 yarn

### 验证命令结果
- `grep -n "yarn" doc/env-setup-record.md`：2 处，均为"不要求/无需配置"说明
- `grep -n "pnpm tauri info" doc/env-setup-record.md`：2 处，均为"M1 E-002 验收"说明
