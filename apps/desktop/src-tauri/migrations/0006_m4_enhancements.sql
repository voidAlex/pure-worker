-- M4 作业/考评与题库增强迁移
-- Version: 0006
-- Description: 新增 grading_job 表、扩展 assignment_ocr_result 和 question_bank 表字段
--
-- 变更内容：
-- 1. 新增 grading_job 表：关联一次批改任务的配置和状态
-- 2. assignment_ocr_result 新增冲突标记和复核字段
-- 3. question_bank 新增题型、学生来源等字段
-- 4. 新增 wrong_answer_record 表：错题记录
-- 5. 新增 practice_sheet 表：练习卷记录

-- ============================================================
-- Grading Job Table（批改任务表）
-- 关联一次批改的配置（班级、模式、标准答案等）和状态
-- ============================================================
CREATE TABLE IF NOT EXISTS grading_job (
    id TEXT PRIMARY KEY,
    class_id TEXT NOT NULL,
    title TEXT NOT NULL,
    grading_mode TEXT NOT NULL DEFAULT 'basic',
    status TEXT NOT NULL DEFAULT 'pending',
    answer_key_json TEXT,
    scoring_rules_json TEXT,
    total_assets INTEGER NOT NULL DEFAULT 0,
    processed_assets INTEGER NOT NULL DEFAULT 0,
    failed_assets INTEGER NOT NULL DEFAULT 0,
    conflict_count INTEGER NOT NULL DEFAULT 0,
    task_id TEXT,
    output_path TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (class_id) REFERENCES classroom(id),
    FOREIGN KEY (task_id) REFERENCES async_task(id)
);

-- ============================================================
-- assignment_asset 新增字段：关联 grading_job
-- ============================================================
ALTER TABLE assignment_asset ADD COLUMN job_id TEXT REFERENCES grading_job(id);
ALTER TABLE assignment_asset ADD COLUMN original_filename TEXT;
ALTER TABLE assignment_asset ADD COLUMN file_size INTEGER;
ALTER TABLE assignment_asset ADD COLUMN mime_type TEXT;
ALTER TABLE assignment_asset ADD COLUMN image_width INTEGER;
ALTER TABLE assignment_asset ADD COLUMN image_height INTEGER;
ALTER TABLE assignment_asset ADD COLUMN preprocess_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE assignment_asset ADD COLUMN preprocessed_path TEXT;
ALTER TABLE assignment_asset ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';

-- ============================================================
-- assignment_ocr_result 新增字段：冲突标记、复核状态、多模态结果
-- ============================================================
ALTER TABLE assignment_ocr_result ADD COLUMN ocr_raw_text TEXT;
ALTER TABLE assignment_ocr_result ADD COLUMN multimodal_score REAL;
ALTER TABLE assignment_ocr_result ADD COLUMN multimodal_feedback TEXT;
ALTER TABLE assignment_ocr_result ADD COLUMN conflict_flag INTEGER NOT NULL DEFAULT 0;
ALTER TABLE assignment_ocr_result ADD COLUMN review_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE assignment_ocr_result ADD COLUMN reviewed_by TEXT;
ALTER TABLE assignment_ocr_result ADD COLUMN reviewed_at TEXT;
ALTER TABLE assignment_ocr_result ADD COLUMN final_score REAL;
ALTER TABLE assignment_ocr_result ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0;
ALTER TABLE assignment_ocr_result ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';

-- ============================================================
-- question_bank 新增字段：题型、学科、年级、标签
-- ============================================================
ALTER TABLE question_bank ADD COLUMN question_type TEXT;
ALTER TABLE question_bank ADD COLUMN subject TEXT;
ALTER TABLE question_bank ADD COLUMN grade TEXT;
ALTER TABLE question_bank ADD COLUMN tags_json TEXT;
ALTER TABLE question_bank ADD COLUMN template_params_json TEXT;
ALTER TABLE question_bank ADD COLUMN parent_id TEXT REFERENCES question_bank(id);

-- ============================================================
-- Wrong Answer Record Table（错题记录表）
-- 记录学生某次批改中的错题信息
-- ============================================================
CREATE TABLE IF NOT EXISTS wrong_answer_record (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    ocr_result_id TEXT NOT NULL,
    question_no TEXT NOT NULL,
    knowledge_point TEXT,
    difficulty TEXT,
    student_answer TEXT,
    correct_answer TEXT,
    score REAL,
    full_score REAL,
    error_type TEXT,
    is_resolved INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id),
    FOREIGN KEY (job_id) REFERENCES grading_job(id),
    FOREIGN KEY (ocr_result_id) REFERENCES assignment_ocr_result(id)
);

-- ============================================================
-- Practice Sheet Table（练习卷表）
-- 记录为学生生成的专属练习卷
-- ============================================================
CREATE TABLE IF NOT EXISTS practice_sheet (
    id TEXT PRIMARY KEY,
    student_id TEXT NOT NULL,
    title TEXT NOT NULL,
    knowledge_points_json TEXT,
    difficulty TEXT,
    question_count INTEGER NOT NULL DEFAULT 0,
    questions_json TEXT,
    answers_json TEXT,
    file_path TEXT,
    answer_file_path TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    task_id TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (student_id) REFERENCES student(id),
    FOREIGN KEY (task_id) REFERENCES async_task(id)
);

-- ============================================================
-- 索引
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_grading_job_class ON grading_job(class_id, status) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_grading_job_task ON grading_job(task_id);
CREATE INDEX IF NOT EXISTS idx_asset_job ON assignment_asset(job_id) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_ocr_result_review ON assignment_ocr_result(review_status, conflict_flag) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_ocr_result_job ON assignment_ocr_result(job_id) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_wrong_answer_student ON wrong_answer_record(student_id, created_at) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_wrong_answer_job ON wrong_answer_record(job_id) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_wrong_answer_knowledge ON wrong_answer_record(knowledge_point) WHERE is_deleted = 0 AND is_resolved = 0;
CREATE INDEX IF NOT EXISTS idx_practice_sheet_student ON practice_sheet(student_id, created_at) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_question_bank_type ON question_bank(question_type, subject, difficulty) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_question_bank_parent ON question_bank(parent_id) WHERE is_deleted = 0;
