# PureWorker 与 opencode / oh-my-openagent / openclaw 架构对比调研报告

> 调研时间：2026-03-21  
> 调研方式：拉取外部源码后进行本地源码阅读与证据摘录，不依赖二手总结  
> 对比对象：`opencode`、`oh-my-openagent`、`openclaw`、`pure-worker`  
> 调研范围：agent、提示词体系、供应商接入、skills 加载机制、插件机制、多 agent 协同  
> 调研备注：外部源码已拉取到 `/tmp` 临时目录进行阅读，本文完成后将按要求删除

---

## 1. 结论摘要

这三款开源项目并不是简单地“模型 + prompt + tools”拼装，而是都把 **Agent 运行时** 当作独立系统在设计：

1. **opencode** 强在“可配置 Agent Runtime”——配置分层、权限规则、目录化 agent/command/plugin 加载、MCP 一等公民、ACP 协议桥接都比较成熟。
2. **oh-my-openagent** 强在“编排纪律”——通过 Sisyphus / Hephaestus / Oracle / Metis / Momus 等角色，把 intent gate、delegation、todo、verification、background task 变成硬约束。
3. **openclaw** 强在“control plane / gateway / ACP bridge”——它不是只做一个 CLI agent，而是在做跨会话、跨渠道、跨 runtime 的统一代理层。
4. **pure-worker 当前并不弱在单点能力缺失**，而是弱在这些能力还没有被提升为统一运行时：
   - Provider、Prompt、Skills、MCP、Agentic Search 都已经有底座；
   - 但还没有形成稳定的 **Agent Profile → Prompt Builder → Tool Exposure → Session/Event Runtime → Verification** 这一条主执行链。
5. 如果要吸收这三者经验，**最该学的不是照搬 UI 或 prompt 文案，而是补齐 Agent Runtime 架构层**。

---

## 2. 调研对象与源码版本

### 2.1 opencode

- 仓库：`https://github.com/anomalyco/opencode`
- 本次调研 commit：`2e0d5d230893dbddcefb35a02f53ff2e7a58e5d0`

### 2.2 oh-my-openagent

- 仓库：`https://github.com/code-yeongyu/oh-my-openagent`
- 本次调研 commit：`363661c0d682d1e19ef28221dd01707e255cf4f8`

### 2.3 openclaw

- 仓库：`https://github.com/openclaw/openclaw`
- 本次调研 commit：`8a05c05596ca9ba0735dafd8e359885de4c2c969`

### 2.4 pure-worker（对比基线）

- 本地仓库路径：`/mnt/c/Users/wangl/SynologyDrive/code/pure-worker`
- 本次重点对照目录：`apps/desktop/src-tauri/src/services/` 与 `doc/`

---

## 3. 调研方法与证据原则

本次结论只基于以下两类信息：

1. **外部项目源码文件**
2. **pure-worker 当前实现源码与现有审计文档**

不采用“README 上写了什么就算什么”的方式，而是尽量以实现文件为准。以下章节中给出的“证据文件”均为本次实际阅读的文件路径。

---

## 4. pure-worker 当前基线

在对比优秀项目前，先明确 pure-worker 已经具备哪些能力，避免误判为“从零开始”。

### 4.1 已有能力

1. **Provider 配置与多模型字段**
   - `apps/desktop/src-tauri/src/services/llm_provider.rs:77-128`
   - 已支持 `default_text_model`、`default_vision_model`、`default_tool_model`、`default_reasoning_model` 的任务分流。

2. **Prompt 模板注册与能力匹配**
   - `apps/desktop/src-tauri/src/services/prompt_template_registry.rs:18-277`
   - 已有 `TaskType / Modality / OutputProtocol / ModelCapability / TemplateSelector / fallback chain`。

3. **Skills 注册、校验、执行桥接**
   - `apps/desktop/src-tauri/src/services/skill.rs:15-317`
   - `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs:19-289`
   - 已支持 skill registry、路径校验、ToolDyn 适配、统一通过 `SkillExecutorService` 执行。

