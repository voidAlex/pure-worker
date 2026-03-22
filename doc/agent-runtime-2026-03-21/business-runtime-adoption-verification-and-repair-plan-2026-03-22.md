# PureWorker Agent Runtime 业务落地验证与修复计划

> 生成时间：2026-03-22  
> 验证范围：仅基于 `doc/agent-runtime-2026-03-21/open-source-agent-architecture-research-2026-03-21.md` 与当前代码仓库源码  
> 验证目标：判断本次 Agent Runtime 重构是否已经**完整应用到业务逻辑**，并形成可执行修复计划  
> 证据原则：只认源码中的真实调用链、真实 IPC 暴露、真实前端接线路径；**不以设计文档、计划文档、TODO 文档作为完成依据**

---

## 1. 结论摘要

本次业务验证结论是：**重构尚未完整应用到业务逻辑上**。

更准确地说，当前状态是：

1. **统一运行时已经接入部分核心 AI 业务**，尤其是聊天、家校沟通、学期评语、活动公告生成，以及增强型批改中的 LLM 判分环节。
2. **但仍有大量业务链路停留在“半迁移”状态**：
   - 运行时抽象存在，但没有形成完整业务闭环；
   - 部分业务仍直接走旧服务链，而非统一 `ExecutionRequest -> ExecutionOrchestrator -> SessionEvent / ExecutionStore` 主链；
   - 某些业务虽然调用了新运行时，但关键能力（Agentic Search 证据注入、ExecutionStore 持久化、MCP 会话级可见性）并未在真实业务路径中生效。
3. **因此不能认定这次重构已经“完成并落地到业务”**；当前更接近“运行时底座已上线，但业务接入不完整”。

---

## 2. 本次业务验证的方法

本次验证不是继续看抽象层，而是按真实业务入口逐条核对：

1. 前端页面是否调用对应 IPC；
2. IPC 是否转入统一执行运行时；
3. 服务层是否真正使用 `AgentProfileRegistry`、`PromptAssemblerService`、`ExecutionOrchestratorBuilder`；
4. 运行时输出是否真正回流到业务数据、会话事件、任务进度、执行记录；
5. 用户可见行为是否已经与重构目标一致。

本次重点检查 6 条业务主链：

1. 聊天对话
2. 家校沟通生成
3. 学期评语生成 / 批量生成
4. 活动公告生成
5. 作业批改
6. 记忆检索 / 技能执行 / 相关辅助链路

---

## 3. 业务落地判定标准

若要认定“重构已完整应用在业务逻辑上”，至少需要同时满足以下条件：

1. **业务入口统一**：前端/IPC 已切换到新运行时入口，而不是新旧链路并存、能力分裂。
2. **运行时能力真实生效**：Agent Profile、Prompt Assembler、Tool Exposure、Model Routing、Session Event 不只是存在，而是被真实业务链调用。
3. **业务特性不倒退**：旧业务中的证据检索、流式反馈、任务进度、草稿落库、审批/采纳流程都能继续工作。
4. **会话与执行可追溯**：真实业务调用能够进入 `execution_session / execution_message / execution_record` 或等价统一运行时审计链。
5. **能力不只是局部试点**：不能只有 chat 或某个子服务迁移，其他核心 AI 业务仍各走各的。

按这个标准，本次重构**未达到完整业务落地**。

---

## 4. 逐条业务验证结果

### 4.1 聊天对话链路：**部分完成（主入口已迁移，但闭环未完成）**

#### 已落地证据

1. 聊天 IPC 已明确改为兼容层，并委托新执行命令：
   - `apps/desktop/src-tauri/src/commands/chat.rs:3`
   - `apps/desktop/src-tauri/src/commands/chat.rs:57`
   - `apps/desktop/src-tauri/src/commands/chat.rs:79`
2. 新执行命令已构建统一编排器：
   - `apps/desktop/src-tauri/src/commands/execution.rs:15`
   - `apps/desktop/src-tauri/src/commands/execution.rs:101`
   - `apps/desktop/src-tauri/src/commands/execution.rs:148`
3. 前端聊天 Hook 直接调用 `chat_stream`：
   - `apps/desktop/src/hooks/useChatStream.ts:410`
