-- WP-AI-011: 教师偏好记忆系统迁移
-- Version: 0013
-- Description: 创建教师偏好记忆表和候选记忆表，支持 soul.md / user.md 文件级记忆
--
-- 说明：
-- 1. 所有表使用 TEXT 主键（UUID）
-- 2. 所有表包含软删除字段 is_deleted（默认 0）
-- 3. 所有表包含 created_at / updated_at（ISO 8601 文本时间）
-- 4. 业务查询需显式包含 WHERE is_deleted = 0

-- ============================================================
-- Teacher Preference Table（教师偏好记忆表）
-- 存储教师显式设置、推断的偏好，支持多种类型和来源追踪
-- ============================================================
CREATE TABLE IF NOT EXISTS teacher_preference (
    id TEXT PRIMARY KEY,
    preference_key TEXT NOT NULL,
    preference_value TEXT NOT NULL,
    preference_type TEXT NOT NULL CHECK(preference_type IN ('output_style', 'tone', 'format', 'workflow', 'other')),
    source TEXT NOT NULL CHECK(source IN ('explicit', 'inferred', 'imported', 'default')),
    confirmed_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 唯一索引：同一 key 的活跃偏好（软删除排除）
CREATE UNIQUE INDEX IF NOT EXISTS idx_teacher_preference_key_active
ON teacher_preference(preference_key) WHERE is_deleted = 0 AND is_active = 1;

-- 类型索引：按偏好类型查询
CREATE INDEX IF NOT EXISTS idx_teacher_preference_type
ON teacher_preference(preference_type) WHERE is_deleted = 0 AND is_active = 1;

-- 来源索引：按来源查询
CREATE INDEX IF NOT EXISTS idx_teacher_preference_source
ON teacher_preference(source) WHERE is_deleted = 0;

-- ============================================================
-- Memory Candidate Table（候选记忆表）
-- 存储 AI 检测到的重复模式，等待教师确认
-- ============================================================
CREATE TABLE IF NOT EXISTS memory_candidate (
    id TEXT PRIMARY KEY,
    candidate_key TEXT NOT NULL,
    candidate_value TEXT NOT NULL,
    detected_count INTEGER NOT NULL DEFAULT 1,
    confidence_score REAL,
    pattern_evidence TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'confirmed', 'rejected')) DEFAULT 'pending',
    confirmed_at TEXT,
    rejected_at TEXT,
    rejection_reason TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 唯一索引：同一 key 的待处理候选
CREATE UNIQUE INDEX IF NOT EXISTS idx_memory_candidate_key_pending
ON memory_candidate(candidate_key) WHERE is_deleted = 0 AND status = 'pending';

-- 状态索引：按状态查询
CREATE INDEX IF NOT EXISTS idx_memory_candidate_status
ON memory_candidate(status) WHERE is_deleted = 0;

-- 检测次数索引：用于优先级排序
CREATE INDEX IF NOT EXISTS idx_memory_candidate_count
ON memory_candidate(detected_count DESC) WHERE is_deleted = 0 AND status = 'pending';

-- ============================================================
-- Preference Detection Pattern Table（偏好检测模式表）
-- 记录用于检测重复模式的历史数据
-- ============================================================
CREATE TABLE IF NOT EXISTS preference_detection_pattern (
    id TEXT PRIMARY KEY,
    pattern_type TEXT NOT NULL,
    pattern_key TEXT NOT NULL,
    pattern_value TEXT,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    last_occurred_at TEXT NOT NULL,
    context_hash TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 复合索引：按类型和 key 查询
CREATE UNIQUE INDEX IF NOT EXISTS idx_detection_pattern_type_key
ON preference_detection_pattern(pattern_type, pattern_key) WHERE is_deleted = 0;

-- ============================================================
-- 默认教师偏好数据初始化
-- 这些默认值在应用启动时会被加载到 soul.md 和 user.md
-- ============================================================
INSERT OR IGNORE INTO teacher_preference (id, preference_key, preference_value, preference_type, source, confirmed_at, is_active, is_deleted, created_at, updated_at) VALUES
-- 输出风格偏好
('pref-001', 'output_style.default', 'structured', 'output_style', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-002', 'output_style.comments', 'detailed_with_examples', 'output_style', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-003', 'output_style.announcements', 'formal_with_bullet_points', 'output_style', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),

-- 语气偏好
('pref-004', 'tone.general', 'professional_friendly', 'tone', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-005', 'tone.parent_communication', 'empathetic_clear', 'tone', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-006', 'tone.student_feedback', 'encouraging_constructive', 'tone', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),

-- 格式偏好
('pref-007', 'format.date', 'YYYY-MM-DD', 'format', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-008', 'format.numbering', 'chinese_numerals', 'format', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-009', 'format.lists', 'hierarchical_with_emojis', 'format', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),

-- 工作流偏好
('pref-010', 'workflow.confirmation_required', 'high_risk_only', 'workflow', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-011', 'workflow.desensitize_default', 'true', 'workflow', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now')),
('pref-012', 'workflow.auto_save_interval', '30', 'workflow', 'default', datetime('now'), 1, 0, datetime('now'), datetime('now'));