4. **Builtin tools 与 Tool Registry**
   - `apps/desktop/src-tauri/src/services/builtin_skills/mod.rs:17-63`
   - `apps/desktop/src-tauri/src/services/tool_registry.rs:12-230`
   - 已有 builtin / skill / MCP 三类工具元数据与按角色白名单过滤。

5. **MCP 基础接入**
   - `apps/desktop/src-tauri/src/services/mcp_runtime.rs:1-328`
   - `apps/desktop/src-tauri/src/services/mcp_tool_adapter.rs:16-154`
   - `apps/desktop/src-tauri/src/lib.rs:302-330`
   - 已具备 stdio MCP 初始化、`tools/list`、`tools/call`、启动时注册到 Tool Registry 的能力。

6. **领域编排已有一个雏形：Agentic Search**
   - `apps/desktop/src-tauri/src/services/agentic_search_agent.rs:20-395`
   - 已有面向学生/记忆检索的专用 Agent Builder 和工具白名单。

### 4.2 当前核心短板

pure-worker 的问题并不是“没有这些组件”，而是：

1. **这些组件分散存在，缺少统一 agent runtime 层**。
2. **prompt 仍偏模板注册，不是按 agent 身份动态装配**。
3. **skills / MCP / builtin tools 虽已可调用，但缺少像 opencode / oh-my-openagent 那样的能力分层与加载优先级体系**。
4. **多 agent 协同只在局部领域存在，还没有通用 delegation / background task / session bridge**。
5. **插件机制更像“功能注册”，还不是“运行时扩展系统”**。

---

## 5. 维度一：Agent 架构

### 5.1 opencode 的做法

证据文件：

- `packages/opencode/src/agent/agent.ts:24-251`

关键点：

1. `Agent.Info` 直接把 agent 抽象成正式配置对象：
   - `name / description / mode / permission / model / variant / prompt / options / steps`
2. 内建 agent 不是一个，而是一组：
   - `build`、`plan`、`general`、`explore`、`compaction`、`title`、`summary`
3. agent 的核心差异不只是 prompt，而是 **权限规则 + 模式 + 默认行为**。
4. 用户配置可以覆盖或新增 agent，说明 opencode 的 agent 层是“可声明式扩展”的。

评价：

- opencode 把 agent 看成 **带权限和执行边界的 runtime profile**，这比“写几个 prompt 常量”成熟得多。

### 5.2 oh-my-openagent 的做法

证据文件：

- `src/agents/sisyphus.ts:44-340`
- `src/agents/hephaestus/agent.ts:32-154`

关键点：

1. `Sisyphus` 负责 orchestrator 角色，`Hephaestus` 负责 autonomous deep worker 角色。
2. agent prompt 不是静态文案，而是根据：
   - 可用 agent
   - 可用 tools
   - 可用 skills
   - 可用 categories
   - 当前模型
   动态拼装。
3. `Hephaestus` 还会按模型类型切换 prompt source：`gpt` / `gpt-5-3-codex` / `gpt-5-4`。

评价：

- oh-my-openagent 把 agent 看成 **可编排角色系统**，非常强调职责分化与流程纪律。

### 5.3 openclaw 的做法

证据文件：

- `src/acp/translator.ts:339-1100`
- `src/agents/workspace.ts:12-141, 168-320`

关键点：

1. openclaw 的 agent 不只服务于 CLI 会话，而是服务于 ACP / gateway / workspace runtime。
2. workspace 会自动准备 `AGENTS.md / SOUL.md / TOOLS.md / IDENTITY.md / USER.md / BOOTSTRAP.md / MEMORY.md` 等 bootstrap 文件。
3. agent 身份与工作区记忆是 runtime 的一部分，而不是外部附加文本。

评价：

- openclaw 更接近“Agent OS / Agent Gateway”的方向，抽象层级高于一般桌面助手。

### 5.4 pure-worker 的差距

pure-worker 当前有：

- 领域 agent builder（如 `AgenticSearchAgentBuilder`）
- chat / grading / communication 等 prompt 模板

但缺少：

