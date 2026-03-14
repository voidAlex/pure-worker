# WP-AI-003: AI Chat with Streaming Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement streaming AI chat with conversation persistence (multi-turn support), including database soft delete fix and frontend streaming UI.

**Architecture:** 
- Backend: Extend existing chat command with streaming support using Tauri events + Rig streaming API
- Frontend: React hook `useChatStream` + UI component with word-by-word rendering
- Database: Add `is_deleted` to conversation tables + create migration

**Tech Stack:** Tauri 2.x, Rust (Rig), React/TypeScript, SQLite, sqlx

**User Decisions Applied:**
- No stop button for Phase 1 MVP
- Auto-title: first 50 chars of user message
- Error handling: show error, user re-sends manually
- WP-AI-003 IN Phase 1 scope (not separate)

---

## Overview

This plan implements WP-AI-003 (AI Chat with Streaming) including:
1. Database migration to add soft delete (`is_deleted`) to conversation tables
2. Backend streaming IPC command `chat_stream` with Tauri events
3. Frontend `useChatStream` hook with streaming state management
4. UI components for streaming chat display
5. Conversation persistence (create, list, get messages)

---

## File Structure

### New Files (Backend)
- `apps/desktop/src-tauri/migrations/0002_add_soft_delete_to_conversation.sql`
- `apps/desktop/src-tauri/src/models/conversation.rs`
- `apps/desktop/src-tauri/src/services/conversation_service.rs`
- `apps/desktop/src-tauri/src/commands/conversation.rs`

### Modified Files (Backend)
- `apps/desktop/src-tauri/src/models/mod.rs`
- `apps/desktop/src-tauri/src/services/mod.rs`
- `apps/desktop/src-tauri/src/commands/mod.rs`
- `apps/desktop/src-tauri/src/commands/chat.rs`
- `apps/desktop/src-tauri/src/lib.rs`

### New Files (Frontend)
- `apps/desktop/src/hooks/useChatStream.ts`
- `apps/desktop/src/components/chat/ChatPanel.tsx`
- `apps/desktop/src/components/chat/ChatMessage.tsx`
- `apps/desktop/src/components/chat/ChatInput.tsx`
- `apps/desktop/src/services/chatService.ts`

### Modified Files (Frontend)
- `apps/desktop/src/pages/DashboardPage.tsx`

---

## Task Dependencies & Execution Order

```
┌─────────────────────────────────────────────────────────────────┐
│                         PARALLEL GROUP 1                        │
│  (Database + Backend Models - No Dependencies)                  │
├─────────────────────────────────────────────────────────────────┤
│  T-001: Create migration for soft delete                        │
│  T-002: Create conversation models                              │
│  T-003: Create conversation service                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PARALLEL GROUP 2                           │
│  (Backend Commands - Depends on Group 1)                        │
├─────────────────────────────────────────────────────────────────┤
│  T-004: Create conversation commands                            │
│  T-005: Implement streaming chat command                        │
│  T-006: Register new commands in lib.rs                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PARALLEL GROUP 3                           │
│  (Frontend - Depends on Group 2 bindings)                       │
├─────────────────────────────────────────────────────────────────┤
│  T-007: Create useChatStream hook                               │
│  T-008: Create chat service layer                               │
│  T-009: Create ChatPanel component                              │
│  T-010: Create ChatMessage component                            │
│  T-011: Create ChatInput component                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         GROUP 4                                 │
│  (Integration - Depends on Group 3)                             │
├─────────────────────────────────────────────────────────────────┤
│  T-012: Integrate ChatPanel into DashboardPage                  │
│  T-013: Run full verification (Rust + TypeScript)               │
└─────────────────────────────────────────────────────────────────┘
```

---

## Task Details

### T-001: Database Migration - Add Soft Delete to Conversation Tables

**Files:**
- Create: `apps/desktop/src-tauri/migrations/0002_add_soft_delete_to_conversation.sql`

**Dependencies:** None

**Category:** database

**Skills:** rust, sql

**Description:**
Create migration to add `is_deleted` column to both `conversation` and `conversation_message` tables, and update existing queries.

- [ ] **Step 1: Write migration file**

```sql
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
```

- [ ] **Step 2: Verify migration syntax**

Run: `cd apps/desktop/src-tauri && cargo sqlx migrate run --dry-run`
Expected: No syntax errors

- [ ] **Step 3: Commit**

```bash
git add migrations/0002_add_soft_delete_to_conversation.sql
git commit -m "feat: add soft delete migration for conversation tables"
```

**Success Criteria:**
- Migration file exists with proper SQL
- Includes `is_deleted` column for both tables
- Includes updated indexes with `WHERE is_deleted = 0`

---

### T-002: Create Conversation Models

**Files:**
- Create: `apps/desktop/src-tauri/src/models/conversation.rs`
- Modify: `apps/desktop/src-tauri/src/models/mod.rs`

**Dependencies:** T-001

**Category:** backend

**Skills:** rust

**Description:**
Create Rust data models for conversation and conversation_message with serde serialization.

- [ ] **Step 1: Create conversation.rs model file**

