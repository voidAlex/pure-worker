# PureWorker AI 能力缺口补齐行动计划

> 产出时间：2026-03-14  
> 依据来源：现有代码深度扫描 + `doc/prd-v1.0.md` + `doc/tech-solution-v1.0.md` + `doc/development-plan-v1.0.md` + 外部参考（OpenAI/Anthropic 兼容、Agentic search、长期记忆、Markdown 渲染与思考过程展示）

---

## 1. 目标与结论

当前项目并不是“完全没有 AI 底座”，而是已经具备了一批分散能力，但**工作台、运行时编排、模型能力治理、学生数据检索链路**还没有真正打通。

本次扫描后的结论是：

1. **聊天底座已有雏形，但没有形成真正的会话系统**：前端工作台仍是内存态，后端仍是单轮问答。
2. **AI 配置页的“可配”与运行时的“可用”不一致**：设置页能拉模型、标记多模态，但真正执行链仍主要走 OpenAI 兼容模式，且只有一个 `default_model`。
3. **Skills 底座已基本可用，MCP 仍停留在注册表层**：技能可以注册并转成 Rig Tool，MCP 目前只有 CRUD 与健康检查，没有进入 Agent 执行链。
4. **学生记忆与证据检索已经有基础设施，但还没有面向“学生问题回答”的专用编排**。
5. **日程事件已存在，但“行课记录”这个教师业务核心对象缺失**，导致学生表现、作业、试卷、课堂观察无法按课次沉淀。
6. **自主记忆目前只有“学生长期记忆”能力，没有“教师偏好 / 助手偏好 / 跨会话工作偏好”体系**。

因此，推荐的实施路线不是逐项零散补丁，而是按以下四条主线推进：

- **主线 A：工作台会话化**（历史、Markdown、思考/状态轨迹）
- **主线 B：模型与供应商运行时重构**（OpenAI / Anthropic 标准、自定义供应商、模型能力分层）
- **主线 C：Agent 执行链补全**（skills / 内部工具 / MCP / Agentic search）
- **主线 D：教师场景知识底座补齐**（行课记录 + 自主记忆）

---

## 2. 扫描范围与关键证据

### 2.1 前端工作台 / 对话 UI

- `apps/desktop/src/components/dashboard/AiWorkbench.tsx`
- `apps/desktop/src/components/layout/AiPanel.tsx`
- `apps/desktop/src/pages/DashboardPage.tsx`

确认事实：

- `AiPanel.tsx` 当前把消息保存在前端组件状态中，不是持久化会话。
- UI 目前仅做纯文本展示，没有 Markdown 渲染器。
- “思考中...” 只是加载态文案，不是模型轨迹、工具轨迹或推理摘要。

### 2.2 后端聊天与 Agent 调用链

- `apps/desktop/src-tauri/src/commands/chat.rs`
- `apps/desktop/src-tauri/src/services/llm_provider.rs`
- `apps/desktop/src-tauri/src/services/skill_tool_adapter.rs`

确认事实：

- `chat_with_ai` 当前输入是 `message + agent_role`，输出是 `content + model`，仍是**单轮调用**。
- `chat_with_ai` 会尝试加载已启用 skills，但没有会话上下文、没有消息持久化、没有事件流。
- skills 可以通过 `build_all_enabled_skill_tools()` 注入 Rig Agent。

### 2.3 会话数据库

- `apps/desktop/src-tauri/migrations/0001_init.sql`

确认事实：

- 已有 `conversation`、`conversation_message` 表。
- 这两张表当前**没有 `is_deleted` 字段**，与项目“业务表必须软删除”的规范不一致。
- 当前代码中没有配套的 conversation CRUD / message CRUD IPC 命令把它们用起来。

### 2.4 AI 配置与模型管理

- `apps/desktop/src/pages/SettingsPage.tsx`
- `apps/desktop/src-tauri/src/models/ai_config.rs`
- `apps/desktop/src-tauri/src/commands/ai_config.rs`
- `apps/desktop/src-tauri/src/services/llm_provider.rs`
- `apps/desktop/src-tauri/migrations/0005_ai_config.sql`

确认事实：