4. 前端 Hook 已监听 `chat-stream` / `execution-stream` 两类事件：
   - `apps/desktop/src/hooks/useChatStream.ts:369`

#### 未完成点

1. `AiPanel` 仍调用 `chat_with_ai`，拿到的是兼容性占位文案，而不是真正的业务内容：
   - `apps/desktop/src/components/layout/AiPanel.tsx:170`
   - `apps/desktop/src-tauri/src/commands/chat.rs:61`
   - `apps/desktop/src-tauri/src/commands/chat.rs:64`
2. 执行编排器虽然存在 `use_agentic_search` 字段，但提示词装配时 evidence 仍传空：
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs:146`
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs:277`
3. 会话事件主链目前只稳定覆盖 `Start / Chunk / Complete / Error`；虽然前端和 `commands/execution.rs` 已为 `ThinkingStatus / SearchSummary / Reasoning` 预留转发与消费能力，但 `ExecutionOrchestrator` 真实主链尚未产出这些事件，更没有 `ToolCall / ToolResult / ExecutionSummary` 的业务发布链。

#### 业务判断

聊天主入口已迁入新运行时，但仍保留明显兼容层和能力缺口，属于**接入成功但业务闭环未完成**。

---

### 4.2 家校沟通生成：**部分完成（生成走新链，检索与落库仍未统一）**

#### 已落地证据

1. 前端学生详情页直接调用 AI 生成命令：
   - `apps/desktop/src/pages/StudentDetailPage.tsx:299`
2. IPC 进入 `AiGenerationService::generate_parent_communication`：
   - `apps/desktop/src-tauri/src/commands/ai_generation.rs:34`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:130`
3. 生成阶段已转入统一运行时：
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:189`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:541`
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs:97`

#### 未完成点

1. 证据检索仍直接调用旧的 `MemorySearchService`，不是通过统一搜索运行时：
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:138`
2. 业务落库仍直接写 `parent_communication` 业务表，未进入统一 `execution_store`：
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:209`
   - `apps/desktop/src-tauri/src/services/parent_communication.rs:55`
3. `execution_session / execution_message / execution_record` 虽存在，但该生成链未见真实写入调用：
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs:20`
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs:73`

#### 业务判断

家校沟通生成已经**部分吃到新运行时能力**，但证据与执行审计仍是分裂的，不能算完整迁移。

---

### 4.3 学期评语生成 / 批量生成：**部分完成（单次生成已迁移，批量链仍是旧任务编排）**

#### 已落地证据

1. 单条学期评语生成通过 `AiGenerationService` 走统一运行时：
   - `apps/desktop/src-tauri/src/commands/ai_generation.rs:72`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:258`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:300`
2. 运行时 profile 已有专门的 `generation.semester_comment`：
   - `apps/desktop/src-tauri/src/services/ai_orchestration/agent_profile_registry.rs:201`
3. 前端批量页已通过 AI 生成命令启动任务：
   - `apps/desktop/src/pages/SemesterCommentsPage.tsx:164`

#### 未完成点

1. 批量链本质上还是旧式 `AsyncTask + for student in class_students` 循环：
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:380`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:397`
2. 批量任务只有进度 JSON，没有统一运行时事件树、子任务树或执行记录沉淀：
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:429`
   - `apps/desktop/src-tauri/src/services/async_task.rs:90`
3. 导出、采纳、更新仍完全基于旧业务表 `semester_comment`，而不是统一执行存储：
   - `apps/desktop/src/pages/SemesterCommentsPage.tsx:187`
   - `apps/desktop/src/pages/SemesterCommentsPage.tsx:251`
   - `apps/desktop/src/pages/SemesterCommentsPage.tsx:289`
   - `apps/desktop/src-tauri/src/commands/export.rs:74`

#### 业务判断

学期评语属于**单点迁移较好、批量流程仍旧业务化**的典型。单次生成接入新运行时，但批量任务并没有被运行时重新组织。

---

### 4.4 活动公告生成：**部分完成（AI 生成已迁移，但后续编辑/管理完全是旧业务 CRUD）**

#### 已落地证据

1. 前端页面直接调用活动公告 AI 生成：
   - `apps/desktop/src/pages/ActivityAnnouncementsPage.tsx:105`
2. 生成命令进入 `AiGenerationService::generate_activity_announcement`：
   - `apps/desktop/src-tauri/src/commands/ai_generation.rs:135`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:452`
