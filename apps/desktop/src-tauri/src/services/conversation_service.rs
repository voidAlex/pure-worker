//! 对话会话服务层
//!
//! 提供会话和消息的增删改查，支持软删除过滤

use chrono::Utc;
use sqlx::{Row, SqlitePool};
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
        let now = Utc::now().to_rfc3339();

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
        let rows = sqlx::query(
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
            "#,
        )
        .bind(teacher_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|row| ConversationListItem {
                id: row.get("id"),
                title: row.get("title"),
                scenario: row.get("scenario"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                message_count: row.get("message_count"),
            })
            .collect();

        Ok(items)
    }

    /// 更新会话
    pub async fn update_conversation(
        pool: &SqlitePool,
        input: UpdateConversationInput,
    ) -> Result<Conversation, AppError> {
        let now = Utc::now().to_rfc3339();

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
        let now = Utc::now().to_rfc3339();

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

        sqlx::query("UPDATE conversation_message SET is_deleted = 1 WHERE conversation_id = ?")
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
        let now = Utc::now().to_rfc3339();

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

        sqlx::query("UPDATE conversation SET updated_at = ? WHERE id = ?")
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
        let rows = sqlx::query(
            "SELECT id, role, content, tool_name, created_at FROM conversation_message WHERE conversation_id = ? AND is_deleted = 0 ORDER BY created_at ASC LIMIT ? OFFSET ?"
        )
        .bind(conversation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let messages = rows
            .into_iter()
            .map(|row| MessageListItem {
                id: row.get("id"),
                role: row.get("role"),
                content: row.get("content"),
                tool_name: row.get("tool_name"),
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(messages)
    }

    /// 获取会话的完整消息历史
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
            "UPDATE conversation_message SET is_deleted = 1 WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("消息不存在：{}", id)));
        }

        Ok(())
    }

    /// 更新消息内容
    pub async fn update_message_content(
        pool: &SqlitePool,
        id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE conversation_message SET content = ? WHERE id = ? AND is_deleted = 0")
            .bind(content)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// 生成会话标题
    pub fn generate_title(message: &str) -> String {
        let trimmed = message.trim();
        if trimmed.len() <= 50 {
            trimmed.to_string()
        } else {
            format!("{}...", &trimmed[..50])
        }
    }
}
