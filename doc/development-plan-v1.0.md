# PureWorker 开发计划（v1.0 / 基于 tech-solution-v1.0）

> 目标：将 `doc/tech-solution-v1.0.md` 拆分为可执行、可排期、可验收、可跟踪的细粒度开发任务。  
> 范围：一期 MVP（模块 5、模块 3、模块 4.2、模块 4.3）。  
> 原则：本地优先、白盒执行、人审闭环、默认安全、先工程底座后业务闭环。

---

## 0. 交付约束与执行口径

### 0.1 约束（全程生效）
- 所有安装依赖默认使用国内源/国内镜像（含 Node、Rust、Python、Tauri 相关下载）。
- AI/Skills/MCP 禁止删除文件能力（禁止 `delete/remove/rm` 能力暴露）。
- 所有高危动作（外发、覆盖写入、批量改写）必须进入人工确认闸门。
- 默认本地存储，不明文存储密钥，所有关键操作写审计日志。

### 0.2 任务状态规范
- 状态：`todo -> in_progress -> blocked -> done`
- 阻塞必须记录：阻塞原因、影响范围、临时方案、解除条件。
- 每个任务必须绑定：输入、输出、验收标准、依赖关系。

### 0.3 里程碑基线（12 周）
- M1（第 1-2 周）：工程底座
- M2（第 3-5 周）：基础教务管理（模块 3）
- M3（第 6-8 周）：班务与家校沟通（模块 4.3）
- M4（第 9-10 周）：作业/考评与题库（模块 4.2）
- M5（第 11-12 周）：扩展收口与验收（模块 5 收口）

---

## 1. 第一步：配置开发环境（所有依赖使用国内源）

> 本节为项目 0 号任务，必须先完成再进入任何编码任务。

## 1.1 环境基线版本
- Rust：`>= 1.77.2`（建议 stable 最新）
- Node.js：`>= 20`（建议 LTS）
- 包管理：pnpm / npm（二选一为主，团队统一）
- Python：`>= 3.10`（由 uv 管理）
- Tauri CLI：2.x

## 1.2 Node 生态国内源配置（pnpm 为主，npm 备选）
```bash
# pnpm 为主
pnpm config set registry https://registry.npmmirror.com

# npm 作为备选
npm config set registry https://registry.npmmirror.com

# 验证
pnpm config get registry
npm config get registry
```

## 1.3 Rust 生态国内源配置（rustup/cargo）

在 shell 配置文件（`~/.zshrc` 或 `~/.bashrc`）增加：

```bash
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
```

配置 `~/.cargo/config.toml`：

```toml
[source.crates-io]
replace-with = "rsproxy-sparse"

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true
```

验证：
```bash
rustup show
cargo fetch
```

## 1.4 Python/uv 国内源配置
```bash
# uv 使用国内 PyPI 源
export UV_DEFAULT_INDEX="https://mirrors.aliyun.com/pypi/simple/"

# pip 使用国内源（兼容兜底）
pip config set global.index-url https://mirrors.aliyun.com/pypi/simple/

# uv 拉取 Python 构建包走 GitHub 加速代理
export UV_PYTHON_INSTALL_MIRROR="https://ghproxy.com/https://github.com/astral-sh/python-build-standalone/releases/download"

# 验证
uv --version
uv python install 3.12
```

## 1.5 Tauri 相关下载加速
```bash
export TAURI_BUNDLER_TOOLS_GITHUB_MIRROR="https://ghproxy.com/https://github.com"
```

说明：该变量用于 Tauri bundler 相关工具下载加速（WebView2/NSIS/WiX 等依赖下载链路）。

## 1.6 可选：macOS Homebrew 国内源（仅 macOS）
```bash
export HOMEBREW_API_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"
export HOMEBREW_BOTTLE_DOMAIN="https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles"
export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
```

## 1.7 环境验收（DoD）
- [x] `pnpm`（及备选 `npm`）registry 指向国内源（不要求 yarn）。
- [x] `cargo fetch` 不走默认 crates.io 公网慢链路。
- [x] `uv python install` 可在可接受时长内完成。
- [x] `pnpm tauri info` 成功执行 → 延后至 M1 E-002 验收。
- [x] 形成《环境初始化记录》文档（机器、系统、版本、源配置、验证截图/日志）。

---

## 2. M1（第 1-2 周）：工程底座

## 2.1 仓库与工程骨架
- [x] E-001 创建目录骨架：`apps/desktop`、`packages/*`、`doc/`。
- [x] E-002 初始化 Tauri 2 + React + TypeScript 工程（验收项：`pnpm tauri info` 成功执行）。
- [x] E-003 统一 lint/format 配置（ESLint/Prettier/Rustfmt/Clippy）。
- [x] E-004 约定分支策略与提交规范（不在本文定义 Git 细则，仅落地模板）。

验收：本地可启动 UI 与 Tauri 外壳，静态检查可运行。