- 设置页已支持：供应商预设、自定义 base URL、拉取模型列表、展示 `is_vision` 标签。
- `AiConfig` 只有一个 `default_model` 字段。
- `ModelInfo` 只有 `id / name / is_vision`，能力描述过于粗糙。
- 供应商名校验仍是白名单策略，运行时主要基于 OpenAI 兼容客户端。
- Anthropic / Gemini 在“配置层”出现了，但“执行层”并未完整按各自标准实现。

### 2.5 Prompt 模板与多模态提示词

- `packages/prompt-templates/templates/*.toml`
- `apps/desktop/src-tauri/src/services/prompt_template.rs`
- `apps/desktop/src-tauri/src/services/multimodal_grading.rs`

确认事实：

- 文本类模板已经有版本化雏形（家校沟通、学期评语、活动公告）。
- 多模态批改仍是单独硬编码 prompt，不在统一模板体系中。
- 目前没有“按模型能力切换 prompt 版本”的机制。

### 2.6 Skills / 内部工具 / MCP

- `apps/desktop/src-tauri/src/services/skill.rs`
- `apps/desktop/src-tauri/src/services/skill_discovery.rs`
- `apps/desktop/src-tauri/src/services/skill_executor.rs`
- `apps/desktop/src-tauri/src/services/unified_tool.rs`
- `apps/desktop/src-tauri/src/services/mcp_server.rs`
- `apps/desktop/src-tauri/src/commands/mcp_server.rs`

确认事实：

- Skill 注册、发现、执行、Rig Tool 适配已经基本具备。
- 内置技能也已存在（OCR、图片预处理、Office、导出等）。
- MCP 目前只有注册表和健康检查，没有 MCP 客户端、没有 tools/list、没有 tools/call、没有转换为 Rig Tool。
- **Skills 添加方式需要重构**：当前是通过软件内的 CRUD 表单让用户手动编辑，而非从文件夹或 GitHub 仓库导入。

### 2.7 学生记忆 / 检索 / 档案 / 行课

- `apps/desktop/src-tauri/src/services/memory_search.rs`
- `apps/desktop/src-tauri/src/services/student_memory.rs`
- `apps/desktop/src-tauri/src/services/student.rs`
- `apps/desktop/src-tauri/src/services/schedule_event.rs`
- `apps/desktop/src-tauri/src/models/student.rs`

确认事实：

- 已有 `search_evidence`：SQL 过滤 + FTS5 + 文件检索 + 规则重排。
- 已有学生长期记忆 Markdown 体系。
- 已有 `StudentProfile360` 聚合能力。
- 已有 `schedule_event`，但它只是日程事件，不是“行课记录”。
- 还没有把“成绩 / 作业 / 试卷 / 观察 / 表现 / 沟通”统一挂到某次课上。

---

## 3. 逐项问题分析与方案

> 说明：你原始列表中有两个“问题 9”，这里统一按 10 个问题处理，避免遗漏。

### 3.1 问题 1：工作台对话没有历史记录

#### 现状

- 前端消息仅存在 `AiPanel.tsx` 本地状态。
- 后端已有 `conversation / conversation_message` 表，但未真正启用。
- `chat_with_ai` 仍是单轮 IPC，没有 conversation_id。

#### 方案

建立**正式会话域模型**：

1. 补迁移，给 `conversation`、`conversation_message` 增加：
   - `is_deleted`
   - `updated_at`（message 表建议也补齐）
   - 可选的 `metadata_json`
2. 新增 Conversation 服务与 IPC：
   - `list_conversations`
   - `get_conversation_detail`
   - `create_conversation`
   - `rename_conversation`
   - `archive/delete_conversation`
   - `list_conversation_messages`
3. 改造 `chat_with_ai` 为会话化命令：
   - 输入增加 `conversation_id`
   - 写入 user message / assistant message
   - 会话标题支持首次自动摘要生成
4. 前端增加左侧会话列表，并让工作台支持切换、续聊、新建。

#### 依赖关系

- 这是整个工作台升级的**第一优先级前置项**。
- Markdown、思考过程、Agentic search 都应挂在这个会话对象之上。

---

### 3.2 问题 2：工作台对话没有 Markdown 渲染，也没有展示思考过程

#### 现状

