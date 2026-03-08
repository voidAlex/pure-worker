-- 创建课表文件表，用于管理课表事件关联的教案/课件文件
-- Version: 0003
-- Description: 支持课表事件关联文件（教案/课件等）

CREATE TABLE IF NOT EXISTS schedule_file (
    id TEXT PRIMARY KEY,
    class_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_type TEXT,
    file_size INTEGER,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    FOREIGN KEY (class_id) REFERENCES classroom(id)
);

-- 创建索引以提高查询性能
CREATE INDEX IF NOT EXISTS idx_schedule_file_class ON schedule_file(class_id) WHERE is_deleted = 0;
