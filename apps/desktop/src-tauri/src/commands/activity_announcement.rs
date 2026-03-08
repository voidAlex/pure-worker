//! 活动公告命令入口
//!
//! 提供活动公告相关 Tauri IPC 命令。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::State;

use crate::error::AppError;
use crate::models::activity_announcement::{
    ActivityAnnouncement, CreateActivityAnnouncementInput, ListActivityAnnouncementsInput,
    UpdateActivityAnnouncementInput,
};
use crate::services;

/// 删除活动公告输入
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteActivityAnnouncementInput {
    pub id: String,
}

/// 删除活动公告响应
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct DeleteActivityAnnouncementResponse {
    pub success: bool,
}

/// 列出活动公告
#[tauri::command]
#[specta::specta]
pub async fn list_activity_announcements(
    pool: State<'_, SqlitePool>,
    input: ListActivityAnnouncementsInput,
) -> Result<Vec<ActivityAnnouncement>, AppError> {
    services::activity_announcement::ActivityAnnouncementService::list_by_class(&pool, input).await
}

/// 创建活动公告
#[tauri::command]
#[specta::specta]
pub async fn create_activity_announcement(
    pool: State<'_, SqlitePool>,
    input: CreateActivityAnnouncementInput,
) -> Result<ActivityAnnouncement, AppError> {
    services::activity_announcement::ActivityAnnouncementService::create(&pool, input).await
}

/// 更新活动公告
#[tauri::command]
#[specta::specta]
pub async fn update_activity_announcement(
    pool: State<'_, SqlitePool>,
    input: UpdateActivityAnnouncementInput,
) -> Result<ActivityAnnouncement, AppError> {
    services::activity_announcement::ActivityAnnouncementService::update(&pool, input).await
}

/// 删除活动公告
#[tauri::command]
#[specta::specta]
pub async fn delete_activity_announcement(
    pool: State<'_, SqlitePool>,
    input: DeleteActivityAnnouncementInput,
) -> Result<DeleteActivityAnnouncementResponse, AppError> {
    services::activity_announcement::ActivityAnnouncementService::delete(&pool, &input.id).await?;
    Ok(DeleteActivityAnnouncementResponse { success: true })
}
