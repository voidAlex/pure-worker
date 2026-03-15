use serde::{Deserialize, Serialize};
use specta::Type;

/// 创建教师档案的输入参数
#[derive(Debug, Clone, Deserialize, Type)]
pub struct CreateTeacherProfileInput {
    /// 教师姓名
    pub name: String,
    /// 任教学段
    pub teaching_stage: String,
    /// 任教学科
    pub teaching_subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct TeacherProfile {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub subject: String,
    pub textbook_version: Option<String>,
    pub tone_preset: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}
