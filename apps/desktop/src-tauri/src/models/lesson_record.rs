use serde::{Deserialize, Serialize};
use specta::Type;

/// 课程记录状态
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct LessonRecord {
    pub id: String,
    pub class_id: String,
    pub schedule_event_id: Option<String>,
    pub subject: String,
    pub lesson_date: String,
    pub lesson_index: Option<i32>,
    pub topic: Option<String>,
    pub teaching_goal: Option<String>,
    pub homework_summary: Option<String>,
    pub teacher_note: Option<String>,
    pub status: String,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建课程记录输入
#[derive(Debug, Deserialize, Type)]
pub struct CreateLessonRecordInput {
    pub class_id: String,
    pub schedule_event_id: Option<String>,
    pub subject: String,
    pub lesson_date: String,
    pub lesson_index: Option<i32>,
    pub topic: Option<String>,
    pub teaching_goal: Option<String>,
    pub homework_summary: Option<String>,
    pub teacher_note: Option<String>,
    pub status: Option<String>,
}

/// 更新课程记录输入
#[derive(Debug, Deserialize, Type)]
pub struct UpdateLessonRecordInput {
    pub id: String,
    pub subject: Option<String>,
    pub lesson_date: Option<String>,
    pub lesson_index: Option<i32>,
    pub topic: Option<String>,
    pub teaching_goal: Option<String>,
    pub homework_summary: Option<String>,
    pub teacher_note: Option<String>,
    pub status: Option<String>,
}

/// 查询课程记录列表输入
#[derive(Debug, Deserialize, Type)]
pub struct ListLessonRecordsInput {
    pub class_id: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub status: Option<String>,
}

/// 课程总结信息（聚合学生表现）
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LessonSummary {
    pub lesson_record_id: String,
    pub class_id: String,
    pub subject: String,
    pub lesson_date: String,
    pub topic: Option<String>,
    pub status: String,
    /// 关联的观察记录数量
    pub observation_count: i64,
    /// 关联的成绩记录数量
    pub score_count: i64,
    /// 关联的作业资产数量
    pub assignment_count: i64,
    /// 关联的家校沟通数量
    pub communication_count: i64,
}
