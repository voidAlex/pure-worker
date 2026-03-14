# WP-AI-010: Agentic Search 编排器实现计划

## 目标
实现 Agentic Search 编排器，让 AI 回答学生问题时先检索相关资料再生成回答。

## 现状
- `memory_search.rs` 已具备证据检索能力
- `student_memory.rs` 支持长期记忆
- `StudentProfile360` 已存在
- 但工作台聊天没有自动编排"先检索，再回答"

## 实现内容

### 1. 意图识别
- `IntentClassifier`: 识别查询是否涉及学生/班级/课堂/作业
- 实体解析: 学生姓名、班级、时间范围、学科

### 2. 检索编排器
`AgenticSearchOrchestrator`：
- 步骤 1: 实体解析（学生、班级、时间）
- 步骤 2: 并行检索多个数据源
  - 学生档案 (`student.get_profile_360`)
  - 证据检索 (`memory.search_evidence`)
  - 行课记录 (`lesson.search_records`)
  - 作业结果 (`assignment.search_results`)
  - 家校沟通 (`communication.search_history`)
- 步骤 3: 证据去重与重排
- 步骤 4: 生成带证据来源的回答

### 3. Rig Agent 实现
- `AgenticSearchAgent`: 使用 Rig 框架的 Agent
- Tool 集合: 上述检索工具
- 输出格式: 结构化 JSON（结论+证据链）

### 4. 证据链展示
回答应包含：
- 结论摘要
- 证据来源概览（数据源、时间范围、可信度）
- 风险提示（证据不足、数据过旧）

### 5. 与工作台集成
- `chat_with_ai` 命令支持 `use_agentic_search` 参数
- 前端展示证据链卡片

## 文件变更
- `src/services/agentic_search.rs`: 编排器核心
- `src/services/intent_classifier.rs`: 意图识别
- `src/agents/agentic_search_agent.rs`: Rig Agent 实现
- `src/commands/chat.rs`: 集成 agentic search
- `src/models/search_result.rs`: 搜索结果模型

## 验收标准
- [ ] 意图识别准确率 > 80%
- [ ] 支持多数据源并行检索
- [ ] 回答包含结构化证据链
- [ ] 证据不足时给出风险提示