- 前端当前按纯文本渲染。
- 项目依赖中尚无 `react-markdown` 等渲染库。
- 后端当前没有流式事件，也没有“思考摘要 / 工具轨迹 / 检索轨迹”结构化输出。

#### 方案

#### A. Markdown 渲染

建议前端采用：

- `react-markdown`
- `remark-gfm`
- 如需数学公式则增加 `remark-math + rehype-katex`
- 如要兜底 HTML 清理，可在扩展 HTML 能力时再引入 `DOMPurify`

渲染目标：

- 标题、列表、表格、代码块、引用、任务清单
- 教师场景常见内容：表格、要点列表、分级小标题、代码/JSON/清单

#### B. 思考过程展示

不建议展示原始 Chain-of-Thought，建议改为**结构化轨迹事件**：

1. `status`：正在分析 / 正在检索 / 正在调用工具 / 正在组织答案
2. `tool_call`：调用了哪个技能、哪个内部工具、哪个 MCP 工具
3. `search_summary`：检索了哪些学生资料 / 课堂记录 / 记忆证据
4. `thinking_summary`：系统生成的简短思考摘要，不暴露原始推理全文

#### C. 传输方式

建议把聊天 IPC 升级为**事件流式返回**，优先顺序：

1. 增量文本 token
2. 结构化状态事件
3. 最终完成事件

#### 依赖关系

- 必须先完成会话化；否则流式轨迹无法稳定挂载到 message。
- 工具 / MCP / Agentic search 接入后，轨迹展示价值才真正体现出来。

---

### 3.3 问题 3：AI 对话无法加载 skills，也无法调用内部工具和 MCP 工具

#### 现状

- **skills：后端底座其实已经部分打通**，`chat_with_ai` 会加载 `build_all_enabled_skill_tools()`。
- **内部工具：目前更多以“内置 skill”的形式暴露给 Agent，而不是独立工具域**。
- **MCP：完全没接入运行时**，只有注册表和健康检查。
- **skills 添加方式需要调整**：当前实现是通过软件内的 CRUD 表单让用户手动编辑 skill 信息，这与 Agent Skills 规范的最佳实践不符。技能应该是**可版本控制、可协作、可复用**的代码资产，而不是孤立的数据库记录。

#### 结论

这个问题要拆成三块看：

1. **skills 不是“完全不能用”，而是缺少工作台可观测性和启用治理**。
2. **内部工具缺少工具目录、权限域和角色白名单**。
3. **MCP 是真正的功能缺口，目前还没进入 Agent Runtime**。

#### 方案

#### A. 统一工具目录（推荐）

引入统一 Tool Registry，把三类能力统一建模：

- Builtin Tool（Rust 内部能力）
- Skill Tool（来自 skill_registry）
- MCP Tool（来自 mcp server tools/list）

统一元数据建议包含：

- `tool_id`
- `tool_type`（builtin / skill / mcp）
- `display_name`
- `description`
- `input_schema`
- `permission_scope`
- `enabled`
- `agent_visibility`
- `supports_streaming`

#### B. 工作台侧的工具治理

工作台对话至少支持：

- 展示本轮可用工具集合
- 显示工具调用轨迹
- 失败时给出中文错误说明
- 对高风险工具保留审批 / 确认环节

#### C. MCP 接入

新增：

- `mcp_client.rs`
- `mcp_tool_adapter.rs`
- 启动 MCP server 子进程并完成协议握手
- `tools/list` 拉取 MCP 工具清单
- `tools/call` 执行 MCP 工具
- 按 `permission_scope` 与启用状态过滤

#### D. Agent 角色白名单

不同工作台角色不应暴露相同工具：

- 班主任助手：学生档案 / 记忆检索 / 家校沟通 / 行课记录
- 批改助手：OCR / 图片预处理 / 批改 / 题库 / 试卷 / 多模态
- 行政助手：日程 / 通知 / 模板 / 导出

#### E. Skills 导入方式重构（关键调整）

**核心原则**：Skills 不应在软件内通过表单编辑，而应作为**外部代码资产**导入。

##### 推荐导入方式

