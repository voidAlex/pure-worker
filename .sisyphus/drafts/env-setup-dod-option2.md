# Draft: 环境配置 DoD（按方案 2 调整）

## Requirements (confirmed)
- 按“处理方式 2（调整验收口径）”执行：
  - 团队统一使用 pnpm，不强制安装/配置 yarn。
  - `pnpm tauri info` 的验收移动到 M1 E-002（初始化 Tauri 工程）阶段。
  - 第 1 章验收以“国内源配置完成”为主，工程级实证可在工程初始化后补齐。
- 调整完成后：需要提交一次仓库（一次 commit）。

## User Confirmations (this session)
- 分支策略：提交到当前分支（不新建分支）。
- Commit message 风格：`docs: 调整环境DoD`（docs 前缀）。
- 文档改动范围：同时更新 `doc/development-plan-v1.0.md` 与 `doc/env-setup-record.md`，保持一致。
 - README：不需要同步更新。

## Technical Decisions
- 仅修改文档（`doc/*.md`）来反映验收口径调整；不做代码/工程初始化动作。

## Evidence / Current State Observed
- 已生成环境初始化记录：`doc/env-setup-record.md`
- 当前仓库尚未初始化 Tauri 工程：缺少 `apps/desktop/`（因此无法在仓库内执行 `pnpm tauri info`）。

## Scope Boundaries
- INCLUDE:
  - 更新 `doc/development-plan-v1.0.md` 第 1 章 DoD 表述与注释
  - 更新 `doc/env-setup-record.md` 的验收清单与备注，反映新的验收口径
  - 生成一次 commit（仅包含上述文档改动）
- EXCLUDE:
  - 初始化工程骨架（E-001/E-002）
  - 安装/配置 yarn
  - 为 `cargo fetch` / `pnpm tauri info` 做工程级实证（延后到相关里程碑任务）

## Open Questions

### CRITICAL
 - 你希望“本地 + 所有远端”（原话：本地+所有远端）。
   - 需要进一步明确：具体要 push 哪些 remote、以及 push 的目标分支。

### Minor
- 是否需要同步更新 `README.md`，在“快速开始”里补一句“团队统一 pnpm（yarn 可不配）”？（可选）
