//! 审计日志服务模块
//!
//! 提供操作审计日志的记录和查询功能

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::audit_log::AuditLog;

/// 审计服务，负责记录和查询操作日志
pub struct AuditService;

impl AuditService {
    /// 记录审计日志（无详细信息）
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `actor`: 操作者标识
    /// - `action`: 操作类型
    /// - `target_type`: 目标资源类型
    /// - `target_id`: 目标资源ID（可选）
    /// - `risk_level`: 风险等级（low/medium/high）
    /// - `confirmed_by_user`: 是否经过用户确认
    pub async fn log(
        pool: &SqlitePool,
        actor: &str,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        risk_level: &str,
        confirmed_by_user: bool,
    ) -> Result<(), AppError> {
        Self::log_with_detail(
            pool,
            actor,
            action,
            target_type,
            target_id,
            risk_level,
            confirmed_by_user,
            None,
        )
        .await
    }

    /// 记录审计日志（带详细信息）
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `actor`: 操作者标识
    /// - `action`: 操作类型
    /// - `target_type`: 目标资源类型
    /// - `target_id`: 目标资源ID（可选）
    /// - `risk_level`: 风险等级（low/medium/high）
    /// - `confirmed_by_user`: 是否经过用户确认
    /// - `detail_json`: 结构化详细信息（JSON格式，可选）
    #[allow(clippy::too_many_arguments)]
    pub async fn log_with_detail(
        pool: &SqlitePool,
        actor: &str,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        risk_level: &str,
        confirmed_by_user: bool,
        detail_json: Option<&str>,
    ) -> Result<(), AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO audit_log (id, actor, action, target_type, target_id, risk_level, confirmed_by_user, detail_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(actor)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(risk_level)
        .bind(confirmed_by_user as i32)
        .bind(detail_json)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 查询审计日志列表
    ///
    /// # 参数
    /// - `pool`: 数据库连接池
    /// - `limit`: 返回记录数量限制
    pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<AuditLog>, AppError> {
        let logs = sqlx::query_as::<_, AuditLog>(
            "SELECT id, actor, action, target_type, target_id, risk_level, confirmed_by_user, detail_json, created_at FROM audit_log ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(logs)
    }
}
