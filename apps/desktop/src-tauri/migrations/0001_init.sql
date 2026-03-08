-- PureWorker Database Initialization Migration
-- Version: 0001
-- Description: Create all core tables, FTS5 virtual table, indexes and triggers
-- 
-- PRAGMA settings (applied via connection hooks in Rust):
--   - journal_mode = WAL
--   - synchronous = NORMAL  
--   - busy_timeout = 5000
--   - foreign_keys = ON
--
-- Tables use soft delete (is_deleted INTEGER NOT NULL DEFAULT 0)
-- Primary keys as TEXT (UUID)
-- Timestamp fields as TEXT (ISO 8601)

-- ============================================================
-- Teacher Profile Table
-- ============================================================
CREATE TABLE IF NOT EXISTS teacher_profile (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    stage TEXT NOT NULL,
    subject TEXT NOT NULL,
    textbook_version TEXT,
    tone_preset TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ============================================================
-- Classroom Table
-- ============================================================
CREATE TABLE IF NOT EXISTS classroom (
    id TEXT PRIMARY KEY,
    grade TEXT NOT NULL,
    class_name TEXT NOT NULL,
    subject TEXT NOT NULL,
    teacher_id TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (teacher_id) REFERENCES teacher_profile(id)
);

-- ============================================================
-- Student Table
-- ============================================================
CREATE TABLE IF NOT EXISTS student (
    id TEXT PRIMARY KEY,
    student_no TEXT NOT NULL,
    name TEXT NOT NULL,
    gender TEXT,
    class_id TEXT NOT NULL,
    meta_json TEXT,
    folder_path TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- ============================================================
-- Student Tag Table
-- ============================================================
CREATE TABLE IF NOT EXISTS student_tag (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    tag_name TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id)
);

-- ============================================================
-- Conversation Table
-- ============================================================
CREATE TABLE IF NOT EXISTS conversation (
    id TEXT PRIMARY KEY,
    teacher_id TEXT NOT NULL,
    title TEXT,
    scenario TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (teacher_id) REFERENCES teacher_profile(id)
);

-- ============================================================
-- Conversation Message Table
-- ============================================================
CREATE TABLE IF NOT EXISTS conversation_message (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_name TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversation(id)
);

-- ============================================================
-- Student File Map Table
-- ============================================================
CREATE TABLE IF NOT EXISTS student_file_map (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT,
    source TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id)
);

-- ============================================================
-- Memory Index Table
-- ============================================================
CREATE TABLE IF NOT EXISTS memory_index (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    class_id TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    summary TEXT,
    created_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (student_id) REFERENCES student(id),
    FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- ============================================================
-- Observation Note Table (FTS source)
-- ============================================================
CREATE TABLE IF NOT EXISTS observation_note (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT,
    created_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id)
);

-- ============================================================
-- Parent Communication Table (FTS source)
-- ============================================================
CREATE TABLE IF NOT EXISTS parent_communication (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    draft TEXT,
    adopted_text TEXT,
    status TEXT,
    evidence_json TEXT,
    created_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id)
);

-- ============================================================
-- Score Record Table
-- ============================================================
CREATE TABLE IF NOT EXISTS score_record (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    exam_name TEXT NOT NULL,
    subject TEXT NOT NULL,
    score REAL NOT NULL,
    full_score REAL NOT NULL,
    rank_in_class INTEGER,
    exam_date TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id)
);