1. **统一 AgentProfile 结构体**：没有把 `角色 / 权限 / prompt / tool slice / model policy / verification policy` 聚合成一等对象。
2. **primary agent / subagent / summarize / title / plan 等模式化角色**。
3. **agent 运行边界**：现在 tool allowlist 仍以 registry + role 白名单为主，粒度还不够细。

结论：

- pure-worker 需要先补“Agent Profile 层”，而不是继续堆 prompt template。

---

## 6. 维度二：提示词体系（Prompt System）

### 6.1 opencode 的做法：内建 prompt 文件 + 配置覆盖

证据文件：

- `packages/opencode/src/agent/agent.ts:12-16, 131-203`

关键点：

1. prompt 被拆成独立文件：`generate.txt / compaction.txt / explore.txt / summary.txt / title.txt`
2. prompt 与 agent 角色绑定，但仍可由配置覆盖。
3. prompt 不是唯一变量，权限和模式与 prompt 同等重要。

### 6.2 oh-my-openagent 的做法：动态 Prompt Builder

证据文件：

- `src/agents/sisyphus.ts:44-340`
- `src/agents/hephaestus/agent.ts:48-125`

关键点：

1. prompt 由代码动态生成，而不是只靠静态文本模板。
2. prompt 内容直接感知当前可用工具、可用 agent、可用 skill、可用分类。
3. 不同模型有不同 prompt build path。

这是它最值得 pure-worker 学的点之一：

- **Prompt 不应该只按业务模板选，而应该按“运行时上下文”装配。**

### 6.3 openclaw 的做法：workspace bootstrap 文件驱动

证据文件：

- `src/agents/workspace.ts:24-35, 132-179`

关键点：

1. prompt 上下文部分通过 workspace 文件装配；
2. agent 身份、工具说明、用户偏好、bootstrap 指令可以由不同文件承载。

### 6.4 pure-worker 的差距

证据文件：

- `apps/desktop/src-tauri/src/services/prompt_template_registry.rs:98-277`

优点：

1. 已有任务类型、模态、能力要求、fallback。
2. 已有版本化模板注册表，适合教育场景业务输出。

不足：

1. 这是 **业务模板系统**，不是 **agent runtime prompt system**。
2. 目前缺少把以下信息装入 prompt 的统一装配器：
   - 当前角色
   - 当前可用工具
   - 当前会话阶段
   - 当前验证规则
   - 当前模型能力
3. 缺少“系统层 prompt + agent 层 prompt + task 层 prompt + tool policy 摘要”这种分层结构。

结论：

- pure-worker 应保留现有业务模板体系，但需要新增一层 **Prompt Assembler / Agent Prompt Builder**。

---

## 7. 维度三：供应商接入（Provider / Gateway）

### 7.1 opencode 的做法：大而全的 provider runtime

证据文件：

- `packages/opencode/src/provider/provider.ts:22-145, 152-239`

关键点：

1. 一次性集成大量 provider：OpenAI、Anthropic、Azure、Google、Vertex、OpenRouter、xAI、Mistral、Groq、DeepInfra、Cerebras、Cohere、Gateway、TogetherAI、Perplexity、Vercel、GitLab 等。
2. 对不同 provider 做差异化 loader，例如：
   - OpenAI / xAI 走 responses API
   - Copilot 判断是否切 responses / chat
   - Azure 根据模式决定 responses / chat
3. Provider 不是简单枚举，而是 **带模型发现、环境变量注入、fallback、特性处理** 的适配层。

### 7.2 oh-my-openagent 的做法：更关注模型与 agent prompt 绑定

证据文件：

- `src/agents/hephaestus/agent.ts:18-30, 56-125`

关键点：

1. 不同模型映射不同 prompt builder。
2. 模型不是简单用于“选 API”，而是决定 agent 行为。

### 7.3 openclaw 的做法：Gateway catalog + discovery + fallback

证据文件：

- `src/agents/vercel-ai-gateway.ts:4-197`

关键点：

