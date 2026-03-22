-- 执行运行时数据表
-- Version: 0016
-- Description: 为统一 AI 执行运行时创建执行会话、消息和执行记录表

-- ============================================================
-- Execution Session Table
-- 执行会话表，表示一次完整的 AI 执行流程
-- ============================================================
CREATE TABLE IF NOT EXISTS execution_session (
    id TEXT PRIMARY KEY,
    teacher_id TEXT NOT NULL,
    title TEXT,
    entrypoint TEXT NOT NULL,
    agent_profile_id TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (teacher_id) REFERENCES teacher_profile(id)
);

-- ============================================================
-- Execution Message Table
-- 执行消息表，存储会话中的消息（用户输入和 AI 回复）
-- ============================================================
CREATE TABLE IF NOT EXISTS execution_message (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_name TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES execution_session(id)
);

-- ============================================================
-- Execution Record Table
-- 执行记录表，存储每次 AI 执行的详细元数据和结果摘要
-- ============================================================
CREATE TABLE IF NOT EXISTS execution_record (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    execution_message_id TEXT NOT NULL,
    entrypoint TEXT NOT NULL,
    agent_profile_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    status TEXT NOT NULL,
    reasoning_summary TEXT,
    search_summary_json TEXT,
    tool_calls_summary_json TEXT,
    error_message TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES execution_session(id),
    FOREIGN KEY (execution_message_id) REFERENCES execution_message(id)
);

-- ============================================================
-- Indexes for Execution Runtime
-- ============================================================

-- 按教师ID查询会话
CREATE INDEX IF NOT EXISTS idx_execution_session_teacher_id ON execution_session(teacher_id);

-- 按会话ID查询消息
CREATE INDEX IF NOT EXISTS idx_execution_message_session_id ON execution_message(session_id);

-- 按会话ID查询执行记录
CREATE INDEX IF NOT EXISTS idx_execution_record_session_id ON execution_record(session_id);

-- 按执行消息ID查询执行记录（一对一关系）
CREATE INDEX IF NOT EXISTS idx_execution_record_message_id ON execution_record(execution_message_id);

-- 按状态查询执行记录（用于失败重试或监控）
CREATE INDEX IF NOT EXISTS idx_execution_record_status ON execution_record(status);

-- 按创建时间查询（用于排序和分页）
CREATE INDEX IF NOT EXISTS idx_execution_session_created_at ON execution_session(created_at);
CREATE INDEX IF NOT EXISTS idx_execution_record_created_at ON execution_record(created_at);