1. **本地文件夹导入**
   - 支持选择本地 `.agents/skills/{skill-name}/` 目录
   - 自动读取 `SKILL.md`  frontmatter 和 `scripts/` 目录
   - 支持热更新（文件变更后自动同步）
   - 保留原始文件结构，便于版本控制

2. **GitHub 仓库导入**
   - 支持导入公开的 skills 仓库
   - 支持指定分支 / tag / commit
   - 自动克隆到本地缓存目录
   - 支持后续更新（拉取最新版本）

3. **内置 Skills  Bundle**
   - 预置官方维护的常用 skills
   - 随软件版本发布
   - 用户可选择启用/禁用

##### 废除的交互方式

- ❌ 不再提供表单填写 skill 名称、描述、代码
- ❌ 不再在数据库中存储 skill 源码（只存元数据和启用状态）
- ❌ 不再支持在 UI 中直接编辑 skill 代码

##### 技术实现要点

- `skill_discovery.rs` 已具备扫描目录能力，需扩展支持 Git 仓库
- 数据库中 `skill_registry` 表转为**只读索引**，记录导入来源和启用状态
- 提供 "重新导入/更新" 功能，方便同步外部变更
- 保留 `permission_scope`、`enabled` 等运行时控制字段

##### 优势

- ✅ 符合 Agent Skills 规范（https://agentskills.io/specification）
- ✅ Skills 可纳入 Git 版本控制
- ✅ 支持社区共享和复用
- ✅ 开发与使用分离，降低出错风险
- ✅ 便于团队协作和代码审查

---

### 3.4 问题 4：AI 配置还不支持自定义供应商；需要支持 OpenAI 和 Anthropic 标准

#### 现状

- 设置页可以输入自定义 provider_name 与 base_url。
- 但后端 provider_name 仍受白名单约束。
- 运行时主要按 OpenAI 兼容客户端执行。
- Anthropic 只是预设出现，执行层并未完整按 Anthropic Messages API 抽象。

#### 方案

建立**Provider Adapter 分层**，不要再把“供应商名称”与“客户端实现”耦合在一起。

建议的 Provider 分类：

1. `openai_compatible`
2. `anthropic_native`
3. `custom_openai_compatible`
4. `custom_anthropic_compatible`
5. 后续扩展：`gemini_native`、`ollama_openai_compatible`

配置模型建议从“白名单名称”改为“协议类型 + 基础地址 + 认证方式”：

- `provider_key`
- `provider_type`
- `display_name`
- `base_url`
- `auth_mode`
- `api_key`
- `models_fetch_strategy`
- `extra_headers_json`
- `config_json`

#### 关键原则

- **OpenAI 标准** 和 **Anthropic 标准** 分别是两套协议适配器，不应强行复用一个客户端。
- “自定义供应商”本质上不是随便输一个名字，而是选择“兼容 OpenAI”还是“兼容 Anthropic”。

---

### 3.5 问题 5：AI 配置无法选择获取到的供应商模型

#### 现状

- 设置页可以拉模型列表，也能点选默认模型。
- 但选择结果最终只落在一个 `default_model` 上。
- 没有把模型选择和“任务场景 / 模态能力 / 运行时路由”绑定起来。

#### 方案

把“模型选择”从单字段升级为**场景化模型配置**：

- `default_text_model`
- `default_multimodal_model`
- `default_reasoning_model`（可选）
- `default_embedding_model`（后续 Agentic search 需要时启用）

并且允许在设置页对已拉取模型做以下操作：

1. 标记“默认文本模型”
2. 标记“默认多模态模型”
3. 手动覆盖模型能力（防止第三方接口元数据不准）
4. 缓存模型列表，避免每次重新请求

---

### 3.6 问题 6：AI 配置无法区分当前模型是文本模型还是多模态模型

#### 现状

- 现在只有 `is_vision: bool`。
- 该字段来自硬编码前缀推断，不够稳健。

#### 方案

把模型能力元数据从布尔值升级为结构化 schema：

- `supports_text_input`
- `supports_image_input`
- `supports_audio_input`
- `supports_tool_calling`
- `supports_reasoning`
- `supports_json_mode`
- `context_window`
- `max_output_tokens`
- `source`（provider_api / manual_override / builtin_catalog）