1. 有静态 catalog，也会尝试动态拉取 Gateway 模型列表。
2. 如果远端 discovery 失败，会 fallback 到静态模型集。
3. 模型定义里带 `reasoning / input / contextWindow / maxTokens / cost`，属于运行时元数据，而不是仅 UI 展示。

### 7.4 pure-worker 的差距

证据文件：

- `apps/desktop/src-tauri/src/services/llm_provider.rs:77-182`
- `doc/ai-capability-gap-audit-report-2026-03-14.md:179-218`

优点：

1. 已支持 `openai / anthropic / deepseek / qwen / gemini / custom`。
2. 已支持任务类型到模型的选择。

不足：

1. provider runtime 仍以“配置服务”视角为主，而不是“统一 provider engine”。
2. 模型能力元数据尚未全面下沉到执行决策层。
3. 缺少成熟的 discovery + fallback catalog 机制。
4. 缺少像 opencode / openclaw 那样对 provider 特性进行差异化路由。

结论：

- pure-worker 下一步不该只是“继续加 provider 名称”，而应升级为 **Provider Runtime + Model Catalog + Capability Router**。

---

## 8. 维度四：Skills 加载机制

### 8.1 opencode 的做法：配置目录优先、可加载 skill 目录

证据文件：

- `packages/opencode/src/config/config.ts:81-178`
- `packages/opencode/src/agent/agent.ts:55-74`

关键点：

1. 支持多级配置来源：remote well-known、global、project、`.opencode` 目录、inline、managed config。
2. `.opencode/agents/`、`.opencode/commands/`、`.opencode/plugins/` 会被自动扫描合并。
3. skill 目录会影响 agent 的默认 external_directory 白名单。

### 8.2 oh-my-openagent 的做法：scope precedence + skill/command 合并暴露

证据文件：

- `src/tools/skill/tools.ts:13-23, 37-98, 186-317`

关键点：

1. scope 优先级明确：`project > user > opencode/opencode-project > plugin/config/builtin`
2. skills 与 commands 统一为 slash-invocable item 暴露。
3. skill 可以携带 metadata、compatibility、license、allowedTools。
4. skill 如果内嵌 MCP 配置，还能在 skill 加载时展示 MCP capabilities。

评价：

- 这已经不是“数据库里存几条 skill 记录”，而是 **文件系统优先的能力包生态**。

### 8.3 pure-worker 的现状与差距

证据文件：

- `apps/desktop/src-tauri/src/services/skill.rs:15-317`
- `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs:19-289`
- `apps/desktop/src-tauri/src/services/runtime_paths.rs:205-276`
- `apps/desktop/src-tauri/migrations/0009_skill_seed_data.sql:1-123`

优点：

1. 已有 skill registry、skill 类型、路径校验、env_path 校验。
2. 已有 builtin skill 的文件化导出能力，`runtime_paths.rs` 甚至会在运行时解包 `SKILL.md / package.json / builtin-skills.json`。
3. 已有统一适配器把 skill 桥到 Rig Tool。

不足：

1. 当前 skills 的主入口仍偏数据库 CRUD，而不是目录发现 + 文件覆盖优先级。
2. 缺少 project / user / builtin / plugin 多级来源合并策略。
3. 缺少类似 oh-my-openagent 的 command/skill 一体化加载体验。
4. 缺少 skill-embedded MCP 能力声明与发现。

结论：

- pure-worker 已经具备 skills 底座，但还没有形成 **workspace-first skill runtime**。

---

## 9. 维度五：插件机制

### 9.1 opencode 的做法：plugin 是配置系统的一部分

证据文件：

- `packages/opencode/src/config/config.ts:130-166, 206-214, 259-265`

关键点：

1. plugin 配置和 agent / command 一样，是整体 config merge 的一部分。
2. 可以从不同目录自动加载 plugin。
3. 甚至会为 plugin 安装依赖。

### 9.2 oh-my-openagent 的做法：Claude Code 兼容 plugin component loader

证据文件：

- `src/plugin-handlers/plugin-components-loader.ts:25-69`
- `src/plugin/tool-registry.ts:42-159`

