# 按“方式 2”调整环境 DoD（文档一致化 + 单次提交并推送）

## TL;DR
> **目标**：按你选定的“方式 2（调整验收口径）”处理第 1 章环境配置 DoD：明确团队统一 pnpm、移除 yarn 强制要求、将 `pnpm tauri info` 验证迁移到 M1 E-002，并保持 `doc/env-setup-record.md` 与计划一致；最后在**当前分支**做**一次** commit，并 push 到**所有已配置 remote**。

**交付物**：
- 更新 `doc/development-plan-v1.0.md`（第 1 章 DoD 与说明）
- 更新 `doc/env-setup-record.md`（验收清单与备注对齐新的口径）
- 1 次 git commit：`docs: 调整环境DoD`
- push：对 `git remote -v` 列出的所有 remote，推送当前分支同名分支

**估算**：短（文档改动 + git 操作）

---

## Context

### 原始请求
- “按 2 处理…处理完后提交一次仓库”

### 已确认决策（本次会话）
- 采用“方式 2（调整验收口径）”：
  - 团队统一使用 pnpm，不强制安装/配置 yarn。
  - `pnpm tauri info` 的验收从第 1 章移到 M1 E-002（初始化 Tauri 工程）阶段。
  - 第 1 章验收以“国内源配置完成”为主，工程级实证可在工程初始化后补齐。
- 文档改动范围：仅两份文档一致化
  - `doc/development-plan-v1.0.md`
  - `doc/env-setup-record.md`
- 不需要更新 README。
- Git：当前分支提交（不新建分支），单次 commit，commit message 固定 `docs: 调整环境DoD`。
- Push：推送到**所有已配置 remote**，目标为**当前分支同名**。

### Metis Review（已吸收）
Metis 提醒的关键风险点（计划中已加入对应 guardrails/验收）：
- “方式 2”的目标文本需可执行/可验收（用可 grep 的关键句/条目）
- push 到多个 remote 的失败策略需提前定义
- 必须锁死变更范围（仅两份 doc），防止 scope creep

---

## Work Objectives

### Core Objective
把第 1 章“配置开发环境”的验收口径从“强制全生态/工程实证”调整为“pnpm-only + 配置完成为主 + 工程级验证后移”，并保证文档一致性与可执行验收。

### Must Have
- 第 1 章 DoD：
  - 不再要求 yarn 必须安装/配置
  - 明确团队统一 pnpm
  - `pnpm tauri info` 不再是第 1 章 DoD 必须项，迁移到 M1 E-002（必须明确写入 E-002 的验收或子条目）
- `doc/env-setup-record.md` 与上述口径一致（避免相互矛盾）
- 单次提交 + push 到所有 remote

### Must NOT Have（Guardrails）
- 不得新增/初始化任何工程目录（例如 `apps/desktop/`）
- 不得增加或要求安装 yarn
- 不得修改 README 或其他 doc（除非你后续明确允许）
- 不得产生“需要人工手动打开检查”的验收项；验收必须命令/文本可验证
- 不得使用 `--no-verify` 跳过 git hooks（除非你明确要求）
- 不得执行 `git pull --rebase` / `git push --force` 等改写历史动作（本任务仅文档更新，默认不需要）

---

## Verification Strategy

本任务是“文档一致化 + Git 操作”，验收以 **git diff 范围** + **关键文本断言** + **commit/push 结果** 为准。

### 验收命令（必须逐条执行并记录输出）

1) **确认当前分支与 remotes（防误推）**
```bash
git branch --show-current
git remote -v
```
断言：
- 分支名非空（非 detached HEAD）
- remote 列表明确（后续将对每个 remote 推送）

2) **变更范围锁定**
```bash
git diff --name-only
```
断言：仅包含：
- `doc/development-plan-v1.0.md`
- `doc/env-setup-record.md`

3) **关键文本断言（方式 2 已落地）**
```bash
rg -n "pnpm tauri info" doc/development-plan-v1.0.md
rg -n "yarn" doc/development-plan-v1.0.md doc/env-setup-record.md
```
断言：
- 第 1 章 DoD 不再要求 `pnpm tauri info` 当场通过；必须明确迁移到 M1 E-002（或写明“工程初始化后验证”）
- 不出现“必须安装/配置 yarn”的硬性要求（允许提到“若使用 yarn 则可配置”，但不得作为 DoD 必须项）

