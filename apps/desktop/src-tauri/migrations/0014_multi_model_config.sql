-- Migration: 0014_multi_model_config
-- Description: Add multi-model configuration fields to ai_config table
-- Version: 0014
-- WP-AI-005: 多模型配置支持
--
-- 说明：
-- 1. 添加多模型配置字段，支持按任务类型选择不同模型
-- 2. 保留 default_model 字段用于向后兼容
-- 3. 新字段均为可选（NULLABLE），逐步迁移

-- ============================================================
-- 添加多模型配置字段到 ai_config 表
-- ============================================================

-- 默认文本对话模型
ALTER TABLE ai_config ADD COLUMN default_text_model TEXT;

-- 默认视觉/多模态模型
ALTER TABLE ai_config ADD COLUMN default_vision_model TEXT;

-- 默认工具调用模型
ALTER TABLE ai_config ADD COLUMN default_tool_model TEXT;

-- 默认推理增强模型
ALTER TABLE ai_config ADD COLUMN default_reasoning_model TEXT;

-- ============================================================
-- 数据迁移：将现有 default_model 值复制到 default_text_model
-- ============================================================
UPDATE ai_config SET default_text_model = default_model WHERE default_text_model IS NULL;

-- ============================================================
-- 为多模型字段创建索引
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_ai_config_text_model ON ai_config(default_text_model) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_ai_config_vision_model ON ai_config(default_vision_model) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_ai_config_tool_model ON ai_config(default_tool_model) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_ai_config_reasoning_model ON ai_config(default_reasoning_model) WHERE is_deleted = 0;