```rust
//! 对话会话数据模型
//!
//! 定义会话(conversation)和消息(conversation_message)的数据结构

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::FromRow;

/// 会话实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct Conversation {
    pub id: String,
    pub teacher_id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
    pub is_deleted: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 会话消息实体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, Type)]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,  // user, assistant, system, tool
    pub content: String,
    pub tool_name: Option<String>,
    pub is_deleted: i32,
    pub created_at: DateTime<Utc>,
}

/// 创建会话输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateConversationInput {
    pub teacher_id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
}

/// 更新会话输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateConversationInput {
    pub id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
}

/// 创建消息输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateMessageInput {
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
}

/// 会话列表项（简化输出）
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConversationListItem {
    pub id: String,
    pub title: Option<String>,
    pub scenario: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i64,
}

/// 消息列表项
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MessageListItem {
    pub id: String,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 流式聊天请求输入
#[derive(Debug, Deserialize, Type)]
pub struct ChatStreamInput {
    pub conversation_id: Option<String>,  // None = new conversation
    pub message: String,
    pub agent_role: String,
}

/// 流式聊天事件类型（用于Tauri事件）
#[derive(Debug, Clone, Serialize, Type)]
pub enum ChatStreamEvent {
    /// 开始生成
    Start { message_id: String },
    /// 内容片段（增量）
    Chunk { content: String },
    /// 生成完成
    Complete,
    /// 生成错误
    Error { message: String },
}
```

- [ ] **Step 2: Update models/mod.rs**

Add to `models/mod.rs`:

```rust
pub mod conversation;
```

- [ ] **Step 3: Verify compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/models/conversation.rs src/models/mod.rs
git commit -m "feat: add conversation data models with soft delete support"
```

**Success Criteria:**
- `Conversation` and `ConversationMessage` models defined
- Includes `is_deleted` field
- Input/Output DTOs defined
- `ChatStreamEvent` enum for streaming events
- Compiles without errors

---

### T-003: Create Conversation Service

**Files:**
- Create: `apps/desktop/src-tauri/src/services/conversation_service.rs`
- Modify: `apps/desktop/src-tauri/src/services/mod.rs`

**Dependencies:** T-002

**Category:** backend

**Skills:** rust, sqlx

**Description:**
Create service layer for conversation CRUD operations with soft delete filtering.

- [ ] **Step 1: Create conversation_service.rs**

```rust
//! 对话会话服务层
//!
//! 提供会话和消息的增删改查，支持软删除过滤

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::conversation::{
    Conversation, ConversationListItem, ConversationMessage, CreateConversationInput,
    CreateMessageInput, MessageListItem, UpdateConversationInput,
};

/// 对话服务
pub struct ConversationService;

impl ConversationService {
    /// 创建新会话
    pub async fn create_conversation(
        pool: &SqlitePool,
        input: CreateConversationInput,
    ) -> Result<Conversation, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO conversation (id, teacher_id, title, scenario, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, 0, ?, ?)"
        )
        .bind(&id)
        .bind(&input.teacher_id)
        .bind(&input.title)
        .bind(&input.scenario)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        Self::get_conversation_by_id(pool, &id).await
    }

    /// 根据ID获取会话（仅未删除）
    pub async fn get_conversation_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<Conversation, AppError> {
        let conversation = sqlx::query_as::<_, Conversation>(
            "SELECT id, teacher_id, title, scenario, is_deleted, created_at, updated_at FROM conversation WHERE id = ? AND is_deleted = 0"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("会话不存在：{}", id)))?;

        Ok(conversation)
    }

    /// 获取教师的会话列表
    pub async fn list_conversations(
        pool: &SqlitePool,
        teacher_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationListItem>, AppError> {
        let items = sqlx::query_as::<_, ConversationListItem>(
            r#"
            SELECT 
                c.id,
                c.title,
                c.scenario,
                c.created_at,
                c.updated_at,
                COUNT(cm.id) as message_count
            FROM conversation c
            LEFT JOIN conversation_message cm ON c.id = cm.conversation_id AND cm.is_deleted = 0
            WHERE c.teacher_id = ? AND c.is_deleted = 0
            GROUP BY c.id
            ORDER BY c.updated_at DESC
            LIMIT ? OFFSET ?
            "#
        )
        .bind(teacher_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    /// 更新会话
    pub async fn update_conversation(
        pool: &SqlitePool,
        input: UpdateConversationInput,
    ) -> Result<Conversation, AppError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE conversation SET title = COALESCE(?, title), scenario = COALESCE(?, scenario), updated_at = ? WHERE id = ? AND is_deleted = 0"
        )
        .bind(&input.title)
        .bind(&input.scenario)
        .bind(&now)
        .bind(&input.id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("会话不存在：{}", input.id)));
        }

        Self::get_conversation_by_id(pool, &input.id).await
    }

    /// 软删除会话
    pub async fn delete_conversation(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let now = Utc::now();

        // 软删除会话
        let result = sqlx::query(
            "UPDATE conversation SET is_deleted = 1, updated_at = ? WHERE id = ? AND is_deleted = 0"
        )
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("会话不存在：{}", id)));
        }

        // 软删除该会话的所有消息
        sqlx::query(
            "UPDATE conversation_message SET is_deleted = 1 WHERE conversation_id = ?"
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 创建消息
    pub async fn create_message(
        pool: &SqlitePool,
        input: CreateMessageInput,
    ) -> Result<ConversationMessage, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO conversation_message (id, conversation_id, role, content, tool_name, is_deleted, created_at) VALUES (?, ?, ?, ?, ?, 0, ?)"
        )
        .bind(&id)
        .bind(&input.conversation_id)
        .bind(&input.role)
        .bind(&input.content)
        .bind(&input.tool_name)
        .bind(&now)
        .execute(pool)
        .await?;

        // 更新会话的 updated_at
        sqlx::query(
            "UPDATE conversation SET updated_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(&input.conversation_id)
        .execute(pool)
        .await?;

        Self::get_message_by_id(pool, &id).await
    }

    /// 根据ID获取消息
    pub async fn get_message_by_id(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<ConversationMessage, AppError> {
        let message = sqlx::query_as::<_, ConversationMessage>(
            "SELECT id, conversation_id, role, content, tool_name, is_deleted, created_at FROM conversation_message WHERE id = ? AND is_deleted = 0"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("消息不存在：{}", id)))?;

        Ok(message)
    }

    /// 获取会话的消息列表
    pub async fn list_messages(
        pool: &SqlitePool,
        conversation_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MessageListItem>, AppError> {
        let messages = sqlx::query_as::<_, MessageListItem>(
            "SELECT id, role, content, tool_name, created_at FROM conversation_message WHERE conversation_id = ? AND is_deleted = 0 ORDER BY created_at ASC LIMIT ? OFFSET ?"
        )
        .bind(conversation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    /// 获取会话的完整消息历史（用于AI上下文）
    pub async fn get_conversation_history(
        pool: &SqlitePool,
        conversation_id: &str,
        limit: i64,
    ) -> Result<Vec<ConversationMessage>, AppError> {
        let messages = sqlx::query_as::<_, ConversationMessage>(
            "SELECT id, conversation_id, role, content, tool_name, is_deleted, created_at FROM conversation_message WHERE conversation_id = ? AND is_deleted = 0 ORDER BY created_at ASC LIMIT ?"
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    /// 软删除消息
    pub async fn delete_message(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE conversation_message SET is_deleted = 1 WHERE id = ? AND is_deleted = 0"
        )
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("消息不存在：{}", id)));
        }

        Ok(())
    }

    /// 生成会话标题（取用户消息前50字符）
    pub fn generate_title(message: &str) -> String {
        let trimmed = message.trim();
        if trimmed.len() <= 50 {
            trimmed.to_string()
        } else {
            format!("{}...", &trimmed[..50])
        }
    }
}
```

- [ ] **Step 2: Update services/mod.rs**

Add to `services/mod.rs`:

```rust
pub mod conversation_service;
```

- [ ] **Step 3: Verify compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/services/conversation_service.rs src/services/mod.rs
git commit -m "feat: add conversation service with soft delete filtering"
```

