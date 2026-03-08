//! 校本模板文件数据模型
//!
//! 定义模板文件的结构体及创建/更新/列表查询输入类型

use serde::{Deserialize, Serialize};
use specta::Type;

/// 校本模板文件记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct TemplateFile {
    pub id: String,
    #[sqlx(rename = "type")]
    pub r#type: String,
    pub school_scope: Option<String>,
    pub version: Option<String>,
    pub file_path: String,
    pub enabled: i32,
    pub is_deleted: i32,
    pub created_at: String,
}

/// 创建模板文件输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateTemplateFileInput {
    pub r#type: String,
    pub school_scope: Option<String>,
    pub version: Option<String>,
    pub file_path: String,
    pub enabled: Option<i32>,
}

/// 更新模板文件输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateTemplateFileInput {
    pub id: String,
    pub r#type: Option<String>,
    pub school_scope: Option<String>,
    pub version: Option<String>,
    pub file_path: Option<String>,
    pub enabled: Option<i32>,
}

/// 列表查询输入
#[derive(Debug, Deserialize, Type)]
pub struct ListTemplateFilesInput {
    pub r#type: Option<String>,
    pub enabled: Option<i32>,
}
