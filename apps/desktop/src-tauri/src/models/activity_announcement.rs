//! 活动公告数据模型
//!
//! 定义班会/活动公告的结构体及创建/更新/列表查询输入类型

use serde::{Deserialize, Serialize};
use specta::Type;

/// 活动公告记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ActivityAnnouncement {
    pub id: String,
    pub class_id: String,
    pub title: String,
    pub topic: Option<String>,
    pub audience: String,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub template_id: Option<String>,
    pub status: String,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建活动公告输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateActivityAnnouncementInput {
    pub class_id: String,
    pub title: String,
    pub topic: Option<String>,
    pub audience: Option<String>,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub template_id: Option<String>,
    pub status: Option<String>,
}

/// 更新活动公告输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateActivityAnnouncementInput {
    pub id: String,
    pub title: Option<String>,
    pub topic: Option<String>,
    pub audience: Option<String>,
    pub draft: Option<String>,
    pub adopted_text: Option<String>,
    pub template_id: Option<String>,
    pub status: Option<String>,
}

/// 列表查询输入
#[derive(Debug, Deserialize, Type)]
pub struct ListActivityAnnouncementsInput {
    pub class_id: String,
    pub audience: Option<String>,
}