## 2.2 IPC 与后端命令框架
- [x] E-010 建立 `command` 分层：设置、档案、任务、审批、导出。
- [x] E-011 统一错误模型（业务错误码 + 中文可读错误）。
- [x] E-012 前后端类型契约方案（建议 `specta` 或等效机制）。
- [x] E-013 IPC 命令权限清单与最小暴露。

验收：前端可调用后端健康检查命令并返回结构化响应。

## 2.3 SQLite 与迁移体系
- [x] E-020 接入 SQLite 连接池（sqlx 或同类）。
- [x] E-021 连接初始化强制执行：
  - `PRAGMA journal_mode=WAL;`
  - `PRAGMA synchronous=NORMAL;`
  - `PRAGMA busy_timeout=5000;`
  - `PRAGMA foreign_keys=ON;`（逐连接生效）
- [x] E-022 首批迁移：核心表 + 索引 + FTS + 触发器。
- [x] E-023 迁移失败阻断启动（禁止降级到默认模式）。

验收：新库初始化成功；迁移脚本可重复执行且幂等。

## 2.4 任务状态机与审计底座
- [x] E-030 任务状态机：`queued/running/waiting_human/recovering/completed/failed/cancelled`。
- [x] E-031 异步任务持久化字段落地（checkpoint、lease、heartbeat）。
- [x] E-032 审计日志统一写入接口。
- [x] E-033 任务进度白盒结构（步骤、百分比、剩余时间）。

验收：模拟长任务中断后可恢复；审计日志可查询。

---

## 3. M2（第 3-5 周）：基础教务管理（模块 3）

## 3.1 班级/学科/教师关系
- [x] M2-001 班级 CRUD（软删除）。
- [x] M2-002 学科与教师绑定模型。
- [x] M2-003 关系一致性约束（外键/业务校验）。

## 3.2 学生 360 档案
- [x] M2-010 学生 CRUD（含 `student_no + class_id` 唯一性策略）。
- [x] M2-011 标签系统（增删改查 + 有效性校验）。
- [x] M2-012 成绩记录时间序列。
- [x] M2-013 观察记录与沟通历史关联视图。

## 3.3 Excel 批量导入
- [x] M2-020 导入模板定义与字段映射。
- [x] M2-021 行级错误回传（错误行号 + 原因 + 修复建议）。
- [x] M2-022 导入去重与覆盖策略（明确“新增/跳过/更新”）。
- [x] M2-023 导入审计日志（操作者、文件、结果统计）。

## 3.4 日程与课表
- [x] M2-030 课表事件 CRUD。
- [x] M2-031 教案/课件文件关联。
- [x] M2-032 时间冲突检测规则。

验收：完成班级→学生→成绩→观察→课表全链路可操作。

---

## 4. M3（第 6-8 周）：班务与家校沟通（模块 4.3）

## 4.1 记忆检索（Agentic Search）
- [x] M3-001 SQL 精确过滤（学生/班级/标签/时间）。
- [x] M3-002 SQLite FTS 召回（观察/沟通文本）。
- [x] M3-003 文件遍历补充（学生 memory Markdown）。
- [x] M3-004 规则重排（近期优先、学科优先、高置信优先）。
- [x] M3-005 Top-K 证据注入器（防上下文污染）。

## 4.2 学生长期记忆 Markdown 体系
- [x] M3-010 固定目录结构创建：`/workspace/students/{student_id}/memory/`。
- [x] M3-011 月度文件模板与 frontmatter 校验。
- [x] M3-012 固定章节解析器。
- [x] M3-013 高敏信息拦截（身份证号/住址等）。

## 4.3 家长沟通文案
- [x] M3-020 Prompt 注入拼装（成绩趋势+标签+观察+历史语气）。
- [x] M3-021 输出结构约束（先肯定→问题→建议）。
- [x] M3-022 卡片操作（编辑/采纳/重生）。
- [x] M3-023 采纳结果回写与证据可追溯。

## 4.4 期末评语批量生成
- [x] M3-030 班级级任务队列。
- [x] M3-031 语义去重与模板多样化。
- [x] M3-032 每条评语依据计数标注。
- [x] M3-033 人工确认后批量导出。

## 4.5 班会/活动文案
- [x] M3-040 单主题多对象文案（家长/学生/校内通知）。
- [x] M3-041 校本模板套用。

验收：3 个典型场景之一“家长沟通文案/期末评语”闭环可演示。✅ 已满足

---

## 5. M4（第 9-10 周）：作业/考评与题库（模块 4.2）

## 5.1 作业图片批量结构化
- [x] M4-001 前端拖拽仅传路径/句柄元数据（禁止 Base64 大包 IPC）。
- [x] M4-002 后端按路径白名单读取文件。
- [x] M4-003 图像预处理（去噪/矫正/分割）。
- [x] M4-004 OCR 提取与题块定位。
- [x] M4-005 学号姓名归并与自动核对。
- [x] M4-006 导出 Excel（姓名/学号/题号/得分/置信度/冲突标记）。

## 5.2 增强模式批卷（可选增强）
- [x] M4-010 接入多模态判卷（试卷 + 标准答案）。
- [x] M4-011 OCR 结果与多模态判定融合。
- [x] M4-012 冲突标记与低置信人工复核。
- [x] M4-013 降级策略（模型不可用自动回落 OCR+规则）。