前端展示上，不再只分“文本 / 多模态”，而是展示：

- 文本
- 图像理解
- 工具调用
- JSON 输出
- 推理增强

这样后续在不同 Agent 场景中才能自动路由正确模型。

---

### 3.7 问题 7：提示词没有针对多模态模型进行专门优化

#### 现状

- 文本任务已有模板体系。
- 多模态批改 prompt 仍然写死在 `multimodal_grading.rs`。
- 没有 prompt version / modality variant / capability fallback。

#### 方案

为 prompt 模板体系增加以下维度：

1. **任务类型**：chat / communication / grading / agentic_search_summary
2. **模态类型**：text / multimodal
3. **模型能力要求**：json_mode / tool_calling / vision
4. **输出协议**：markdown / structured_json / draft_card

例如：

- `chat_homeroom_text.toml`
- `chat_homeroom_multimodal.toml`
- `grading_multimodal_json.toml`
- `student_analysis_agentic_search.toml`

同时要求：

- 多模态 prompt 必须显式说明输入资源类型（图像、作业、标准答案、截图等）
- 输出必须对齐教师审阅闭环（草稿态、证据链、可复核）
- 对不支持视觉的模型要有降级 prompt

---

### 3.8 问题 8：还没实现 Agentic search，例如查询某学生表现时应先搜索其所有资料

#### 现状

- `memory_search.rs` 已经具备通用证据检索能力。
- `student_memory.rs` 已经支持长期记忆文件。
- `student.rs` 已有 `StudentProfile360`。
- 但工作台聊天没有自动编排“先检索，再回答”。

#### 方案

把 `search_evidence` 从“可被调用的一个工具”升级成**默认检索前置策略**。

建议的查询编排：

1. 识别用户意图：是否涉及学生 / 班级 / 课堂 / 作业 / 试卷 / 沟通历史
2. 先做实体解析：定位学生、班级、时间范围、学科
3. 汇聚结构化证据：
   - 学生基本档案
   - 成绩
   - 观察记录
   - 家校沟通
   - 长期记忆 Markdown
   - 错题 / 作业 / 批改结果
4. 做证据去重与重排
5. 生成“带证据来源的回答”

#### 推荐的能力拆分

- `student.resolve_entity`
- `student.get_profile_360`
- `memory.search_evidence`
- `lesson.search_records`（行课记录补齐后）
- `assignment.search_results`
- `communication.search_history`

#### 输出要求

工作台回答不应只给结论，还应给：

- 结论摘要
- 证据来源概览
- 时间范围
- 风险提示（如证据不足、数据过旧）

---

### 3.9 问题 9：缺少行课记录；学生表现、作业、试卷都应跟对应行课记录关联起来

#### 现状

- 当前只有 `schedule_event`，表达的是“事件”，不是“课”。
- 学生成绩、作业、观察记录多数只挂在 student_id 或 job_id 上。
- 无法回答“这次课上谁表现异常”“某次课后的作业整体掌握如何”。

#### 方案

新增**Lesson Record（行课记录）**领域对象，不要直接复用 `schedule_event` 代替。

建议模型：

#### A. lesson_record

- `id`
- `class_id`
- `schedule_event_id`（可空）
- `subject`
- `lesson_date`
- `lesson_index`
- `topic`
- `teaching_goal`
- `homework_summary`
- `teacher_note`
- `status`
- `is_deleted`
- `created_at / updated_at`

#### B. 各业务表补 lesson_record_id

优先补到：

- `observation_note`
- `score_record`（至少允许关联考试或课堂测）
- `grading_job`
- `assignment_asset`
- `parent_communication`（可选，若沟通来自某次课后）

#### C. 查询价值

一旦补齐行课记录，可以支持：

- 某次课的课堂表现总结
- 某次课后作业完成情况
- 某个知识点对应的学生问题分布
- 某学生在连续若干课次中的变化轨迹

这会成为 Agentic search 的核心锚点。

---

### 3.10 问题 10：缺少自主记忆，应根据 soul.md 记住用户偏好，类似 Claude Code 的 claude.md

#### 现状