-- ============================================================
-- Async Task Table
-- ============================================================
CREATE TABLE IF NOT EXISTS async_task (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    target_id TEXT,
    status TEXT NOT NULL,
    progress_json TEXT,
    context_data TEXT,
    checkpoint_cursor TEXT,
    completed_items_json TEXT,
    partial_output_path TEXT,
    lease_until TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at TEXT,
    worker_id TEXT,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ============================================================
-- Assignment Asset Table
-- ============================================================
CREATE TABLE IF NOT EXISTS assignment_asset (
    id TEXT PRIMARY KEY,
    class_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    hash TEXT,
    captured_at TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- ============================================================
-- Assignment OCR Result Table
-- ============================================================
CREATE TABLE IF NOT EXISTS assignment_ocr_result (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL,
    job_id TEXT,
    student_id TEXT NOT NULL,
    question_no TEXT,
    answer_text TEXT,
    confidence REAL,
    score REAL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (asset_id) REFERENCES assignment_asset(id),
    FOREIGN KEY (student_id) REFERENCES student(id)
);

-- ============================================================
-- Question Bank Table
-- ============================================================
CREATE TABLE IF NOT EXISTS question_bank (
    id TEXT PRIMARY KEY,
    source TEXT,
    knowledge_point TEXT,
    difficulty TEXT,
    stem TEXT NOT NULL,
    answer TEXT,
    explanation TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- ============================================================
-- Schedule Event Table
-- ============================================================
CREATE TABLE IF NOT EXISTS schedule_event (
    id TEXT PRIMARY KEY,
    class_id TEXT NOT NULL,
    title TEXT NOT NULL,
    start_at TEXT NOT NULL,
    end_at TEXT,
    linked_file_id TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- ============================================================
-- Template File Table
-- ============================================================
CREATE TABLE IF NOT EXISTS template_file (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    school_scope TEXT,
    version TEXT,
    file_path TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

-- ============================================================
-- Skill Registry Table
-- ============================================================
CREATE TABLE IF NOT EXISTS skill_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT,
    source TEXT,
    permission_scope TEXT,
    status TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

-- ============================================================
-- MCP Server Registry Table
-- ============================================================
CREATE TABLE IF NOT EXISTS mcp_server_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    transport TEXT NOT NULL,
    command TEXT,
    args_json TEXT,
    env_json TEXT,
    permission_scope TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

-- ============================================================
-- Audit Log Table
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT,
    risk_level TEXT NOT NULL,
    confirmed_by_user INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

-- ============================================================
-- Approval Request Table
-- ============================================================
CREATE TABLE IF NOT EXISTS approval_request (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    request_type TEXT NOT NULL,
    action_summary TEXT NOT NULL,
    params_preview TEXT,
    risk_level TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    resolved_by TEXT,
    resolved_at TEXT,
    timeout_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES async_task(id)
);

-- ============================================================
-- FTS5 Virtual Table for Full-Text Search
-- ============================================================
CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
    source_table,
    source_id,
    student_id,
    class_id,
    content,
    created_at,
    tokenize = 'unicode61'
);

-- ============================================================
-- FTS Sync Triggers for observation_note
-- ============================================================
CREATE TRIGGER IF NOT EXISTS obs_note_ai AFTER INSERT ON observation_note
WHEN new.is_deleted = 0
BEGIN
    INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
    VALUES('observation_note', new.id, new.student_id, 
           (SELECT class_id FROM student WHERE id = new.student_id), 
           new.content, new.created_at);
END;

CREATE TRIGGER IF NOT EXISTS obs_note_au AFTER UPDATE ON observation_note
BEGIN
    DELETE FROM memory_fts WHERE source_table = 'observation_note' AND source_id = old.id;
    INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
    SELECT 'observation_note', new.id, new.student_id, 
           (SELECT class_id FROM student WHERE id = new.student_id), 
           new.content, new.created_at
    WHERE new.is_deleted = 0;
END;

CREATE TRIGGER IF NOT EXISTS obs_note_ad AFTER DELETE ON observation_note
BEGIN
    DELETE FROM memory_fts WHERE source_table = 'observation_note' AND source_id = old.id;
END;

-- ============================================================
-- FTS Sync Triggers for parent_communication
-- ============================================================
CREATE TRIGGER IF NOT EXISTS pc_ai AFTER INSERT ON parent_communication
WHEN new.is_deleted = 0
BEGIN
    INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
    VALUES('parent_communication', new.id, new.student_id,
           (SELECT class_id FROM student WHERE id = new.student_id),
           COALESCE(new.adopted_text, new.draft), new.created_at);
END;

CREATE TRIGGER IF NOT EXISTS pc_au AFTER UPDATE ON parent_communication
BEGIN
    DELETE FROM memory_fts WHERE source_table = 'parent_communication' AND source_id = old.id;
    INSERT INTO memory_fts(source_table, source_id, student_id, class_id, content, created_at)
    SELECT 'parent_communication', new.id, new.student_id,
           (SELECT class_id FROM student WHERE id = new.student_id),
           COALESCE(new.adopted_text, new.draft), new.created_at
    WHERE new.is_deleted = 0;
END;

CREATE TRIGGER IF NOT EXISTS pc_ad AFTER DELETE ON parent_communication
BEGIN
    DELETE FROM memory_fts WHERE source_table = 'parent_communication' AND source_id = old.id;
END;

-- ============================================================
-- Indexes for Performance (with WHERE is_deleted = 0 for soft delete)
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_student_active_class ON student(class_id) WHERE is_deleted = 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_student_tag_unique_active ON student_tag(student_id, tag_name) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_memory_index_active ON memory_index(student_id, class_id, created_at) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_async_task_claim ON async_task(status, lease_until, updated_at);
CREATE INDEX IF NOT EXISTS idx_student_no_class ON student(student_no, class_id) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_score_student_subject ON score_record(student_id, subject, exam_date);
CREATE INDEX IF NOT EXISTS idx_score_exam ON score_record(student_id, exam_name, subject);
CREATE INDEX IF NOT EXISTS idx_obs_note_student ON observation_note(student_id, created_at) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_pc_student ON parent_communication(student_id, created_at) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_ocr_result ON assignment_ocr_result(asset_id, student_id, question_no);
CREATE INDEX IF NOT EXISTS idx_conv_msg ON conversation_message(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_file_map_student ON student_file_map(student_id, file_type, created_at) WHERE is_deleted = 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_template_unique ON template_file(type, school_scope, version) WHERE is_deleted = 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_skill_unique ON skill_registry(name, version) WHERE is_deleted = 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_mcp_unique ON mcp_server_registry(name) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_approval_pending ON approval_request(status, timeout_at) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_approval_task ON approval_request(task_id) WHERE status = 'pending';
