//! 导出命令模块
//!
//! 提供导出相关 Tauri IPC 命令接口（健康检查、学期评语导出）。

use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{QueryBuilder, SqlitePool};
use tauri::State;

use crate::error::AppError;
use crate::services::audit::AuditService;

/// 导出学期评语输入
#[derive(Debug, Deserialize, Type)]
pub struct ExportSemesterCommentsInput {
    /// 任务 ID（按任务批次导出）
    pub task_id: Option<String>,
    /// 学期筛选
    pub term: Option<String>,
    /// 导出文件路径（由前端通过文件对话框获取）
    pub file_path: String,
}

/// 导出结果响应
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ExportSemesterCommentsResponse {
    /// 导出文件路径
    pub file_path: String,
    /// 导出记录条数
    pub exported_count: i32,
}

/// 学期评语导出查询行
#[derive(sqlx::FromRow)]
struct ExportRow {
    student_name: String,
    term: String,
    adopted_text: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub db_connected: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn health_check(pool: State<'_, SqlitePool>) -> Result<HealthCheckResponse, AppError> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&*pool)
        .await?;

    Ok(HealthCheckResponse {
        status: String::from("ok"),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_connected: true,
    })
}

/// 导出学期评语到 Excel 文件
#[tauri::command]
#[specta::specta]
pub async fn export_semester_comments(
    pool: State<'_, SqlitePool>,
    input: ExportSemesterCommentsInput,
) -> Result<ExportSemesterCommentsResponse, AppError> {
    if input.file_path.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("导出文件路径不能为空")));
    }

    let mut query = QueryBuilder::new(
        "SELECT s.name as student_name, sc.term, sc.adopted_text, sc.created_at \
         FROM semester_comment sc \
         JOIN student s ON sc.student_id = s.id AND s.is_deleted = 0 \
         WHERE sc.is_deleted = 0 AND sc.status = 'adopted'",
    );

    if let Some(task_id) = input.task_id.as_deref() {
        query.push(" AND sc.task_id = ").push_bind(task_id);
    }
    if let Some(term) = input.term.as_deref() {
        query.push(" AND sc.term = ").push_bind(term);
    }

    query.push(" ORDER BY s.name ASC");

    let rows = query
        .build_query_as::<ExportRow>()
        .fetch_all(&*pool)
        .await?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .write_string(0, 0, "学生姓名")
        .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
    worksheet
        .write_string(0, 1, "学期")
        .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
    worksheet
        .write_string(0, 2, "评语内容")
        .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
    worksheet
        .write_string(0, 3, "生成时间")
        .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;

    for (idx, row) in rows.iter().enumerate() {
        let current_row = (idx + 1) as u32;
        worksheet
            .write_string(current_row, 0, &row.student_name)
            .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
        worksheet
            .write_string(current_row, 1, &row.term)
            .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
        worksheet
            .write_string(current_row, 2, row.adopted_text.as_deref().unwrap_or(""))
            .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
        worksheet
            .write_string(current_row, 3, &row.created_at)
            .map_err(|e| AppError::FileOperation(format!("Excel 写入失败：{e}")))?;
    }

    workbook
        .save(&input.file_path)
        .map_err(|e| AppError::FileOperation(format!("Excel 保存失败：{e}")))?;

    AuditService::log(
        &pool,
        "system",
        "export_semester_comments",
        "semester_comment",
        None,
        "high",
        true,
    )
    .await?;

    Ok(ExportSemesterCommentsResponse {
        file_path: input.file_path,
        exported_count: rows.len() as i32,
    })
}