3. 生成阶段已调用统一运行时：
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:488`

#### 未完成点

1. 页面列表、更新、删除全部还是传统 CRUD：
   - `apps/desktop/src/pages/ActivityAnnouncementsPage.tsx:92`
   - `apps/desktop/src/pages/ActivityAnnouncementsPage.tsx:139`
   - `apps/desktop/src/pages/ActivityAnnouncementsPage.tsx:152`
2. 后端公告服务也仅是常规表操作，不感知统一执行会话：
   - `apps/desktop/src-tauri/src/services/activity_announcement.rs:64`
   - `apps/desktop/src-tauri/src/services/activity_announcement.rs:112`
   - `apps/desktop/src-tauri/src/services/activity_announcement.rs:175`

#### 业务判断

这里的迁移本质上只是“生成动作接入了新运行时”，而不是“整个公告业务完成运行时化”。

---

### 4.5 作业批改：**部分完成（运行时仅进入增强评分子步骤，且尚未形成真正多模态主链）**

#### 已落地证据

1. 前端页面通过 `startGrading` 启动任务：
   - `apps/desktop/src/pages/AssignmentGradingPage.tsx:202`
2. 启动命令创建异步任务并执行 OCR / 批改流程：
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:203`
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:226`
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:243`
3. 增强型批改里的 LLM 判分环节已经调用统一运行时：
   - `apps/desktop/src-tauri/src/services/multimodal_grading.rs:182`
   - `apps/desktop/src-tauri/src/services/multimodal_grading.rs:188`
   - `apps/desktop/src-tauri/src/services/multimodal_grading.rs:195`
4. 项目内置手工回归脚本也只把“聊天 / 家长沟通 / 学期评语 / 作业批改”作为当前运行时验证主链，侧面说明批改仍是重点但未完全跑通：
   - `apps/desktop/src-tauri/src/bin/manual-runtime-regression.rs:103`

#### 未完成点

1. 批改业务主干仍是 `OCR -> 更新 job_progress -> AsyncTask` 的旧式批处理框架：
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:249`
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:280`
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:289`
2. 统一运行时只被用于“增强型评分子步骤”，并没有统领整条批改业务：
   - `apps/desktop/src-tauri/src/commands/assignment_grading.rs:261`
3. 前端看到的仍是旧任务进度模型，而不是统一 Session Event：
   - `apps/desktop/src/pages/AssignmentGradingPage.tsx:303`
   - `apps/desktop/src/pages/AssignmentGradingPage.tsx:316`
4. OCR 仍未形成真实生产链路；当前实现检测到模型文件后直接报“ONNX Runtime 未集成”，其余情况回退到模拟识别写入结果：
   - `apps/desktop/src-tauri/src/services/ocr.rs:118`
   - `apps/desktop/src-tauri/src/services/ocr.rs:125`
   - `apps/desktop/src-tauri/src/services/ocr.rs:135`
5. 当前增强型评分虽然走了运行时，但传入运行时的 `ExecutionRequest.attachments` 仍是空数组，因此它更接近“基于文本元数据的评分子步骤”，而不是真正把作业图像纳入多模态运行时主链：
   - `apps/desktop/src-tauri/src/services/multimodal_grading.rs:196`
   - `apps/desktop/src-tauri/src/services/multimodal_grading.rs:201`

#### 业务判断

作业批改是**最典型的半迁移业务**：运行时只进入了 LLM 子能力，且还没有真正接住图像输入，没有成为业务主编排层。

---

### 4.6 练习卷生成：**未接入运行时（AI 邻接业务仍是规则链）**

#### 证据

1. 练习卷生成服务从错题表和题库表直接组卷、扰动参数、导出 Word，没有进入统一运行时：
   - `apps/desktop/src-tauri/src/services/practice_sheet.rs:62`
   - `apps/desktop/src-tauri/src/services/practice_sheet.rs:114`
   - `apps/desktop/src-tauri/src/services/practice_sheet.rs:155`
   - `apps/desktop/src-tauri/src/services/practice_sheet.rs:187`
2. 该文件内也未命中 `ExecutionOrchestrator / ExecutionRequest / AgentProfileRegistry / PromptAssembler` 等运行时接入痕迹。

#### 业务判断

练习卷生成虽然不是本轮运行时重构的主报告对象，但它是紧邻批改结果消费的 AI 邻接业务。目前仍是**纯规则生成链**，后续若希望统一教师端 AI 体验，应纳入后续运行时接入范围。

---

### 4.7 记忆检索：**未完成迁移（仍是独立旧服务）**

#### 证据

1. IPC `search_evidence` 仍直接调用 `MemorySearchService`：
   - `apps/desktop/src-tauri/src/commands/memory_search.rs:18`
   - `apps/desktop/src-tauri/src/commands/memory_search.rs:24`
2. `search.agentic` profile 虽存在，但没有对应通用业务入口把它作为统一搜索产品能力使用：
   - `apps/desktop/src-tauri/src/services/ai_orchestration/agent_profile_registry.rs:240`
3. `AgenticSearchAgentBuilder::execute_search_stage` 只看到定义，没有看到真实主链接线：
   - `apps/desktop/src-tauri/src/services/agentic_search_agent.rs:92`

#### 业务判断

检索能力仍然主要是**老的业务服务能力**，而不是统一运行时中的一等业务阶段。

---

### 4.8 技能执行 / MCP / 执行存储：**基础设施存在，但尚未成为业务默认路径**

#### 技能执行

1. 技能执行仍走独立 `SkillExecutorService`，并未并入执行运行时：
   - `apps/desktop/src-tauri/src/commands/skill_executor.rs:37`
   - `apps/desktop/src-tauri/src/services/skill_executor.rs:40`

#### MCP

1. 运行时构建器支持 `with_mcp_servers`，但实际业务命令未注入：
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs:431`
2. 现有业务命令构建编排器时均只传 profile/event_bus/tool_registry：
   - `apps/desktop/src-tauri/src/commands/execution.rs:101`
   - `apps/desktop/src-tauri/src/commands/execution.rs:148`
   - `apps/desktop/src-tauri/src/services/ai_generation.rs:548`
   - `apps/desktop/src-tauri/src/services/multimodal_grading.rs:188`