**Success Criteria:**
- All CRUD operations implemented
- `is_deleted = 0` filter applied in all queries
- `generate_title` function for auto-title
- Compiles without errors

---

### T-004: Create Conversation Commands

**Files:**
- Create: `apps/desktop/src-tauri/src/commands/conversation.rs`
- Modify: `apps/desktop/src-tauri/src/commands/mod.rs`

**Dependencies:** T-003

**Category:** backend

**Skills:** rust, tauri

**Description:**
Create IPC commands for conversation management (create, list, get, delete, get messages).

- [ ] **Step 1: Create conversation.rs commands file**

```rust
//! 对话会话 IPC 命令模块
//!
//! 提供前端会话管理的 IPC 接口

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::conversation::{
    Conversation, ConversationListItem, CreateConversationInput, MessageListItem,
    UpdateConversationInput,
};
use crate::services::conversation_service::ConversationService;

/// 列出会话请求
#[derive(Debug, Deserialize, Type)]
pub struct ListConversationsInput {
    pub teacher_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 列出会话响应
#[derive(Debug, Serialize, Type)]
pub struct ListConversationsResponse {
    pub conversations: Vec<ConversationListItem>,
    pub total: i64,
}

/// 获取消息请求
#[derive(Debug, Deserialize, Type)]
pub struct GetMessagesInput {
    pub conversation_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 创建会话 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn create_conversation(
    pool: State<'_, SqlitePool>,
    input: CreateConversationInput,
) -> Result<Conversation, AppError> {
    ConversationService::create_conversation(&pool, input).await
}

/// 列出会话 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn list_conversations(
    pool: State<'_, SqlitePool>,
    input: ListConversationsInput,
) -> Result<ListConversationsResponse, AppError> {
    let limit = input.limit.unwrap_or(50);
    let offset = input.offset.unwrap_or(0);

    let conversations = ConversationService::list_conversations(&pool, &input.teacher_id, limit, offset).await?;
    let total = conversations.len() as i64; // Simplified - could add COUNT query

    Ok(ListConversationsResponse {
        conversations,
        total,
    })
}

/// 获取会话详情 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn get_conversation(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<Conversation, AppError> {
    ConversationService::get_conversation_by_id(&pool, &id).await
}

/// 更新会话 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn update_conversation(
    pool: State<'_, SqlitePool>,
    input: UpdateConversationInput,
) -> Result<Conversation, AppError> {
    ConversationService::update_conversation(&pool, input).await
}

/// 删除会话 IPC 命令（软删除）
#[tauri::command]
#[specta::specta]
pub async fn delete_conversation(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), AppError> {
    ConversationService::delete_conversation(&pool, &id).await
}

/// 获取会话消息列表 IPC 命令
#[tauri::command]
#[specta::specta]
pub async fn list_conversation_messages(
    pool: State<'_, SqlitePool>,
    input: GetMessagesInput,
) -> Result<Vec<MessageListItem>, AppError> {
    let limit = input.limit.unwrap_or(100);
    let offset = input.offset.unwrap_or(0);

    ConversationService::list_messages(&pool, &input.conversation_id, limit, offset).await
}
```

- [ ] **Step 2: Update commands/mod.rs**

Add to `commands/mod.rs`:

```rust
pub mod conversation;
```

