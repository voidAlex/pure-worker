use calamine::{open_workbook_auto, Data, Reader};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::student_import::{
    ImportDuplicateStrategy, ImportRowError, ImportStudentsInput, ImportStudentsResult,
};
use crate::services::audit::AuditService;

pub struct StudentImportService;

impl StudentImportService {
    pub async fn import(
        pool: &SqlitePool,
        input: ImportStudentsInput,
    ) -> Result<ImportStudentsResult, AppError> {
        let file_path = input.file_path.clone();
        let class_id = input.class_id.clone();
        let strategy = input.duplicate_strategy.clone();

        // Validate class exists
        let class_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(1) FROM classroom WHERE id = ? AND is_deleted = 0",
        )
        .bind(&class_id)
        .fetch_one(pool)
        .await?;

        if class_exists == 0 {
            return Err(AppError::InvalidInput(format!(
                "班级不存在或已删除：{class_id}"
            )));
        }

        // Read Excel file (blocking I/O, run in spawn_blocking)
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<StudentRow>, AppError> {
            let mut workbook = open_workbook_auto(&file_path)
                .map_err(|e| AppError::FileOperation(format!("无法打开文件：{e}")))?;

            let sheet_names = workbook.sheet_names().to_vec();
            if sheet_names.is_empty() {
                return Err(AppError::InvalidInput("Excel 文件没有工作表".into()));
            }

            let range = workbook
                .worksheet_range(&sheet_names[0])
                .map_err(|e| AppError::FileOperation(format!("无法读取工作表：{e}")))?;

            let mut rows_data = Vec::new();
            let mut header_map: Option<HeaderMap> = None;

            for (row_idx, row) in range.rows().enumerate() {
                if row_idx == 0 {
                    header_map = Some(HeaderMap::from_row(row)?);
                    continue;
                }

                let hm = header_map.as_ref().unwrap();
                let student_no = cell_to_string(row.get(hm.student_no_idx));
                let name = cell_to_string(row.get(hm.name_idx));
                let gender = hm.gender_idx.and_then(|idx| {
                    let v = cell_to_string(row.get(idx));
                    if v.is_empty() {
                        None
                    } else {
                        Some(v)
                    }
                });

                rows_data.push(StudentRow {
                    row_number: row_idx + 1,
                    student_no,
                    name,
                    gender,
                });
            }

            Ok(rows_data)
        })
        .await
        .map_err(|e| AppError::Internal(format!("读取文件任务失败：{e}")))??;

        let total_rows = rows.len();
        let mut created_count: usize = 0;
        let mut updated_count: usize = 0;
        let mut skipped_count: usize = 0;
        let mut error_count: usize = 0;
        let mut errors: Vec<ImportRowError> = Vec::new();

        let now = Utc::now().to_rfc3339();

        for row in &rows {
            // Validate required fields
            if row.student_no.is_empty() {
                errors.push(ImportRowError {
                    row_number: row.row_number,
                    field: "学号".into(),
                    reason: "学号不能为空".into(),
                    suggestion: "请填写学号".into(),
                });
                error_count += 1;
                continue;
            }
            if row.name.is_empty() {
                errors.push(ImportRowError {
                    row_number: row.row_number,
                    field: "姓名".into(),
                    reason: "姓名不能为空".into(),
                    suggestion: "请填写学生姓名".into(),
                });
                error_count += 1;
                continue;
            }

            // Check duplicate
            let existing = sqlx::query_scalar::<_, String>(
                "SELECT id FROM student WHERE student_no = ? AND class_id = ? AND is_deleted = 0",
            )
            .bind(&row.student_no)
            .bind(&input.class_id)
            .fetch_optional(pool)
            .await?;

            match existing {
                Some(existing_id) => {
                    match strategy {
                        ImportDuplicateStrategy::Skip => {
                            skipped_count += 1;
                        }
                        ImportDuplicateStrategy::Update => {
                            sqlx::query(
                                "UPDATE student SET name = ?, gender = COALESCE(?, gender), updated_at = ? WHERE id = ? AND is_deleted = 0",
                            )
                            .bind(&row.name)
                            .bind(&row.gender)
                            .bind(&now)
                            .bind(&existing_id)
                            .execute(pool)
                            .await?;
                            updated_count += 1;
                        }
                        ImportDuplicateStrategy::Add => {
                            // Add as new student with same student_no (allow duplicates)
                            let id = Uuid::new_v4().to_string();
                            let folder_path = format!("workspace/students/{id}/");
                            sqlx::query(
                                "INSERT INTO student (id, student_no, name, gender, class_id, folder_path, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
                            )
                            .bind(&id)
                            .bind(&row.student_no)
                            .bind(&row.name)
                            .bind(&row.gender)
                            .bind(&input.class_id)
                            .bind(&folder_path)
                            .bind(&now)
                            .bind(&now)
                            .execute(pool)
                            .await?;
                            created_count += 1;
                        }
                    }
                }
                None => {
                    let id = Uuid::new_v4().to_string();
                    let folder_path = format!("workspace/students/{id}/");
                    sqlx::query(
                        "INSERT INTO student (id, student_no, name, gender, class_id, folder_path, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
                    )
                    .bind(&id)
                    .bind(&row.student_no)
                    .bind(&row.name)
                    .bind(&row.gender)
                    .bind(&input.class_id)
                    .bind(&folder_path)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                    created_count += 1;
                }
            }
        }

        AuditService::log(
            pool,
            "system",
            "import_students",
            "student",
            None,
            "high",
            false,
        )
        .await?;

        Ok(ImportStudentsResult {
            total_rows,
            created_count,
            updated_count,
            skipped_count,
            error_count,
            errors,
        })
    }
}

struct StudentRow {
    row_number: usize,
    student_no: String,
    name: String,
    gender: Option<String>,
}

struct HeaderMap {
    student_no_idx: usize,
    name_idx: usize,
    gender_idx: Option<usize>,
}

impl HeaderMap {
    fn from_row(row: &[Data]) -> Result<Self, AppError> {
        let mut student_no_idx = None;
        let mut name_idx = None;
        let mut gender_idx = None;

        for (idx, cell) in row.iter().enumerate() {
            let text = cell.to_string().trim().to_string();
            match text.as_str() {
                "学号" | "student_no" => student_no_idx = Some(idx),
                "姓名" | "name" => name_idx = Some(idx),
                "性别" | "gender" => gender_idx = Some(idx),
                _ => {}
            }
        }

        let student_no_idx = student_no_idx
            .ok_or_else(|| AppError::InvalidInput("Excel 表头缺少「学号」列".into()))?;
        let name_idx =
            name_idx.ok_or_else(|| AppError::InvalidInput("Excel 表头缺少「姓名」列".into()))?;

        Ok(Self {
            student_no_idx,
            name_idx,
            gender_idx,
        })
    }
}

fn cell_to_string(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.trim().to_string(),
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Float(f)) => {
            if (*f - f.round()).abs() < f64::EPSILON {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Some(Data::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}