关键点：

1. 插件会被拆成 commands / skills / agents / mcpServers / hooksConfigs 等组件。
2. 工具注册表会把 builtin tools、background tools、task delegation、skill、skill_mcp、session tools、hashline edit 等统一拼装。
3. 插件体系不是单一扩展点，而是几乎覆盖整个 agent 运行面。

### 9.3 openclaw 的做法：Plugin SDK surface

证据文件：

- `src/plugin-sdk/index.ts:1-81`

关键点：

1. 插件 SDK 导出的是一整套 surface：channel plugin、provider plugin、runtime plugin、context engine、hooks、wizard 等。
2. 插件不只是“附加工具”，而是可以扩 runtime 的多个层面。

### 9.4 pure-worker 的差距

证据文件：

- `apps/desktop/src-tauri/src/services/tool_registry.rs:12-230`
- `apps/desktop/src-tauri/src/services/runtime_paths.rs:205-276`

现状判断：

1. pure-worker 已有工具注册中心，但它更像统一索引，不是插件系统。
2. builtin skills 已经能文件化导出，说明项目实际上已走出“插件包格式”的第一步。
3. 但目前缺少：
   - 插件发现器
   - 插件生命周期
   - 插件来源优先级
   - 插件对 agent / prompt / tools / hooks / mcp 的统一扩展接口

结论：

- pure-worker 的短期目标不必做到 openclaw 那么重，但至少可以先做成 **skill/plugin package loader + runtime registration pipeline**。

---

## 10. 维度六：多 Agent 协同

### 10.1 opencode 的做法：有子 agent，但重点是模式与协议桥接

证据文件：

- `packages/opencode/src/agent/agent.ts:116-157`
- `packages/opencode/src/acp/agent.ts:134-320`

关键点：

1. `general`、`explore` 等 subagent 是正式 agent。
2. ACP bridge 会把 permission request、tool update、usage update、session update 等映射出去。
3. 多 agent 不只是“再调一次模型”，而是与 session / permission / tool telemetry 联动。

### 10.2 oh-my-openagent 的做法：把 delegation 做成主工作流

证据文件：

- `src/agents/sisyphus.ts:75-340`
- `src/plugin/tool-registry.ts:51-147`
- `src/openclaw/dispatcher.ts:69-180`

关键点：

1. 系统 prompt 直接要求：复杂任务默认 delegate，而不是默认自己做。
2. 工具层内建 background task、delegate task、session manager。
3. 可以调度 OpenClaw gateway 或命令型 gateway，说明它的多 agent 协同有外部执行面。

### 10.3 openclaw 的做法：ACP / gateway / control-plane 协作

证据文件：

- `src/acp/translator.ts:53-123, 159-253, 313-340`
- `src/acp/control-plane/spawn.ts:17-77`

关键点：

1. ACP translator 负责把 Gateway 事件翻译为 ACP session updates。
2. session 配置项包括 thought level、fast mode、verbose level、reasoning level、usage detail、elevated actions。
3. spawn 失败时会做 runtime close、session close、binding cleanup、gateway delete，是完整 control-plane 收尾。

### 10.4 pure-worker 的差距

证据文件：

- `apps/desktop/src-tauri/src/services/agentic_search_agent.rs:33-69`
- `apps/desktop/src-tauri/src/services/tool_registry.rs:135-171`

现状：

1. pure-worker 目前的“协同”更多是领域内编排，例如 agentic search。
2. 还没有通用 subagent、background task、session manager、delegation protocol。
3. 也没有外部 agent runtime bridge（例如 ACP）。

结论：

- 这是 pure-worker 与前三者差距最大的一个维度。

---

## 11. 为什么这些项目先进，而 pure-worker 现在容易“散”

根因不是工程能力不够，而是抽象层次不同：

### 11.1 外部优秀项目的抽象层次

它们大多围绕以下主轴组织系统：

`Agent Profile -> Prompt Builder -> Tool Exposure -> Session Runtime -> Event / Permission / Telemetry -> Plugin/Skill/MCP Extension`