- [ ] **Step 3: Verify compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/commands/conversation.rs src/commands/mod.rs
git commit -m "feat: add conversation IPC commands"
```

**Success Criteria:**
- All conversation IPC commands implemented
- Proper error handling with AppError
- Compiles without errors

---

### T-005: Implement Streaming Chat Command

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/chat.rs`

**Dependencies:** T-004

**Category:** backend

**Skills:** rust, tauri, rig

**Description:**
Implement `chat_stream` command that emits Tauri events for streaming responses.

- [ ] **Step 1: Update chat.rs with streaming command**

Add to existing `chat.rs`:

```rust
// ... existing imports ...

use crate::models::conversation::{
    ChatStreamEvent, ChatStreamInput, ConversationMessage, CreateConversationInput,
    CreateMessageInput,
};
use crate::services::conversation_service::ConversationService;

// ... existing code ...

/// 流式 AI 对话命令
/// 
/// 通过 Tauri 事件流式返回 AI 响应，支持多轮对话
#[tauri::command]
#[specta::specta]
pub async fn chat_stream(
    app: tauri::AppHandle,
    pool: State<'_, SqlitePool>,
    input: ChatStreamInput,
) -> Result<String, AppError> {
    if input.message.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("消息内容不能为空")));
    }

    // 获取或创建会话
    let conversation_id = if let Some(id) = input.conversation_id {
        // 验证会话存在
        ConversationService::get_conversation_by_id(&pool, &id).await?;
        id
    } else {
        // 创建新会话
        let teacher_id = get_current_teacher_id(&pool).await?;
        let title = ConversationService::generate_title(&input.message);
        let conversation = ConversationService::create_conversation(
            &pool,
            CreateConversationInput {
                teacher_id,
                title: Some(title),
                scenario: Some(input.agent_role.clone()),
            },
        ).await?;
        conversation.id
    };

    // 保存用户消息
    let user_message = ConversationService::create_message(
        &pool,
        CreateMessageInput {
            conversation_id: conversation_id.clone(),
            role: "user".to_string(),
            content: input.message.clone(),
            tool_name: None,
        },
    ).await?;

    // 创建 AI 消息占位
    let assistant_message = ConversationService::create_message(
        &pool,
        CreateMessageInput {
            conversation_id: conversation_id.clone(),
            role: "assistant".to_string(),
            content: String::new(),
            tool_name: None,
        },
    ).await?;

    // 发送开始事件
    let _ = app.emit(
        "chat-stream",
        ChatStreamEvent::Start {
            message_id: assistant_message.id.clone(),
        },
    );

    // 获取 AI 配置
    let config = LlmProviderService::get_active_config(&pool).await?;
    let client = LlmProviderService::create_client(&config)?;
    let system_prompt = get_system_prompt(&input.agent_role);

    // 获取对话历史
    let history = ConversationService::get_conversation_history(&pool, &conversation_id, 20).await?;

    // 构建消息列表（历史 + 当前）
    let mut messages = build_message_history(&history);
    messages.push(rig::message::Message {
        role: rig::message::Role::User,
        content: input.message.clone(),
    });

    // 流式生成响应
    let mut full_content = String::new();
    
    match stream_completion(&client, &config.default_model, system_prompt, messages, &app).await {
        Ok(content) => {
            full_content = content;
            
            // 更新 AI 消息
            let _ = ConversationService::update_message_content(
                &pool,
                &assistant_message.id,
                &full_content,
            ).await;

            // 发送完成事件
            let _ = app.emit("chat-stream", ChatStreamEvent::Complete);
        }
        Err(e) => {
            // 发送错误事件
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent::Error {
                    message: format!("AI 生成失败：{}", e),
                },
            );
            return Err(e);
        }
    }

    Ok(conversation_id)
}

/// 流式完成生成
async fn stream_completion(
    client: &openai::CompletionsClient,
    model: &str,
    system_prompt: &str,
    messages: Vec<rig::message::Message>,
    app: &tauri::AppHandle,
) -> Result<String, AppError> {
    let agent = client
        .agent(model)
        .preamble(system_prompt)
        .temperature(0.7)
        .build();

    // 使用 Rig 的流式 API
    let stream = agent
        .stream_messages(messages)
        .await
        .map_err(|e| AppError::ExternalService(format!("流式生成启动失败：{}", e)))?;

    let mut full_content = String::new();

    // 处理流式响应
    // Note: Rig 的流式 API 返回的是 Stream，我们需要消费它
    // 这里使用 futures::StreamExt
    use futures::StreamExt;

    let mut stream = stream;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => {
                full_content.push_str(&text);
                // 发送内容片段事件
                let _ = app.emit(
                    "chat-stream",
                    ChatStreamEvent::Chunk {
                        content: text,
                    },
                );
            }
            Err(e) => {
                return Err(AppError::ExternalService(format!("流式生成中断：{}", e)));
            }
        }
    }

    Ok(full_content)
}

/// 构建消息历史（过滤掉当前正在生成的消息）
fn build_message_history(history: &[ConversationMessage]) -> Vec<rig::message::Message> {
    history
        .iter()
        .filter(|m| !m.content.is_empty()) // 过滤空内容（正在生成的消息）
        .map(|m| rig::message::Message {
            role: match m.role.as_str() {
                "user" => rig::message::Role::User,
                "assistant" => rig::message::Role::Assistant,
                "system" => rig::message::Role::System,
                _ => rig::message::Role::User,
            },
            content: m.content.clone(),
        })
        .collect()
}

/// 获取当前教师ID（简化版，从配置中获取）
async fn get_current_teacher_id(pool: &SqlitePool) -> Result<String, AppError> {
    // 从 teacher_profile 表获取第一个教师
    let teacher_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM teacher_profile WHERE is_deleted = 0 LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    teacher_id.ok_or_else(|| AppError::NotFound(String::from("未找到教师档案")))
}

/// 添加到 ConversationService 的 update_message_content 方法
/// 需要在 conversation_service.rs 中添加：
impl ConversationService {
    pub async fn update_message_content(
        pool: &SqlitePool,
        id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE conversation_message SET content = ? WHERE id = ?"
        )
        .bind(content)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
```

