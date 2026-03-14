-- WP-AI-008: Multimodal Prompt Template Registry Migration
-- Description: Create prompt_template_registry table for multimodal template support
-- Date: 2026-03-14

-- ============================================================
-- Prompt Template Registry Table
-- ============================================================
-- Stores metadata for multimodal prompt templates with capability-based selection
CREATE TABLE IF NOT EXISTS prompt_template_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    task_type TEXT NOT NULL, -- chat/communication/grading/agentic_search_summary
    modality TEXT NOT NULL DEFAULT 'text', -- text/multimodal
    capability_requirements_json TEXT, -- JSON array of required capabilities: ["json_mode", "tool_calling", "vision"]
    output_protocol TEXT NOT NULL DEFAULT 'markdown', -- markdown/structured_json/draft_card
    fallback_template_id TEXT, -- ID of fallback template when capabilities not met
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (fallback_template_id) REFERENCES prompt_template_registry(id)
);

-- ============================================================
-- Indexes for Performance
-- ============================================================
-- Index for active template lookup by task type
CREATE INDEX IF NOT EXISTS idx_prompt_template_task_type ON prompt_template_registry(task_type, is_enabled) WHERE is_deleted = 0;

-- Index for modality-based filtering
CREATE INDEX IF NOT EXISTS idx_prompt_template_modality ON prompt_template_registry(modality, is_enabled) WHERE is_deleted = 0;

-- Index for unique template name+version combination
CREATE UNIQUE INDEX IF NOT EXISTS idx_prompt_template_unique ON prompt_template_registry(name, version) WHERE is_deleted = 0;

-- Index for fallback template lookups
CREATE INDEX IF NOT EXISTS idx_prompt_template_fallback ON prompt_template_registry(fallback_template_id) WHERE is_deleted = 0;
