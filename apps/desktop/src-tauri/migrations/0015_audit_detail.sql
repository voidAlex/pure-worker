-- 为审计日志表添加详细信息字段
-- Version: 0002
-- Description: 支持在审计日志中记录结构化的操作详情（如导入统计、文件路径等）

ALTER TABLE audit_log ADD COLUMN detail_json TEXT;