- [ ] **Step 2: Add update_message_content to service**

Add to `conversation_service.rs` in `ConversationService` impl:

```rust
    /// 更新消息内容（用于流式生成完成后更新）
    pub async fn update_message_content(
        pool: &SqlitePool,
        id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE conversation_message SET content = ? WHERE id = ? AND is_deleted = 0"
        )
        .bind(content)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src/commands/chat.rs src/services/conversation_service.rs
git commit -m "feat: implement streaming chat command with Tauri events"
```

**Success Criteria:**
- `chat_stream` command implemented
- Emits Tauri events: Start, Chunk, Complete, Error
- Saves messages to database
- Handles multi-turn with conversation_id
- Compiles without errors

---

### T-006: Register Commands in lib.rs

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Dependencies:** T-005

**Category:** backend

**Skills:** rust, tauri

**Description:**
Register all new IPC commands in the Tauri builder.

- [ ] **Step 1: Update lib.rs command registration**

Add to `create_specta_builder()` in `lib.rs`:

```rust
        commands::conversation::create_conversation,
        commands::conversation::list_conversations,
        commands::conversation::get_conversation,
        commands::conversation::update_conversation,
        commands::conversation::delete_conversation,
        commands::conversation::list_conversation_messages,
        commands::chat::chat_stream,
```

Add to the `collect_commands!` macro call.

- [ ] **Step 2: Verify compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: No errors

- [ ] **Step 3: Generate TypeScript bindings**

Run: `cd apps/desktop && pnpm tauri dev` (or wait for auto-generation)
Expected: `src/bindings.ts` updated with new types

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs ../src/bindings.ts
git commit -m "feat: register conversation and streaming chat commands"
```

**Success Criteria:**
- All new commands registered
- TypeScript bindings generated
- Compiles without errors

---

### T-007: Create useChatStream Hook

**Files:**
- Create: `apps/desktop/src/hooks/useChatStream.ts`

**Dependencies:** T-006 (for bindings)

**Category:** frontend

**Skills:** typescript, react

**Description:**
Create React hook for managing streaming chat state and Tauri event listeners.

- [ ] **Step 1: Create useChatStream.ts**

```typescript
/**
 * 流式聊天 Hook
 * 
 * 管理聊天状态、流式响应、事件监听
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { listen, Event } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/core';
import { ChatStreamEvent, ChatStreamInput } from '@/bindings';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  tool_name?: string;
  created_at: string;
  isStreaming?: boolean;
}

export interface UseChatStreamOptions {
  conversationId?: string;
  agentRole?: string;
  onError?: (error: string) => void;
}

export interface UseChatStreamReturn {
  messages: ChatMessage[];
  isStreaming: boolean;
  error: string | null;
  sendMessage: (message: string) => Promise<void>;
  clearError: () => void;
}

export function useChatStream(options: UseChatStreamOptions = {}): UseChatStreamReturn {
  const { conversationId: initialConversationId, agentRole = 'homeroom', onError } = options;
  
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentConversationId, setCurrentConversationId] = useState<string | undefined>(initialConversationId);
  
  const streamingMessageIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  // 设置事件监听
  useEffect(() => {
    let isActive = true;

    const setupListener = async () => {
      const unlisten = await listen<ChatStreamEvent>('chat-stream', (event: Event<ChatStreamEvent>) => {
        if (!isActive) return;
        
        const payload = event.payload;
        
        switch (payload.type) {
          case 'Start':
            setIsStreaming(true);
            setError(null);
            streamingMessageIdRef.current = payload.message_id;
            // 添加占位消息
            setMessages(prev => [...prev, {
              id: payload.message_id,
              role: 'assistant',
              content: '',
              created_at: new Date().toISOString(),
              isStreaming: true,
            }]);
            break;
            
          case 'Chunk':
            setMessages(prev => {
              const lastMessage = prev[prev.length - 1];
              if (lastMessage && lastMessage.isStreaming) {
                return [
                  ...prev.slice(0, -1),
                  {
                    ...lastMessage,
                    content: lastMessage.content + payload.content,
                  },
                ];
              }
              return prev;
            });
            break;
            
          case 'Complete':
            setIsStreaming(false);
            streamingMessageIdRef.current = null;
            setMessages(prev => {
              const lastMessage = prev[prev.length - 1];
              if (lastMessage && lastMessage.isStreaming) {
                return [
                  ...prev.slice(0, -1),
                  {
                    ...lastMessage,
                    isStreaming: false,
                  },
                ];
              }
              return prev;
            });
            break;
            
          case 'Error':
            setIsStreaming(false);
            setError(payload.message);
            streamingMessageIdRef.current = null;
            if (onError) {
              onError(payload.message);
            }
            break;
        }
      });
      
      unlistenRef.current = unlisten;
    };

    setupListener();

    return () => {
      isActive = false;
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, [onError]);

  // 发送消息
  const sendMessage = useCallback(async (message: string) => {
    if (!message.trim() || isStreaming) return;

    // 添加用户消息到列表
    const userMessageId = `user-${Date.now()}`;
    setMessages(prev => [...prev, {
      id: userMessageId,
      role: 'user',
      content: message.trim(),
      created_at: new Date().toISOString(),
    }]);

    setError(null);

    try {
      const input: ChatStreamInput = {
        conversation_id: currentConversationId,
        message: message.trim(),
        agent_role: agentRole,
      };

      const result = await invoke<string>('chat_stream', { input });
      
      // 如果是新会话，保存会话ID
      if (!currentConversationId && result) {
        setCurrentConversationId(result);
      }
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : '发送消息失败';
      setError(errorMessage);
      setIsStreaming(false);
      if (onError) {
        onError(errorMessage);
      }
    }
  }, [currentConversationId, agentRole, isStreaming, onError]);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    messages,
    isStreaming,
    error,
    sendMessage,
    clearError,
  };
}
```

- [ ] **Step 2: Verify TypeScript types**

Run: `cd apps/desktop && pnpm tsc --noEmit`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/hooks/useChatStream.ts
git commit -m "feat: add useChatStream hook for streaming chat"
```

**Success Criteria:**
- Hook manages streaming state
- Listens to Tauri events
- Handles Start/Chunk/Complete/Error events
- TypeScript types pass

---

### T-008: Create Chat Service Layer

**Files:**
- Create: `apps/desktop/src/services/chatService.ts`

**Dependencies:** T-006 (for bindings)

**Category:** frontend

**Skills:** typescript

**Description:**
Create service layer for conversation management API calls.

- [ ] **Step 1: Create chatService.ts**

```typescript
/**
 * 聊天服务层
 * 
 * 封装对话相关的 IPC 调用
 */