#### 执行存储

1. `execution_store` 已实现三表统一持久化接口：
   - `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs:3`
2. 但在真实业务链中没有检索到实际调用：
   - 全仓仅在 `execution_store.rs` 自身出现相关 insert/update 实现

#### 业务判断

这些能力还停留在“可用底座”，尚未成为真实业务默认路径。

---

## 5. 总体业务判定矩阵

| 业务链路 | 是否接入新运行时 | 完成度判断 | 主要问题 |
|---|---|---|---|
| 聊天对话 | 是 | 部分完成 | 兼容层仍在、证据注入缺失、事件闭环不足 |
| 家校沟通生成 | 是 | 部分完成 | 检索与执行审计未统一 |
| 学期评语单次生成 | 是 | 部分完成 | 落库仍是旧业务表，执行记录未统一 |
| 学期评语批量生成 | 局部 | 部分完成 | 仍是旧异步任务循环编排 |
| 活动公告生成 | 是 | 部分完成 | 仅生成走新链，后续管理仍旧业务 CRUD |
| 作业批改 | 局部 | 部分完成 | 仅增强型 LLM 判分接入，主流程未迁移 |
| OCR 主链 | 否 | 未完成 | ONNX 未接入，当前回退模拟 OCR |
| 练习卷生成 | 否 | 未完成 | 仍是题库+规则扰动链 |
| 记忆检索 | 否 | 未完成 | 仍直接走 MemorySearchService |
| 技能执行 | 否 | 未完成 | 独立执行器，未接入统一运行时 |
| MCP 会话级工具暴露 | 否 | 未完成 | 构建器支持但业务未注入 |
| 执行三表持久化 | 否 | 未完成 | 有实现，无业务调用 |

---

## 6. 关键问题归因

从业务角度看，这次重构没有完整落地，主要不是因为“运行时抽象没做”，而是因为存在以下 5 个断点：

### 6.1 断点一：入口迁了，但业务阶段没迁全

典型表现：

1. 业务命令调用了 `ExecutionOrchestrator`；
2. 但检索、事件、执行记录、工具可见性、任务树仍在旧链路里各自为政。
3. 即便前端已经为 richer events 预留了消费逻辑，后端运行时主链也还没有真正产出这些事件。

