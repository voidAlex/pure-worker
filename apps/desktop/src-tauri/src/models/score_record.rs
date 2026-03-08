use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct ScoreRecord {
    pub id: String,
    pub student_id: String,
    pub exam_name: String,
    pub subject: String,
    pub score: f64,
    pub full_score: f64,
    pub rank_in_class: Option<i32>,
    pub exam_date: String,
    pub is_deleted: i32,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateScoreRecordInput {
    pub student_id: String,
    pub exam_name: String,
    pub subject: String,
    pub score: f64,
    pub full_score: f64,
    pub rank_in_class: Option<i32>,
    pub exam_date: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct UpdateScoreRecordInput {
    pub id: String,
    pub exam_name: Option<String>,
    pub subject: Option<String>,
    pub score: Option<f64>,
    pub full_score: Option<f64>,
    pub rank_in_class: Option<i32>,
    pub exam_date: Option<String>,
}