import { invoke } from '@tauri-apps/core';
import {
  Conversation,
  ConversationListItem,
  MessageListItem,
  CreateConversationInput,
  UpdateConversationInput,
  ListConversationsInput,
  ListConversationsResponse,
  GetMessagesInput,
} from '@/bindings';

export interface ConversationFilters {
  teacherId: string;
  limit?: number;
  offset?: number;
}

export interface MessageFilters {
  conversationId: string;
  limit?: number;
  offset?: number;
}

/**
 * 创建新会话
 */
export async function createConversation(
  input: CreateConversationInput
): Promise<Conversation> {
  return invoke<Conversation>('create_conversation', { input });
}

/**
 * 获取会话列表
 */
export async function listConversations(
  filters: ConversationFilters
): Promise<ListConversationsResponse> {
  const input: ListConversationsInput = {
    teacher_id: filters.teacherId,
    limit: filters.limit,
    offset: filters.offset,
  };
  return invoke<ListConversationsResponse>('list_conversations', { input });
}

/**
 * 获取会话详情
 */
export async function getConversation(id: string): Promise<Conversation> {
  return invoke<Conversation>('get_conversation', { id });
}

/**
 * 更新会话
 */
export async function updateConversation(
  input: UpdateConversationInput
): Promise<Conversation> {
  return invoke<Conversation>('update_conversation', { input });
}

/**
 * 删除会话（软删除）
 */
export async function deleteConversation(id: string): Promise<void> {
  return invoke<void>('delete_conversation', { id });
}

/**
 * 获取会话消息列表
 */
export async function listConversationMessages(
  filters: MessageFilters
): Promise<MessageListItem[]> {
  const input: GetMessagesInput = {
    conversation_id: filters.conversationId,
    limit: filters.limit,
    offset: filters.offset,
  };
  return invoke<MessageListItem[]>('list_conversation_messages', { input });
}
```

- [ ] **Step 2: Verify TypeScript types**

Run: `cd apps/desktop && pnpm tsc --noEmit`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/services/chatService.ts
git commit -m "feat: add chat service layer for conversation management"
```

**Success Criteria:**
- All conversation API functions implemented
- Proper TypeScript types
- No type errors

---

### T-009: Create ChatPanel Component

**Files:**
- Create: `apps/desktop/src/components/chat/ChatPanel.tsx`

**Dependencies:** T-007, T-008

**Category:** frontend

**Skills:** typescript, react, tailwind

**Description:**
Create main chat panel component integrating useChatStream hook.

- [ ] **Step 1: Create ChatPanel.tsx**

```typescript
/**
 * 聊天面板组件
 * 
 * 主聊天界面，包含消息列表和输入框
 */

import React, { useEffect, useState } from 'react';
import { useChatStream } from '@/hooks/useChatStream';
import { ChatMessageList } from './ChatMessageList';
import { ChatInput } from './ChatInput';

export interface ChatPanelProps {
  conversationId?: string;
  agentRole?: string;
  className?: string;
}

export function ChatPanel({ conversationId, agentRole = 'homeroom', className = '' }: ChatPanelProps) {
  const [mounted, setMounted] = useState(false);
  
  const {
    messages,
    isStreaming,
    error,
    sendMessage,
    clearError,
  } = useChatStream({
    conversationId,
    agentRole,
  });

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return null;
  }

  return (
    <div className={`flex flex-col h-full bg-white ${className}`}>
      {/* 消息列表 */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="flex items-center justify-center h-full text-gray-400">
            <div className="text-center">
              <p className="text-lg mb-2">开始新的对话</p>
              <p className="text-sm">输入消息与 AI 助手交流</p>
            </div>
          </div>
        ) : (
          <ChatMessageList messages={messages} />
        )}
        
        {/* 流式指示器 */}
        {isStreaming && (
          <div className="flex items-center gap-2 text-gray-400 text-sm">
            <div className="flex gap-1">
              <div className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
              <div className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
              <div className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
            </div>
            <span>AI 正在思考...</span>
          </div>
        )}
      </div>

      {/* 错误提示 */}
      {error && (
        <div className="px-4 py-2 bg-red-50 border-t border-red-100">
          <div className="flex items-center justify-between">
            <span className="text-red-600 text-sm">{error}</span>
            <button
              onClick={clearError}
              className="text-red-400 hover:text-red-600 text-sm"
            >
              清除
            </button>
          </div>
        </div>
      )}

      {/* 输入框 */}
      <div className="border-t border-gray-200 p-4">
        <ChatInput
          onSend={sendMessage}
          disabled={isStreaming}
          placeholder="输入消息..."
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript types**

Run: `cd apps/desktop && pnpm tsc --noEmit`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/chat/ChatPanel.tsx
git commit -m "feat: add ChatPanel component for streaming chat UI"
```