### 6.2 断点二：AI 生成接入了新链，但业务落库仍是旧表模型

这本身不是问题，但如果没有 `execution_record` 做统一追踪，就会出现：

1. 业务结果能看到；
2. 但运行时执行证据、模型选择、搜索摘要、工具调用摘要看不到；
3. 无法形成真正的“运行时闭环”。

### 6.3 断点三：Agentic Search 仍是“能力准备好”，不是“业务默认能力”

`search.agentic` profile 已有，但：

1. 聊天 evidence 未注入；
2. 独立搜索入口仍走老服务；
3. 搜索摘要未成为真实 UI / 审计 / 会话输出的一部分。

### 6.4 断点四：批量任务体系和统一运行时体系还没汇合

学期评语批量生成、作业批改都说明了这一点：

1. 任务管理是旧式 `AsyncTask`；
2. AI 执行是新式 `ExecutionOrchestrator`；
3. 但两者之间缺乏任务树、子执行记录、统一事件桥接。

### 6.5 断点五：MCP / Skill / ExecutionStore 还没有进入真实业务默认路径

这会导致看起来“底座很先进”，但业务行为上仍然像旧系统。

补充说明：

1. `ExecutionStore` 不是“未来再接”的普通增强项，而是当前所有已迁入运行时的业务仍缺失统一执行主账本；
2. MCP 构建器虽支持 `with_mcp_servers`，但真实业务 builder 没有调用，导致 MCP 工具难以进入会话级工具面；
3. Skill 虽有独立适配与执行能力，但尚未成为统一运行时的默认业务路径。

### 6.6 断点六：AI 邻接能力链路没有一起升级

典型表现：

1. 批改主链依赖 OCR，但 OCR 仍是模拟回退；
2. 练习卷生成消费批改/错题结果，却完全没有接入统一运行时；
3. 导致用户视角下的“AI 教学闭环”仍是割裂的。

---

## 7. 修复目标

修复目标不是再新增抽象，而是把当前已完成的运行时抽象**真正压到业务主链上**。

具体目标：

1. 让核心 AI 业务入口全部统一走执行运行时；
2. 让 Agentic Search 真正参与提示词装配和会话事件；
3. 让执行三表真正成为业务可追溯底座；
4. 让批量任务与执行运行时形成可回放、可恢复、可审计的统一模型；
5. 让 MCP / Skill 至少在主业务链中可控、可见、可审计。
6. 让 OCR 与练习卷这类 AI 邻接能力不再拖累整体业务闭环。

---

## 8. 详细修复计划

### P0：补齐“统一运行时已接入业务，但未闭环”的关键缺口

#### P0-1 聊天链路去兼容壳，统一前后端入口

**目标**：聊天 UI 不再依赖兼容性占位响应，而是完整消费统一运行时事件流。

**修复项：**

1. 前端 `AiPanel` 停止使用 `chat_with_ai` 作为真实内容来源，统一改用流式 hook/事件通道；
2. 保留 `chat_with_ai` 仅作兼容桥时，要明确标为 deprecated，并避免业务 UI 继续引用；
3. 统一聊天页与侧边栏聊天面板的调用模型，避免一个走流式、一个走兼容壳。

**涉及文件：**

- `apps/desktop/src/components/layout/AiPanel.tsx`
- `apps/desktop/src/hooks/useChatStream.ts`
- `apps/desktop/src-tauri/src/commands/chat.rs`
- `apps/desktop/src-tauri/src/commands/execution.rs`

**验收标准：**

1. 所有聊天 UI 都通过统一事件流收消息；
2. 不再向用户展示“[流式响应已启动，请监听 chat-stream 事件]”这类兼容占位文案；
3. 聊天界面可稳定展示 `ThinkingStatus / SearchSummary / Reasoning / Chunk / Complete`。

---

#### P0-2 把 Agentic Search 真正接入聊天与搜索业务主链

**目标**：`use_agentic_search` 不再只是请求字段，而是真正影响提示词和事件输出。

**修复项：**