补充断言（关键）：
- M1 的 E-002（初始化 Tauri 工程）必须新增一条“`pnpm tauri info` 成功执行”的验收（或等效的、可执行命令验收）。

4) **提交断言**
```bash
git status --porcelain
git add doc/development-plan-v1.0.md doc/env-setup-record.md
git commit -m "docs: 调整环境DoD"
git log -1 --pretty=%B
```
断言：
- commit message 精确等于：`docs: 调整环境DoD`
- commit 只包含两份 doc 的改动

5) **push 到所有 remote（同名分支）**
> 使用“尽力推送（best-effort）”策略：某个 remote push 失败不阻断其余 remote，但必须记录失败的 remote 名称与错误原因。
> 注意：如果 remote 是只读（如 upstream）或受保护分支，push 失败是预期情况；记录即可。

```bash
BRANCH=$(git branch --show-current)
for r in $(git remote); do
  echo "\n== pushing to $r ($BRANCH) =="
  git push "$r" "HEAD:$BRANCH" || echo "PUSH_FAILED:$r";
done
```

断言：
- 至少 `origin` 推送成功（如存在 origin）
- 如果存在失败 remote：在输出中能看到 `PUSH_FAILED:<remote>`，并将失败原因记录到执行日志中

---

## Execution Strategy (Parallel Waves)

Wave 1（可并行，但建议顺序执行以减少误差）：
- Task 1：修改 `doc/development-plan-v1.0.md`（口径调整）
- Task 2：修改 `doc/env-setup-record.md`（一致化）

Wave 2（顺序）：
- Task 3：文档验收（rg + git diff 范围锁定）
- Task 4：单次提交（固定 message）
- Task 5：推送到所有 remote（best-effort）

---

## TODOs

- [ ] 1. 调整 `doc/development-plan-v1.0.md`：按“方式 2”改写第 1 章 DoD

  **What to do**:
  - 在 `## 1.7 环境验收（DoD）` 处（当前版本包含：
    - `npm/pnpm/yarn` registry 全部指向国内源
    - `cargo fetch` ...
    - `uv python install` ...
    - `pnpm tauri info` 成功执行
    ）做如下改写：
    - 把第 1 条改为：`npm/pnpm` registry 指向国内源（**不要求 yarn**）。
    - 把 `pnpm tauri info` 这条从第 1 章 DoD 删除，替换为：`pnpm tauri info` 将在 M1 `E-002`（初始化 Tauri 工程）阶段验收。
  - 在 `## 2.1 仓库与工程骨架` 的 `E-002 初始化 Tauri 2 + React + TypeScript 工程。` 下面新增子条目或在“验收”里补一句：
    - `pnpm tauri info` 成功执行。
  - 增加一句“口径说明”（可放在 1.7 DoD 下方的注释）：第 1 章以镜像/国内源配置为主；工程级可执行验证在工程初始化后补齐。

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 仅涉及两份 Markdown 文档的口径调整与一致化。
  - **Skills**: `[]`

  **Acceptance Criteria**:
  - [ ] `rg -n "yarn" doc/development-plan-v1.0.md` 不包含“必须安装/配置 yarn”的要求
  - [ ] `rg -n "pnpm tauri info" doc/development-plan-v1.0.md` 能看到明确迁移到 M1 E-002 的说明（且 E-002 处包含该验收）

  **QA Scenarios**:
  ```
  Scenario: 文档口径已按方式 2 调整
    Tool: Bash
    Steps:
      1. rg -n "环境验收（DoD）" doc/development-plan-v1.0.md
      2. rg -n "yarn" doc/development-plan-v1.0.md
      3. rg -n "pnpm tauri info" doc/development-plan-v1.0.md
    Expected Result:
      - DoD 不再要求 yarn
      - pnpm tauri info 的验证迁移到 M1 E-002
    Evidence: .sisyphus/evidence/task-1-doc-dod-check.txt
  ```