- 已有“学生长期记忆”，但它是**学生侧记忆**。
- 还没有“教师偏好 / 助手行为偏好 / 全局工作偏好”记忆域。
- `0007_m5_settings.sql` 里只有通用设置和默认导出偏好，不等于自主记忆。

#### 方案

建立**三层记忆体系**：

#### A. Session Memory（会话记忆）

- 当前对话中的临时上下文
- 保存在 conversation / conversation_message + summary 中

#### B. Preference Memory（偏好记忆）

新增教师/助手偏好载体：

- `workspace/soul.md`：项目级 / 助手级行为准则
- `workspace/user.md`：教师个人偏好
- `app_settings` + 新表 `teacher_preference_memory`：结构化偏好项

偏好项示例：

- 常用输出风格
- 默认学段 / 学科语气
- 评语偏好
- 导出格式偏好
- 是否偏好先给结论再给证据

#### C. Long-term Working Memory（长期工作记忆）

针对教师而不是学生，沉淀：

- 近期高频工作任务
- 常用模板
- 常用班级/学生关注点
- 教师反复修订后的表达偏好

#### 更新策略

不要做“每轮都写记忆”，建议采用：

1. 用户显式确认记住
2. 高频重复偏好触发候选记忆
3. 重要编辑行为后形成候选记忆卡片
4. 用户确认后写入长期偏好

这样既像 Claude/记忆助手，又符合教师场景下的可控性与安全性。

---

## 4. 总体架构建议

## 4.1 新的 AI Runtime 分层

建议把当前 AI 能力分成五层：

1. **Conversation Layer**：会话、消息、轨迹、草稿状态
2. **Provider Layer**：OpenAI / Anthropic / 自定义兼容协议
3. **Model Capability Layer**：文本、多模态、工具调用、推理、JSON 模式
4. **Tool Runtime Layer**：builtin / skill / MCP 统一工具注册与权限控制
5. **Teacher Knowledge Layer**：学生档案、行课记录、长期记忆、证据检索

## 4.2 不建议继续沿用的做法

1. 不建议继续只靠 `default_model` 驱动所有任务。
2. 不建议把“多模态”继续硬编码为 `is_vision` 一个布尔值。
3. 不建议把 MCP 仅作为设置页里的配置项，不接到 Agent 执行链。
4. 不建议把 `schedule_event` 直接当成 `lesson_record` 使用。
5. 不建议直接暴露原始 Chain-of-Thought，而应提供可审计的结构化轨迹摘要。

---

## 5. 分阶段落地路线图

## Phase 1：工作台会话化 + 基础显示升级

### 目标

让工作台从“演示型单轮聊天”升级为“可持续使用的会话系统”。

### 范围

- conversation / conversation_message 正式启用
- 会话 CRUD
- 左侧历史列表
- Markdown 渲染
- 基础消息卡片与草稿态
- 基础 status 事件

### 产出

- 工作台历史会话可切换
- 刷新不丢消息
- AI 回复可正确渲染 Markdown
- 有最小化轨迹显示（如“正在检索 / 正在回答”）

### 风险

- 需要补 conversation 表软删除迁移
- 需要考虑旧数据兼容

---

## Phase 2：Provider / Model Runtime 重构

### 目标

把 AI 配置页的“可配”真正变成运行时“可用”。

### 范围

- Provider Adapter 抽象
- OpenAI 标准支持
- Anthropic 标准支持
- 自定义供应商按协议接入
- 多模型配置（文本 / 多模态 / 可选推理）
- 模型能力元数据表或缓存

### 产出

- 自定义供应商真正可用
- 文本与多模态模型分离配置
- 运行时能自动选择合适模型

### 风险

- 需要重构 `llm_provider.rs`
- 需要处理不同协议下的认证与流式差异

---

## Phase 3：工具运行时补全（skills / builtin / MCP）

### 目标

让工作台 AI 真正具备“调用工具完成任务”的能力。

### 范围

- Tool Registry 统一建模
- 内部工具统一暴露
- skill 治理与可观测性
- MCP 客户端 + Tool Adapter
- 角色级工具白名单

### 产出

- 工作台可显示本轮工具调用轨迹
- skills、内部工具、MCP 工具都能进入 Agent 执行链

### 风险

- MCP 子进程稳定性
- 工具权限控制
- 长任务超时与错误恢复