1. 在 `ExecutionOrchestrator::execute / execute_streaming` 中，当 `request.use_agentic_search` 或 profile `requires_agentic_search=true` 时：
   - 调用 `AgenticSearchAgentBuilder::execute_search_stage`；
   - 把 evidence 注入 `PromptAssemblerService::assemble`；
   - 发布 `SessionEvent::SearchSummary` 与必要的 `SessionEvent::Reasoning`；
   - 将搜索摘要写入 `ExecutionArtifacts.search_summary_json`。
2. 在 `ExecutionOrchestrator` 主链中补齐事件产出，不再只生成 `Start -> Chunk -> Complete`：
   - 至少补齐 `ThinkingStatus / SearchSummary / Reasoning`；
   - 明确 `ToolCall / ToolResult / ExecutionSummary` 是暂不支持还是立即纳入；
   - 保证前端现有事件消费逻辑不再是“预留但吃不到数据”。
3. 将独立 `search_evidence` 命令升级为：
   - 要么直接改为走 `ExecutionRequestFactory::for_search()`；
   - 要么新增统一搜索执行命令，并逐步废弃旧入口。

**涉及文件：**

- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_orchestrator.rs`
- `apps/desktop/src-tauri/src/services/agentic_search_agent.rs`
- `apps/desktop/src-tauri/src/commands/memory_search.rs`
- `apps/desktop/src-tauri/src/services/ai_orchestration/prompt_assembler.rs`

**验收标准：**

1. `use_agentic_search=true` 时，prompt 中 `evidence_text` 不再是占位默认值；
2. 前端能收到真实 `SearchSummary` 事件；
3. 搜索业务与聊天增强业务共享统一搜索运行时，而不是分裂成两套检索链。
4. `commands/execution.rs` 与前端现有预留事件类型能收到真实数据，而不是空转发能力。

---

#### P0-3 把 ExecutionStore 接到真实业务路径

**目标**：执行三表不再是“空底座”，而成为真实业务的审计与回放来源。

**修复项：**

1. 在 `execute` / `execute_stream` 中写入：
   - `execution_session`
   - `execution_message`
   - `execution_record`
2. 在 `AiGenerationService` 与 `MultimodalGradingService` 调用统一运行时时，记录执行会话与执行记录；
3. 将业务表（如 `parent_communication`、`semester_comment`、`activity_announcement`）与 `execution_record` 建立弱关联（例如 metadata/外键/引用 ID）。
4. 明确聊天会话 (`conversation` / `conversation_message`) 与执行会话 (`execution_session` / `execution_message`) 的映射策略，避免继续形成两套互不对齐的主账本。

**涉及文件：**

- `apps/desktop/src-tauri/src/services/ai_orchestration/execution_store.rs`
- `apps/desktop/src-tauri/src/commands/execution.rs`
- `apps/desktop/src-tauri/src/services/ai_generation.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`

**验收标准：**

1. 聊天、沟通生成、评语生成、活动公告、增强批改至少能查到对应 `execution_record`；
2. `search_summary_json / reasoning_summary / tool_calls_summary_json` 至少对核心链路有真实数据；
3. 业务结果可以回溯到一次具体执行；
4. 会话型业务能够解释清楚 conversation 与 execution 双存储之间的对应关系。

---

### P1：把“生成型业务已部分接入”升级为“完整运行时业务”

#### P1-1 统一生成型业务的请求构造方式

**目标**：家校沟通、学期评语、活动公告都通过统一工厂生成执行请求，而不是各服务手工拼 `ExecutionRequest`。

**修复项：**

1. 扩展 `ExecutionRequestFactory`：
   - `for_parent_communication_generation`
   - `for_semester_comment_generation`
   - `for_activity_announcement_generation`
2. `AiGenerationService` 全部改用工厂方法；
3. 明确各场景的 `entrypoint / agent_profile_id / metadata schema / use_agentic_search policy`；
4. 注意区分活动公告与另外两条生成链：活动公告当前没有证据检索阶段，不应强行套用“搜索增强型生成”的接入策略。

**收益：**

1. 避免同一类业务多个地方散落拼装请求；
2. 后续更容易统一测试与回归。

---

#### P1-2 把“生成后 CRUD”纳入统一执行上下文

**目标**：活动公告、家校沟通、学期评语的“生成、编辑、采纳、导出”能够挂到同一上下文，而不是生成属于新系统、后续流程属于旧系统。

**修复项：**

1. 为生成结果增加 `execution_record_id` 或等价引用；
2. 编辑、采纳、拒绝、导出时保留执行来源；
3. 前端详情页可展示“生成依据 / 执行摘要 / 证据条数”。

---

#### P1-3 统一活动公告业务链

**目标**：活动公告不仅生成接入运行时，其后续查询、更新、删除也可追溯到执行来源。

**修复项：**

1. 给 `activity_announcement` 增加执行来源字段；
2. 页面编辑区可查看本次生成使用的模板、模型、摘要；
3. 后续如支持重新生成，应直接使用同一执行上下文再生成，而不是新旧逻辑混用。

---

### P2：把批量任务与统一运行时汇合

#### P2-1 学期评语批量生成运行时化

**目标**：批量评语不只是 `AsyncTask + for` 循环，而是“任务 + 子执行记录 + 统一事件”。

**修复项：**

1. 每个学生生成动作产生独立 `execution_record`；
2. 任务进度来自子执行聚合，而不是手工计数；
3. 支持失败项重试、子项回放、局部恢复。

---

#### P2-2 作业批改主链运行时化

**目标**：统一运行时从“只负责增强评分子步骤”升级为“统领整个批改业务的 AI 环节”。

**修复项：**

1. 拆分批改阶段：OCR、结构化解析、LLM 判分、融合、冲突处理；
2. 为每个资产创建执行记录，形成资产级执行轨迹；
3. 将前端进度从粗粒度 `processed/failed` 升级为步骤级进度；
4. 让批改冲突、低置信度、人工复核结果都能关联到对应执行记录；
5. 真正把作业图像附件接入运行时请求，避免“多模态 profile + 空 attachments”的伪多模态状态。

**涉及文件：**

- `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`
- `apps/desktop/src-tauri/src/services/assignment_grading.rs`
- `apps/desktop/src-tauri/src/services/async_task.rs`

---

#### P2-3 OCR 真链路替换模拟回退

**目标**：让批改主链具备真实可用的 OCR 能力，而不是以模拟识别结果支撑后续评分。

**修复项：**

1. 明确 OCR 运行时方案（ONNX Runtime 或其他本地 OCR 后端）；
2. 去掉“模型不存在即模拟识别”的默认业务路径，至少要把模拟路径降级为开发/测试专用；
3. 为 OCR 结果引入能力状态标识，前端和审计能区分“真实识别”与“模拟识别”；
4. 将 OCR 失败、回退、耗时写入统一事件或执行记录。

**涉及文件：**

- `apps/desktop/src-tauri/src/services/ocr.rs`
- `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- `apps/desktop/src/pages/AssignmentGradingPage.tsx`