- [ ] 2. 更新 `doc/env-setup-record.md`：与新的 DoD 口径一致化

  **What to do**:
  - 在“验收清单”中明确：yarn 未安装不构成阻塞（因团队统一 pnpm）。
  - `pnpm tauri info` 仍可保留为“待工程初始化后验证”的条目（与 development-plan 对齐）。

  **Recommended Agent Profile**:
  - **Category**: `writing`
    - Reason: 仅 Markdown 文档一致化。
  - **Skills**: `[]`

  **Acceptance Criteria**:
  - [ ] `rg -n "yarn" doc/env-setup-record.md` 不出现“必须安装 yarn”的验收项
  - [ ] `rg -n "pnpm tauri info" doc/env-setup-record.md` 与 development-plan 表述一致（均为“工程初始化后验证”）

  **QA Scenarios**:
  ```
  Scenario: 环境记录与计划一致
    Tool: Bash
    Steps:
      1. rg -n "验收清单" doc/env-setup-record.md
      2. rg -n "pnpm tauri info" doc/env-setup-record.md
      3. rg -n "yarn" doc/env-setup-record.md
    Expected Result:
      - 记录文档不与计划冲突
    Evidence: .sisyphus/evidence/task-2-doc-record-check.txt
  ```

- [ ] 3. 变更范围锁定（只允许两份文档）

  **What to do**:
  - 在提交前执行 `git diff --name-only`，确认变更范围没有扩散。

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 单命令验证范围。
  - **Skills**: `[]`

  **Acceptance Criteria**:
  - [ ] `git diff --name-only` 仅输出两份文件

  **QA Scenarios**:
  ```
  Scenario: 变更范围未扩散
    Tool: Bash
    Steps:
      1. git diff --name-only
    Expected Result:
      - 输出严格为 doc/development-plan-v1.0.md + doc/env-setup-record.md
    Evidence: .sisyphus/evidence/task-3-diff-name-only.txt
  ```

- [ ] 4. 单次提交（固定 message）

  **What to do**:
  - `git add` 两份文档
  - `git commit -m "docs: 调整环境DoD"`

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 标准 git 操作。
  - **Skills**: `["git-master"]`
    - `git-master`: 避免误操作，确保单次提交与消息一致。

  **Acceptance Criteria**:
  - [ ] `git log -1 --pretty=%B` 输出精确为 `docs: 调整环境DoD`

  **QA Scenarios**:
  ```
  Scenario: commit message 与内容正确
    Tool: Bash
    Steps:
      1. git show --name-only --pretty=oneline -1
      2. git log -1 --pretty=%B
    Expected Result:
      - 只包含两份 doc
      - message 精确匹配
    Evidence: .sisyphus/evidence/task-4-commit-check.txt
  ```

- [ ] 5. push 到所有 remote（当前分支同名，best-effort）

  **What to do**:
  - 先运行：`git branch --show-current` 与 `git remote -v`
  - 再循环 `git push <remote> HEAD:<branch>`
  - 记录任何失败 remote 与原因

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 机械性推送流程。
  - **Skills**: `["git-master"]`
    - `git-master`: 对多 remote push 的失败处理更稳健。

  **Acceptance Criteria**:
  - [ ] 对 `git remote` 列出的每个 remote 都尝试 push
  - [ ] 若存在失败 remote，输出中包含 `PUSH_FAILED:<remote>` 并记录错误

  **QA Scenarios**:
  ```
  Scenario: 推送到所有已配置 remote
    Tool: Bash
    Steps:
      1. git branch --show-current
      2. git remote -v
      3. 执行推送循环脚本
    Expected Result:
      - 每个 remote 都有 push 尝试日志
    Evidence: .sisyphus/evidence/task-5-push-all-remotes.txt
  ```

---

## Final Verification Wave

- [ ] F1. 文档一致性审计（deep）
  - 检查两份文档的验收口径互不矛盾
  - 检查不存在“必须 yarn”的强制要求
  - 检查 `pnpm tauri info` 迁移说明明确且可执行

---

## Commit Strategy

- 单次提交：`docs: 调整环境DoD`
  - Files:
    - `doc/development-plan-v1.0.md`
    - `doc/env-setup-record.md`

---

## Success Criteria

- 两份文档口径一致：pnpm-only、yarn 非必需、`pnpm tauri info` 后移到 M1 E-002
- git diff 变更范围仅限两份 doc
- 存在且仅存在 1 次 commit，message 精确匹配
- 对所有 `git remote` 都进行了 push 尝试并记录结果
