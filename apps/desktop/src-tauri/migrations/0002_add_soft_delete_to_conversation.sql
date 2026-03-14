-- Migration: 0002_add_soft_delete_to_conversation
-- Description: Add is_deleted column to conversation tables for soft delete support

-- Add is_deleted to conversation table
ALTER TABLE conversation ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0;

-- Add is_deleted to conversation_message table
ALTER TABLE conversation_message ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0;

-- Update indexes to include soft delete filter
CREATE INDEX IF NOT EXISTS idx_conversation_teacher_active 
ON conversation(teacher_id, updated_at) 
WHERE is_deleted = 0;

CREATE INDEX IF NOT EXISTS idx_conversation_msg_active 
ON conversation_message(conversation_id, created_at) 
WHERE is_deleted = 0;