### 11.2 pure-worker 当前抽象层次

当前更接近：

`Provider Service + Prompt Template Registry + Skill Registry + MCP Runtime + 某些专用 Agent + Tauri IPC`

这条链的问题是：

1. 每个点都能工作；
2. 但中间缺少统一 runtime，把这些点串成稳定执行闭环；
3. 所以越往后加能力，越容易出现“局部可用、整体不协调”。

---

## 12. pure-worker 最值得学习的点

### 12.1 第一优先级：建立 Agent Runtime 核心抽象

建议新增一层统一结构，例如：

- `AgentProfile`
- `PromptAssembler`
- `ToolExposurePolicy`
- `ModelRoutingPolicy`
- `SessionExecutionRuntime`
- `ExecutionEvent`

意义：

1. 把 chat、grading、communication、agentic_search 这些入口统一到同一运行时；
2. 把 skills / builtin / MCP 工具暴露逻辑统一起来；
3. 为未来多 agent、会话事件流、审计日志提供标准接口。

### 12.2 第二优先级：把 Prompt 系统从“模板选择”升级为“运行时装配”

建议：

1. 保留业务模板注册表；
2. 但在其上新增系统层装配：
   - 身份约束
   - 工具摘要
   - 当前任务模式
   - 输出协议
   - 教师审阅闭环要求

### 12.3 第三优先级：把 Skills 做成 workspace-first

建议：

1. 维持 DB registry 作为缓存与审计索引；
2. 真正加载优先从目录发现出发；
3. 加入来源优先级：`project > user > builtin > remote/imported`；
4. 支持 skill 包声明 allowed tools、兼容性、内嵌 MCP。

### 12.4 第四优先级：把 MCP 从“启动时注册”升级为“会话级工具源”

当前做法已经能用，但还不够深。

建议补齐：

1. 会话级 MCP 可见性
2. 权限域隔离
3. 工具调用事件上报
4. 失败重连与状态缓存
5. HTTP / SSE transport 支持

### 12.5 第五优先级：逐步引入多 Agent 协同，但不要一步学 openclaw

pure-worker 是教师场景本地优先桌面应用，不需要一开始就复制 openclaw 的 gateway/control-plane 全套。

更现实的路线：

1. 先做 **本地 subagent / background task / session event**；
2. 再做 **agent delegation policy**；
3. 最后再评估是否需要 ACP 或外部 control-plane。

---

## 13. 分阶段改进建议

### P0（最该立即做）

1. **新增 Agent Runtime 抽象层**
2. **新增 Prompt Assembler**
3. **统一 Tool Exposure（builtin / skill / MCP）**
4. **把 chat / agentic_search 接入统一 Session Event 流**

### P1（完成 P0 后）

1. **Provider Runtime 升级**：模型 catalog、capability router、discovery/fallback
2. **Skill Loader 升级**：目录发现、来源优先级、包导入
3. **MCP 升级**：transport 扩展、状态缓存、调用轨迹

### P2（具备稳定 runtime 后）

1. **多 agent 协同**：explore / summarize / verify / search 等通用子 agent
2. **后台任务与任务树**
3. **必要时评估 ACP / 外部 agent bridge**

---

## 14. 不建议直接照搬的点

### 14.1 不建议直接复制 oh-my-openagent 的超重 prompt 治理

原因：

1. pure-worker 当前阶段更需要 runtime 抽象，而不是先复制超长 system prompt。
2. 过早引入大量 orchestration 纪律，可能让教育场景 agent 变得过重。

### 14.2 不建议直接复制 openclaw 的 gateway/control-plane 体系

原因：

1. openclaw 面向的是更通用的 agent runtime 网络边界；
2. pure-worker 当前仍以本地桌面应用为核心，优先级不匹配。

### 14.3 不建议只学 opencode 的配置层，而忽略执行层

原因：

1. 纯配置分层很好学；
2. 但如果不配套 session runtime、tool telemetry、agent profile，最后仍然会散。

---

## 15. 最终判断

### 15.1 pure-worker 当前位置