---

## Phase 4：Agentic search 与教师知识底座补齐

### 目标

让 AI 回答“某学生怎么样”“这节课效果如何”时，先查数据再回答。

### 范围

- 行课记录模型上线
- 学生数据与课次关联
- Agentic search 编排器
- 证据链回答
- 多模态 / 文本任务 prompt 分型

### 产出

- 查询学生表现时能自动搜档案、成绩、观察、记忆、作业
- 查询课堂表现时能按行课记录聚合

### 风险

- 历史数据补链工作量大
- 需要明确“证据不足”时的退化回答策略

---

## Phase 5：自主记忆

### 目标

让助手逐步记住教师偏好，但仍保持“人可控、可撤销、可审计”。

### 范围

- `soul.md / user.md` 载体
- 教师偏好结构化存储
- 候选记忆机制
- 用户确认后写入长期偏好
- 在工作台启动或生成前自动加载适用偏好

### 产出

- 助手会逐渐贴近教师习惯
- 偏好有来源、有确认、有撤回

### 风险

- 不能把学生隐私与教师偏好混写
- 不能自动记住未经确认的敏感偏好

---

## 6. 推荐实施优先级

### P0（必须先做）

1. 会话持久化与历史列表
2. Markdown 渲染
3. Provider / Model 数据结构重构
4. 多模型配置（文本 / 多模态）

### P1（第二批）

1. 结构化轨迹事件
2. Tool Registry
3. MCP 客户端接入
4. 多模态 prompt 模板化

### P2（第三批）

1. 行课记录领域模型
2. Agentic search 编排器
3. 证据链回答

### P3（第四批）

1. 自主记忆 / 偏好记忆
2. 长期工作偏好候选与确认机制

---

## 7. 推荐拆分的开发任务包

为避免一次性改动过大，建议拆成以下任务包：

1. **WP-AI-001**：工作台会话持久化与会话列表
2. **WP-AI-002**：工作台 Markdown 渲染与消息卡片
3. **WP-AI-003**：聊天流式事件与思考摘要展示
4. **WP-AI-004**：Provider Adapter 重构（OpenAI / Anthropic）
5. **WP-AI-005**：模型能力元数据与多模型配置
6. **WP-AI-006**：统一 Tool Registry + 内部工具接入
7. **WP-AI-007**：MCP Runtime 接入
8. **WP-AI-008**：多模态 Prompt 模板体系
9. **WP-AI-009**：行课记录模型与业务关联改造
10. **WP-AI-010**：Agentic search 编排器
11. **WP-AI-011**：教师偏好记忆 / soul.md 机制

---

## 8. 外部参考对本项目的可借鉴点

### 8.1 Provider / 模型能力

- LiteLLM：适合借鉴 Provider Adapter 与模型能力元数据 schema
- OpenRouter：适合借鉴模型发现、能力标签、动态路由思路
- Anthropic Messages API：适合单独实现 `anthropic_native` 适配器，不要塞进 OpenAI 兼容层

### 8.2 Markdown / 轨迹显示

- `react-markdown` + `remark-gfm`：适合当前 React 技术栈
- 结构化状态事件流：比直接展示原始推理更符合教师产品的人审闭环

### 8.3 Agentic search / 长期记忆

- Anthropic Contextual Retrieval：适合借鉴“先检索上下文，再生成”
- Mem0：适合借鉴分层记忆与记忆更新策略
- OpenClaw 的 Markdown memory：适合借鉴 `soul.md / user.md / memory.md` 这类人可读记忆载体

---

## 9. 最终建议

如果只能给一个总建议，那就是：

**不要把这 10 个问题当成 10 个独立小需求，而要把它们视为“会话系统、模型系统、工具系统、知识系统”四个子系统的一次联动升级。**

最优顺序是：

1. **先把工作台做成真会话系统**
2. **再把 Provider / Model Runtime 做对**
3. **再把 tools / skills / MCP 接入执行链**
4. **最后把 Agentic search、行课记录、自主记忆补成教师场景壁垒**

这样推进，既能尽快看到用户可感知的工作台提升，也不会把后续的 Agentic 能力建立在脆弱底座上。
