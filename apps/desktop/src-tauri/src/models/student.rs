use serde::{Deserialize, Serialize};
use specta::Type;

use crate::models::observation_note::ObservationNote;
use crate::models::parent_communication::ParentCommunication;
use crate::models::score_record::ScoreRecord;
use crate::models::student_tag::StudentTag;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct Student {
    pub id: String,
    pub student_no: String,
    pub name: String,
    pub gender: Option<String>,
    pub class_id: String,
    pub meta_json: Option<String>,
    pub folder_path: String,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateStudentInput {
    pub student_no: String,
    pub name: String,
    pub gender: Option<String>,
    pub class_id: String,
    pub meta_json: Option<String>,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateStudentInput {
    pub id: String,
    pub student_no: Option<String>,
    pub name: Option<String>,
    pub gender: Option<String>,
    pub class_id: Option<String>,
    pub meta_json: Option<String>,
}

/// 学生 360 度全景视图聚合结构
#[derive(Debug, Serialize, Type)]
pub struct StudentProfile360 {
    /// 学生基本信息
    pub student: Student,
    /// 学生标签列表
    pub tags: Vec<StudentTag>,
    /// 最近成绩记录（最多 10 条）
    pub recent_scores: Vec<ScoreRecord>,
    /// 最近观察记录（最多 10 条）
    pub recent_observations: Vec<ObservationNote>,
    /// 最近家校沟通记录（最多 10 条）
    pub recent_communications: Vec<ParentCommunication>,
}