可以把 pure-worker 现在理解为：

- **已经拥有一批 AI 底座零件**；
- **但还没把这些零件装配成真正的 Agent Runtime**。

### 15.2 三个外部项目最值得学习的分别是什么

1. **向 opencode 学**：
   - Agent profile
   - 配置分层
   - MCP / ACP 一等公民化

2. **向 oh-my-openagent 学**：
   - delegation-first 的编排纪律
   - 动态 prompt builder
   - skill / command / task / session 的统一工具面

3. **向 openclaw 学**：
   - session bridge
   - gateway runtime
   - workspace bootstrap files
   - control-plane cleanup 思维

### 15.3 对 pure-worker 最合适的吸收顺序

最合理的吸收顺序是：

1. **先学 opencode 的 runtime 抽象**
2. **再学 oh-my-openagent 的 orchestration discipline**
3. **最后按需要局部借鉴 openclaw 的 bridge / control-plane 思想**

这是因为 pure-worker 当前最缺的不是“再多几个工具”，而是：

> 一个统一、稳定、可扩展、适合教师场景的人审闭环 Agent Runtime。

---

## 16. 关键证据文件清单

### 16.1 opencode

- `/tmp/opencode/packages/opencode/src/agent/agent.ts`
- `/tmp/opencode/packages/opencode/src/config/config.ts`
- `/tmp/opencode/packages/opencode/src/mcp/index.ts`
- `/tmp/opencode/packages/opencode/src/acp/agent.ts`
- `/tmp/opencode/packages/opencode/src/provider/provider.ts`

### 16.2 oh-my-openagent

- `/tmp/oh-my-openagent/src/agents/sisyphus.ts`
- `/tmp/oh-my-openagent/src/agents/hephaestus/agent.ts`
- `/tmp/oh-my-openagent/src/tools/skill/tools.ts`
- `/tmp/oh-my-openagent/src/plugin/tool-registry.ts`
- `/tmp/oh-my-openagent/src/plugin-handlers/plugin-components-loader.ts`
- `/tmp/oh-my-openagent/src/openclaw/dispatcher.ts`

### 16.3 openclaw

- `/tmp/openclaw/src/acp/translator.ts`
- `/tmp/openclaw/src/acp/control-plane/spawn.ts`
- `/tmp/openclaw/src/agents/workspace.ts`
- `/tmp/openclaw/src/plugin-sdk/index.ts`
- `/tmp/openclaw/src/agents/vercel-ai-gateway.ts`

### 16.4 pure-worker

- `apps/desktop/src-tauri/src/services/llm_provider.rs`
- `apps/desktop/src-tauri/src/services/prompt_template_registry.rs`
- `apps/desktop/src-tauri/src/services/skill.rs`
- `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs`
- `apps/desktop/src-tauri/src/services/tool_registry.rs`
- `apps/desktop/src-tauri/src/services/builtin_skills/mod.rs`
- `apps/desktop/src-tauri/src/services/mcp_runtime.rs`
- `apps/desktop/src-tauri/src/services/mcp_tool_adapter.rs`
- `apps/desktop/src-tauri/src/services/agentic_search_agent.rs`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src-tauri/src/services/runtime_paths.rs`
- `apps/desktop/src-tauri/migrations/0009_skill_seed_data.sql`
- `doc/ai-capability-gap-audit-report-2026-03-14.md`
- `doc/ai-capability-gap-action-plan.md`

---

## 17. 后续可直接立项的主题

基于本报告，建议后续直接拆成以下立项：

1. `WP-AI-012 Agent Runtime 重构`
2. `WP-AI-013 Prompt Assembler 与 Agent Profile`
3. `WP-AI-014 Workspace-first Skill Loader`
4. `WP-AI-015 MCP Runtime 深化与会话级工具暴露`
5. `WP-AI-016 通用 Subagent / Background Task 框架`

如果后续继续推进，我建议先从 **WP-AI-012 + WP-AI-013** 开始，因为这两项能把现有 Provider、Prompt、Skill、MCP、Agentic Search 全部串起来。
