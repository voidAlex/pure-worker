use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct TeacherProfile {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub subject: String,
}

#[tauri::command]
#[specta::specta]
pub fn get_teacher_profile() -> Result<TeacherProfile, AppError> {
    Ok(TeacherProfile {
        id: String::from("teacher-placeholder"),
        name: String::from("示例教师"),
        stage: String::from("未设置学段"),
        subject: String::from("未设置学科"),
    })
}