---

#### P2-4 建立任务树 / 子执行模型

**目标**：让 `AsyncTask` 与 `ExecutionRecord` 不再是两套孤立系统。

**修复项：**

1. 为任务增加父子关系或关联子执行表；
2. 批量任务可以映射到多个子执行；
3. 恢复任务时按子执行状态恢复，而不是只靠 checkpoint cursor。

---

### P2.5：把练习卷生成纳入统一业务闭环

#### P2-5 练习卷生成运行时化评估与接入

**目标**：决定练习卷生成是否纳入统一运行时；若纳入，应至少把“题目生成/改写/个性化推荐”升级为运行时能力，而不是纯规则扰动。

**修复项：**

1. 先区分练习卷业务的两层：
   - 规则层：错题筛选、题库过滤、文档导出；
   - AI 层：题目改写、变式生成、难度调整、个性化说明。
2. 若接入运行时：
   - 为练习卷定义独立 profile；
   - 将题目生成/改写阶段接到 `ExecutionOrchestrator`；
   - 保留 Word 导出与题库查询在业务服务层。
3. 若短期不接入，也要在架构上明确它属于“暂未纳入统一运行时的 AI 邻接业务”，避免误判为已完成迁移。

**涉及文件：**

- `apps/desktop/src-tauri/src/services/practice_sheet.rs`
- `apps/desktop/src-tauri/src/commands/assignment_grading.rs`
- `apps/desktop/src/pages/PracticeSheetsPage.tsx`

---

### P3：让 MCP / Skills / 工具暴露真正进入业务默认链路

#### P3-1 在真实业务命令中注入 MCP 可见性