## 5.3 错题重组与专属练习
- [x] M4-020 拉取近一月错题。
- [x] M4-021 题目模板化（知识点/难度/题型）。
- [x] M4-022 参数扰动生成同类题。
- [x] M4-023 生成 Word 练习卷 + 答案页。

验收：3 个典型场景之一“批量作业结构化”闭环可演示。✅ 已满足

---

## 6. M5（第 11-12 周）：系统设置收口与扩展（模块 5）

## 6.1 AI 配置中心
- [x] M5-001 Provider/Model 配置。
- [x] M5-002 参数预设（严谨/创意）。
- [x] M5-003 Keychain 集成（密钥不落盘明文）。

## 6.2 安全与隐私
- [x] M5-010 存储目录与生命周期（导出/归档/擦除）。
- [x] M5-011 外发前脱敏开关。
- [x] M5-012 高危操作二次确认闸门。

## 6.3 模板、快捷键、后台监控
- [x] M5-020 模板上传与版本控制。
- [x] M5-021 默认导出偏好。
- [x] M5-022 全局快捷键。
- [x] M5-023 监控接收文件夹。

## 6.4 Skills 与 MCP
- [x] M5-030 内置 Skills 运行时（office/ocr/image/math/export）。
- [x] M5-031 Python Skill 独立环境隔离（`~/.pureworker/skill-envs/...`）。
- [x] M5-032 uv 健康检查（PATH + 回退路径 + 绝对路径复验）。
- [x] M5-033 一键安装/修复 uv（用户明确触发，禁止静默安装）。
- [x] M5-034 MCP 注册、权限声明、健康检查。

验收：设置中心全量可用；扩展中心可启停可审计。

---

## 7. 跨里程碑必做工程补丁（必须并行纳入）

## 7.1 长任务持久化与崩溃恢复
- [ ] P-001 任务创建即持久化输入快照。
- [ ] P-002 分片提交中间结果。
- [ ] P-003 启动恢复 `running/recovering` 任务。
- [ ] P-004 幂等恢复（`task_id + item_id` 去重）。
- [ ] P-005 恢复提示“继续/终止”交互。
- [ ] P-006 任务租约防抢占。

## 7.2 人工确认异步挂起机制
- [ ] P-010 `approval_request` 持久化为真源。
- [ ] P-011 `request_id -> oneshot` 映射表。
- [ ] P-012 前端确认回传命令与状态更新。
- [ ] P-013 超时过期清理、防泄漏。
- [ ] P-014 重启后 pending 审批恢复。

---

## 8. 每周执行模板（建议）

## 8.1 周计划模板
- 本周目标（对应里程碑任务 ID）
- 输入依赖是否齐备（是/否）
- 产出清单（代码、文档、测试、演示）
- 风险与缓释

## 8.2 周验收模板
- 功能完成率（任务 ID 维度）
- 缺陷统计（严重/高/中/低）
- 质量门禁（测试、构建、性能）
- 延期项与追赶计划

---

## 9. 质量门禁与验收标准（DoD）

## 9.1 功能 DoD
- [ ] 模块 5、3、4.2、4.3 均有端到端闭环演示。
- [ ] 典型场景至少 3 个全链路通过：
  1) 批量作业结构化
  2) 批量期末评语
  3) 家长沟通文案

## 9.2 质量 DoD
- [ ] 关键任务成功率 ≥ 95%
- [ ] OCR 结构化字段准确率 ≥ 90%
- [ ] 批量任务中断恢复成功率 ≥ 99%
- [ ] 关键页面本地交互反馈 < 200ms

## 9.3 安全 DoD
- [ ] 密钥不明文落盘
- [ ] 导出/归档/擦除全链路可追溯
- [ ] 所有 AI 输出默认草稿并需教师确认

---

## 10. 任务分配建议（最小编制映射）

- 桌面/前端工程师（2）：UI、IPC 前端、交互约束、设置中心
- Rust/后端工程师（2）：Agent Core、DB、任务状态机、审批闸门、工具封装
- AI 应用工程师（1）：Prompt、记忆注入、批量生成与去重策略、模型评测
- QA（1）：功能回归、性能压测、恢复测试、安全测试
- 产品/设计（共享）：流程验收、文案规范、模板体验

---

## 11. 文档与追踪产物清单

- `doc/development-plan-v1.0.md`（本文件）
- `doc/env-setup-record.md`（环境初始化记录）
- `doc/milestone-weekly-report.md`（周报模板）
- `doc/risk-register.md`（风险台账）
- `doc/uat-checklist.md`（UAT 验收清单）

---

## 12. 开始执行顺序（可直接开工）

1. 完成第 1 章“国内源环境配置”并产出环境记录。  
2. 进入 M1：先打通 SQLite + IPC + 任务状态机 + 审计日志最小链路。  
3. 按 M2 → M3 → M4 → M5 顺序推进，每周做阶段验收。  
4. 最后一周执行全链路 UAT、性能与安全验收，冻结 v1.0 发布候选。