**Success Criteria:**
- Component integrates useChatStream hook
- Displays messages, streaming state, errors
- TypeScript types pass

---

### T-010: Create ChatMessageList Component

**Files:**
- Create: `apps/desktop/src/components/chat/ChatMessageList.tsx`
- Create: `apps/desktop/src/components/chat/ChatMessage.tsx`

**Dependencies:** T-009

**Category:** frontend

**Skills:** typescript, react, tailwind

**Description:**
Create message list and individual message components.

- [ ] **Step 1: Create ChatMessage.tsx**

```typescript
/**
 * 单条聊天消息组件
 */

import React from 'react';
import { ChatMessage as ChatMessageType } from '@/hooks/useChatStream';

export interface ChatMessageProps {
  message: ChatMessageType;
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user';
  
  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[80%] rounded-2xl px-4 py-3 ${
          isUser
            ? 'bg-blue-500 text-white'
            : 'bg-gray-100 text-gray-800'
        }`}
      >
        {/* 角色标签 */}
        <div className={`text-xs mb-1 ${isUser ? 'text-blue-200' : 'text-gray-500'}`}>
          {isUser ? '我' : 'AI 助手'}
        </div>
        
        {/* 消息内容 */}
        <div className="whitespace-pre-wrap leading-relaxed">
          {message.content || (message.isStreaming ? '' : '...')}
        </div>
        
        {/* 工具调用标识 */}
        {message.tool_name && (
          <div className={`text-xs mt-2 ${isUser ? 'text-blue-200' : 'text-gray-500'}`}>
            使用了工具: {message.tool_name}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create ChatMessageList.tsx**

```typescript
/**
 * 聊天消息列表组件
 */

import React, { useRef, useEffect } from 'react';
import { ChatMessage } from './ChatMessage';
import { ChatMessage as ChatMessageType } from '@/hooks/useChatStream';

export interface ChatMessageListProps {
  messages: ChatMessageType[];
}

export function ChatMessageList({ messages }: ChatMessageListProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  // 自动滚动到底部
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  return (
    <div className="space-y-4">
      {messages.map((message, index) => (
        <ChatMessage key={message.id || index} message={message} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
```

- [ ] **Step 3: Verify TypeScript types**

Run: `cd apps/desktop && pnpm tsc --noEmit`
Expected: No type errors

- [ ] **Step 4: Commit**

```bash
git add src/components/chat/ChatMessage.tsx src/components/chat/ChatMessageList.tsx
git commit -m "feat: add ChatMessage and ChatMessageList components"
```

**Success Criteria:**
- Message components render correctly
- Auto-scroll to bottom
- Different styles for user/assistant
- TypeScript types pass

---

### T-011: Create ChatInput Component

**Files:**
- Create: `apps/desktop/src/components/chat/ChatInput.tsx`

**Dependencies:** T-010

**Category:** frontend

**Skills:** typescript, react, tailwind

**Description:**
Create chat input component with send button and enter key support.

- [ ] **Step 1: Create ChatInput.tsx**

```typescript
/**
 * 聊天输入框组件
 */

import React, { useState, useCallback, KeyboardEvent } from 'react';

export interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export function ChatInput({ onSend, disabled = false, placeholder = '输入消息...' }: ChatInputProps) {
  const [message, setMessage] = useState('');

  const handleSend = useCallback(() => {
    if (message.trim() && !disabled) {
      onSend(message.trim());
      setMessage('');
    }
  }, [message, disabled, onSend]);

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }, [handleSend]);

  return (
    <div className="flex gap-2">
      <textarea
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder={placeholder}
        rows={1}
        className="flex-1 resize-none rounded-lg border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-500"
        style={{ minHeight: '44px', maxHeight: '120px' }}
      />
      <button
        onClick={handleSend}
        disabled={disabled || !message.trim()}
        className="px-6 py-2 bg-blue-500 text-white rounded-lg font-medium disabled:bg-gray-300 disabled:cursor-not-allowed hover:bg-blue-600 transition-colors"
      >
        发送
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript types**

Run: `cd apps/desktop && pnpm tsc --noEmit`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/components/chat/ChatInput.tsx
git commit -m "feat: add ChatInput component with enter key support"
```

**Success Criteria:**
- Input component with textarea
- Send on Enter key
- Disabled state support
- TypeScript types pass

---

### T-012: Integrate ChatPanel into DashboardPage

**Files:**
- Modify: `apps/desktop/src/pages/DashboardPage.tsx`

**Dependencies:** T-011

**Category:** frontend

**Skills:** typescript, react

**Description:**
Integrate ChatPanel into the Dashboard page.

- [ ] **Step 1: Update DashboardPage.tsx**

```typescript
// ... existing imports ...
import { ChatPanel } from '@/components/chat/ChatPanel';

export function DashboardPage() {
  // ... existing code ...

  return (
    <div className="flex h-full">
      {/* 主工作区 */}
      <div className="flex-1 flex flex-col">
        {/* ... existing content ... */}
      </div>
      
      {/* 右侧 AI 面板 */}
      <div className="w-[360px] border-l border-gray-200 bg-white">
        <div className="h-full">
          <div className="px-4 py-3 border-b border-gray-200">
            <h2 className="font-medium text-gray-800">AI 助手</h2>
          </div>
          <div className="h-[calc(100%-57px)]">
            <ChatPanel agentRole="homeroom" />
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript types**

Run: `cd apps/desktop && pnpm tsc --noEmit`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/pages/DashboardPage.tsx
git commit -m "feat: integrate ChatPanel into DashboardPage"
```

**Success Criteria:**
- ChatPanel integrated into DashboardPage
- TypeScript types pass

---

### T-013: Run Full Verification

**Files:** All

**Dependencies:** T-012

**Category:** verification

**Skills:** rust, typescript

**Description:**
Run all verification commands to ensure code quality.

- [ ] **Step 1: Run Rust checks**

```bash
cd apps/desktop/src-tauri

# Format check
cargo fmt --check

# Clippy check
cargo clippy -- -D warnings

# Tests
cargo test
```

Expected:
- Format: clean
- Clippy: no warnings
- Tests: all pass

- [ ] **Step 2: Run TypeScript checks**

```bash
cd apps/desktop

# Type check
pnpm tsc --noEmit

# ESLint
pnpm eslint src/

# Prettier check
pnpm prettier --check src/
```

Expected:
- TypeScript: no errors
- ESLint: no errors
- Prettier: clean

- [ ] **Step 3: Integration test (manual)**

Run: `pnpm tauri dev`

Manual QA:
- [ ] Open chat → type message → see streaming text appear word-by-word
- [ ] Refresh page → conversation persists
- [ ] Send second message in same conversation → both messages visible

- [ ] **Step 4: Final commit**

```bash
git commit -m "feat: WP-AI-003 complete - AI chat with streaming and conversation persistence"
```

**Success Criteria:**
- `cargo clippy -- -D warnings` passes
- `cargo test` passes
- `pnpm tsc --noEmit` passes
- `pnpm eslint src/` passes
- Manual QA passes

---

## Parallel Execution Groups Summary

| Group | Tasks | Dependencies | Category |
|-------|-------|--------------|----------|
| **Group 1** | T-001, T-002, T-003 | None | Database + Backend Models |
| **Group 2** | T-004, T-005, T-006 | Group 1 | Backend Commands |
| **Group 3** | T-007, T-008, T-009, T-010, T-011 | Group 2 | Frontend |
| **Group 4** | T-012, T-013 | Group 3 | Integration + Verification |

---

## Task to Skills Mapping

| Task | Primary Skills |
|------|---------------|
| T-001 | rust, sql, database |
| T-002 | rust, sqlx |
| T-003 | rust, sqlx |
| T-004 | rust, tauri |
| T-005 | rust, tauri, rig |
| T-006 | rust, tauri |
| T-007 | typescript, react |
| T-008 | typescript |
| T-009 | typescript, react, tailwind |
| T-010 | typescript, react, tailwind |
| T-011 | typescript, react, tailwind |
| T-012 | typescript, react |
| T-013 | rust, typescript |

---

## Verification Checklist

### Backend (Rust)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Migration `0002_add_soft_delete_to_conversation.sql` exists
- [ ] New IPC commands registered in `lib.rs`

### Frontend (TypeScript)
- [ ] `pnpm tsc --noEmit` passes
- [ ] `pnpm eslint src/` passes
- [ ] `pnpm prettier --check src/` passes
- [ ] `useChatStream` hook exists
- [ ] Chat components exist

### Integration
- [ ] `chat_stream` command exists and compiles
- [ ] TypeScript bindings generated
- [ ] Streaming events working
- [ ] Conversation persistence working

### Manual QA
- [ ] Open chat → type message → see streaming text appear word-by-word
- [ ] Refresh page → conversation persists
- [ ] Send second message in same conversation → both messages visible

---

## Notes

### Database Soft Delete
- All queries MUST include `WHERE is_deleted = 0`
- Delete operations update `is_deleted = 1` instead of physical delete
- Indexes updated to include soft delete filter

### Streaming Implementation
- Uses Tauri events (`chat-stream`) for real-time updates
- Event types: `Start`, `Chunk`, `Complete`, `Error`
- Frontend listens to events and updates UI incrementally

### Conversation Title
- Auto-generated from first 50 chars of user message
- No extra API call needed

### Error Handling
- Errors displayed in UI
- User manually retries by re-sending message
- No automatic retry mechanism (as per user choice)

### Stop Button
- NOT included in Phase 1 (as per user choice)
- Can be added in future phase

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Rig streaming API changes | Wrap in service layer, easy to swap |
| Tauri event performance | Use refs for unlisten, clean up properly |
| Large conversation history | Limit to last 20 messages for context |
| Database migration failure | Test migration on dev DB first |

---

*Plan generated: 2026-03-14*
*Based on: User decisions for WP-AI-003 Phase 1 MVP*
