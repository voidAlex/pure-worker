use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::teacher_profile::TeacherProfile;

#[tauri::command]
#[specta::specta]
pub fn get_teacher_profile() -> Result<TeacherProfile, AppError> {
    let now = Utc::now().to_rfc3339();

    Ok(TeacherProfile {
        id: Uuid::new_v4().to_string(),
        name: String::from("示例教师"),
        stage: String::from("未设置学段"),
        subject: String::from("未设置学科"),
        textbook_version: None,
        tone_preset: None,
        is_deleted: 0,
        created_at: now.clone(),
        updated_at: now,
    })
}