**目标**：不再只是构建器支持 `with_mcp_servers`，而是业务实际可用。

**修复项：**

1. 在 `commands/execution.rs`、`AiGenerationService`、`MultimodalGradingService` 构建编排器前注入已启用 MCP 服务器映射；
2. 为会话级工具视图增加健康状态与权限域过滤；
3. 工具事件进入 `SessionEvent::ToolCall / ToolResult`。

---

#### P3-2 技能执行与统一运行时桥接

**目标**：技能不再只是独立工具执行器，而能成为运行时工具面的一部分。

**修复项：**

1. 让业务执行时可按 profile 暴露 skill tool；
2. 明确技能调用事件的记录与审计；
3. 对教师场景下真正有业务价值的技能（例如文档导出、OCR、格式处理）形成标准接入模式。

---

## 9. 实施顺序建议

建议按以下顺序推进：

1. **先做 P0-1 / P0-2 / P0-3**  
   原因：这三项能先把“统一运行时是否真的在业务里活着”这个问题解决。
2. **再做 P1-1 / P1-2 / P1-3**  
   原因：生成型业务最容易形成统一样板，做完后可以复用到更多 AI 业务。
3. **再做 P2-1 / P2-2 / P2-3**  
   原因：批量任务和批改链更复杂，适合在 P0/P1 稳定后统一收敛。
4. **最后做 P3**  
   原因：MCP / Skill 是放大器，不应在主链未稳定时先扩大复杂度。

---

## 10. 建议的验收方式

每完成一个阶段，都要从“业务而不是抽象”做回归。

### P0 验收

1. 聊天 UI 不再显示兼容性占位内容；
2. 开启搜索增强时，前端可见真实搜索摘要；
3. 至少一条聊天记录、家校沟通、学期评语生成记录能查到执行记录；
4. 前端已监听的 `ThinkingStatus / SearchSummary / Reasoning` 至少有一条真实业务链可以稳定收到。

### P1 验收

1. 家校沟通 / 学期评语 / 活动公告都能查看执行来源；
2. 生成型业务页面对统一运行时字段有消费；
3. 旧链路中手工拼请求的代码基本清理完成。

### P2 验收

1. 批量评语任务支持子项级追踪；
2. 作业批改能展示分阶段执行进度；
3. OCR 不再默认回退模拟识别；
4. 增强批改请求真正带上作业图像附件；
5. 失败恢复不再只靠粗粒度 task checkpoint。

### P3 验收

1. 至少一个真实业务场景中可控使用 MCP 工具；
2. 技能调用事件可见、可审计、可回放；
3. 工具暴露与 profile 权限真正一致。

---

## 11. 最终结论

基于这次业务验证，可以给出明确判断：

> **这次 Agent Runtime 重构还没有完整应用在业务逻辑上。**

当前状态不是“重构失败”，而是：

1. **运行时底座已经形成；**
2. **部分核心 AI 业务已经接入；**
3. **但真实业务闭环、执行追踪、搜索增强、批量任务统一化仍未完成。**

因此，后续修复重点不该是继续增加抽象，而应该是：

> **把现有运行时真正压到业务主链上，让聊天、生成、检索、批改、任务、工具暴露都以统一方式运行。**

---

## 12. 直接可立项的修复主题

建议直接拆为以下修复主题：

1. `WP-AI-BIZ-001 聊天业务统一运行时闭环`
2. `WP-AI-BIZ-002 Agentic Search 业务接入与搜索摘要事件化`
3. `WP-AI-BIZ-003 ExecutionStore 真实业务落库与审计接线`
4. `WP-AI-BIZ-004 生成型业务统一请求工厂与执行来源追踪`
5. `WP-AI-BIZ-005 学期评语批量任务运行时化`
6. `WP-AI-BIZ-006 作业批改主链运行时化`
7. `WP-AI-BIZ-007 运行时业务事件补齐与前后端真实贯通`
8. `WP-AI-BIZ-008 OCR 真链路接入与模拟路径降级`
9. `WP-AI-BIZ-009 练习卷生成运行时化评估与接入`
10. `WP-AI-BIZ-010 MCP / Skill 业务默认接入`

优先顺序建议：`001 -> 002 -> 003 -> 004 -> 005 -> 006 -> 007 -> 008 -> 009 -> 010`。
