//! 导出渲染内置技能模块
//!
//! 将结构化数据渲染为 Word（docx）或 Excel（xlsx）文档并导出。
//! 使用 docx-rs 生成 Word 文档，rust_xlsxwriter 生成 Excel 文档。

use std::future::Future;
use std::io::Cursor;
use std::path::Path;
use std::pin::Pin;
use std::time::Instant;

use crate::error::AppError;
use crate::services::unified_tool::{
    create_error_result, create_success_result, ToolResult, ToolRiskLevel, UnifiedTool,
};

/// 导出渲染内置技能。
pub struct ExportRenderSkill;

impl UnifiedTool for ExportRenderSkill {
    fn name(&self) -> &str {
        "export.render"
    }

    fn description(&self) -> &str {
        "文档导出渲染：将结构化数据渲染为 Word（docx）或 Excel（xlsx）文件"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "format": {
                    "type": "string",
                    "enum": ["docx", "xlsx"],
                    "description": "导出格式"
                },
                "output_path": { "type": "string", "description": "输出文件路径" },
                "paragraphs": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "段落文本数组（docx 格式时必填）"
                },
                "rows": {
                    "type": "array",
                    "items": { "type": "array", "items": {} },
                    "description": "行数据二维数组（xlsx 格式时必填）"
                }
            },
            "required": ["format", "output_path"]
        })
    }

    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "format": { "type": "string" },
                "output_path": { "type": "string" },
                "paragraph_count": { "type": "integer" },
                "row_count": { "type": "integer" },
                "cell_count": { "type": "integer" }
            }
        })
    }

    fn risk_level(&self) -> ToolRiskLevel {
        ToolRiskLevel::High
    }

    fn invoke(
        &self,
        input: serde_json::Value,
        invoke_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, AppError>> + Send + '_>> {
        let invoke_id = invoke_id.to_string();
        Box::pin(async move {
            let start = Instant::now();
            execute_inner(input, &invoke_id, &start).await
        })
    }
}

/// 导出渲染内部执行逻辑。
async fn execute_inner(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    let skill_name = "export.render";

    let format = match input.get("format").and_then(|v| v.as_str()) {
        Some(f) => f.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'format'（导出格式：docx/xlsx）".to_string(),
            ));
        }
    };

    let output_path = match input.get("output_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'output_path'（输出文件路径）".to_string(),
            ));
        }
    };

    if let Err(e) =
        crate::services::path_whitelist::PathWhitelistService::validate_write_path(&output_path)
    {
        let duration_ms = start.elapsed().as_millis() as u64;
        return Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            format!("输出路径校验失败：{e}"),
        ));
    }

    match format.as_str() {
        "docx" => execute_render_docx(input, invoke_id, start, &output_path).await,
        "xlsx" => execute_render_xlsx(input, invoke_id, start, &output_path).await,
        other => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                format!("不支持的导出格式：'{other}'（支持：docx/xlsx）"),
            ))
        }
    }
}

/// 渲染 Word 文档。
async fn execute_render_docx(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
    output_path: &str,
) -> Result<ToolResult, AppError> {
    let skill_name = "export.render";

    let paragraphs = match input.get("paragraphs").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'paragraphs'（段落文本数组）".to_string(),
            ));
        }
    };

    let out = output_path.to_string();

    let result = tokio::task::spawn_blocking(move || render_docx_blocking(&paragraphs, &out))
        .await
        .map_err(|e| AppError::TaskExecution(format!("Word 文档渲染任务执行失败：{e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(create_success_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            info,
        )),
        Err(err_msg) => Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            err_msg,
        )),
    }
}

/// 渲染 Excel 文档。
async fn execute_render_xlsx(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
    output_path: &str,
) -> Result<ToolResult, AppError> {
    let skill_name = "export.render";

    let rows = match input.get("rows").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            let duration_ms = start.elapsed().as_millis() as u64;
            return Ok(create_error_result(
                skill_name,
                invoke_id,
                ToolRiskLevel::High,
                duration_ms,
                "缺少必填参数 'rows'（行数据二维数组）".to_string(),
            ));
        }
    };

    let out = output_path.to_string();

    let result = tokio::task::spawn_blocking(move || render_xlsx_blocking(&rows, &out))
        .await
        .map_err(|e| AppError::TaskExecution(format!("Excel 文档渲染任务执行失败：{e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(create_success_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            info,
        )),
        Err(err_msg) => Ok(create_error_result(
            skill_name,
            invoke_id,
            ToolRiskLevel::High,
            duration_ms,
            err_msg,
        )),
    }
}

/// 在阻塞线程中生成 Word 文档。
fn render_docx_blocking(
    paragraphs: &[String],
    output_path: &str,
) -> Result<serde_json::Value, String> {
    use docx_rs::{Docx, Paragraph, Run};

    // 写入前二次校验输出路径（防止 TOCTOU：首次校验与实际写入之间路径被替换为 symlink）
    crate::services::path_whitelist::PathWhitelistService::validate_write_path(output_path)
        .map_err(|e| format!("写入前二次路径校验失败：{e}"))?;

    let output = Path::new(output_path);
    let parent = output
        .parent()
        .ok_or_else(|| "输出路径缺少父目录".to_string())?;

    // 逐级安全创建父目录（防止 create_dir_all 的 TOCTOU symlink 攻击）
    ensure_safe_parent_dirs(parent).map_err(|e| format!("创建输出目录失败：{e}"))?;

    let mut doc = Docx::new();
    for text in paragraphs {
        let paragraph = Paragraph::new().add_run(Run::new().add_text(text));
        doc = doc.add_paragraph(paragraph);
    }

    let mut buf = Cursor::new(Vec::new());
    doc.build()
        .pack(&mut buf)
        .map_err(|e| format!("生成 Word 文档失败：{e}"))?;

    // 原子写入：先写临时文件再 rename，防止 TOCTOU symlink 替换攻击
    // 使用 UUID 临时文件名 + create_new 拒绝预占位
    let tmp_path = loop {
        let tmp_name = format!(".tmp-docx-{}", uuid::Uuid::new_v4());
        let path = parent.join(&tmp_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => {
                std::io::Write::write_all(&mut std::io::BufWriter::new(file), buf.get_ref())
                    .map_err(|e| {
                        let _ = std::fs::remove_file(&path);
                        format!("写入 Word 文档到临时文件失败：{e}")
                    })?;
                break path;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("创建临时输出文件失败：{e}")),
        }
    };

    std::fs::rename(&tmp_path, output_path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("将临时文件重命名到最终输出路径失败：{e}")
    })?;

    Ok(serde_json::json!({
        "format": "docx",
        "output_path": output_path,
        "paragraph_count": paragraphs.len(),
    }))
}

/// 在阻塞线程中生成 Excel 文档。
fn render_xlsx_blocking(
    rows: &[serde_json::Value],
    output_path: &str,
) -> Result<serde_json::Value, String> {
    use rust_xlsxwriter::Workbook;

    // 写入前二次校验输出路径（防止 TOCTOU：首次校验与实际写入之间路径被替换为 symlink）
    crate::services::path_whitelist::PathWhitelistService::validate_write_path(output_path)
        .map_err(|e| format!("写入前二次路径校验失败：{e}"))?;

    let output = Path::new(output_path);
    let parent = output
        .parent()
        .ok_or_else(|| "输出路径缺少父目录".to_string())?;

    // 逐级安全创建父目录（防止 create_dir_all 的 TOCTOU symlink 攻击）
    ensure_safe_parent_dirs(parent).map_err(|e| format!("创建输出目录失败：{e}"))?;

    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name("Sheet1")
        .map_err(|e| format!("创建工作表失败：{e}"))?;

    let mut total_cells: usize = 0;

    for (row_idx, row) in rows.iter().enumerate() {
        if let Some(cells) = row.as_array() {
            for (col_idx, cell) in cells.iter().enumerate() {
                let r = row_idx as u32;
                let c = col_idx as u16;
                match cell {
                    serde_json::Value::String(s) => {
                        worksheet
                            .write_string(r, c, s)
                            .map_err(|e| format!("写入单元格失败：{e}"))?;
                    }
                    serde_json::Value::Number(n) => {
                        if let Some(f) = n.as_f64() {
                            worksheet
                                .write_number(r, c, f)
                                .map_err(|e| format!("写入单元格失败：{e}"))?;
                        } else {
                            worksheet
                                .write_string(r, c, n.to_string())
                                .map_err(|e| format!("写入单元格失败：{e}"))?;
                        }
                    }
                    serde_json::Value::Bool(b) => {
                        worksheet
                            .write_boolean(r, c, *b)
                            .map_err(|e| format!("写入单元格失败：{e}"))?;
                    }
                    serde_json::Value::Null => {}
                    other => {
                        worksheet
                            .write_string(r, c, other.to_string())
                            .map_err(|e| format!("写入单元格失败：{e}"))?;
                    }
                }
                total_cells += 1;
            }
        }
    }

    // 原子写入：先保存到临时文件再 rename，防止 TOCTOU symlink 替换攻击
    // 使用 UUID 临时文件名 + 写前检测已存在则重试
    let tmp_path = loop {
        let tmp_name = format!(".tmp-xlsx-{}", uuid::Uuid::new_v4());
        let path = parent.join(&tmp_name);
        // 若已存在（包括 symlink）则重试新 UUID
        if path.symlink_metadata().is_ok() {
            continue;
        }
        workbook.save(&path).map_err(|e| {
            let _ = std::fs::remove_file(&path);
            format!("保存 Excel 到临时文件失败：{e}")
        })?;
        break path;
    };

    std::fs::rename(&tmp_path, output_path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        format!("将临时文件重命名到最终输出路径失败：{e}")
    })?;

    Ok(serde_json::json!({
        "format": "xlsx",
        "output_path": output_path,
        "row_count": rows.len(),
        "cell_count": total_cells,
    }))
}

/// 逐级安全创建父目录（替代 create_dir_all，防止 TOCTOU symlink 攻击）。
///
/// 从白名单根目录向上追溯，对每一级：若不存在则 create_dir，创建后立即 symlink_metadata 检测。
fn ensure_safe_parent_dirs(target_parent: &Path) -> Result<(), AppError> {
    // 寻找已存在的祖先作为白名单根（必须是已存在的目录）
    let mut whitelist_root = target_parent.to_path_buf();
    while !whitelist_root.exists() {
        if let Some(parent) = whitelist_root.parent() {
            whitelist_root = parent.to_path_buf();
        } else {
            return Err(AppError::InvalidInput(String::from(
                "无法找到已存在的祖先目录作为白名单根",
            )));
        }
    }

    // 校验白名单根非 symlink 且是目录
    let root_meta = whitelist_root.symlink_metadata().map_err(|e| {
        AppError::FileOperation(format!(
            "无法读取白名单根目录元数据 '{}'：{e}",
            whitelist_root.display()
        ))
    })?;
    if root_meta.file_type().is_symlink() {
        return Err(AppError::PermissionDenied(format!(
            "白名单根目录是符号链接，已拒绝：'{}'",
            whitelist_root.display()
        )));
    }
    if !root_meta.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "白名单根目录不是目录：'{}'",
            whitelist_root.display()
        )));
    }

    // 从白名单根之后开始，逐级安全创建
    let relative = target_parent
        .strip_prefix(&whitelist_root)
        .map_err(|_| AppError::InvalidInput(String::from("目标父目录不在白名单根目录内")))?;

    let mut current = whitelist_root.clone();
    for component in relative.components() {
        if let std::path::Component::Normal(name) = component {
            current.push(name);
            if !current.exists() {
                std::fs::create_dir(&current).map_err(|e| {
                    AppError::FileOperation(format!("创建目录 '{}' 失败：{e}", current.display()))
                })?;
            }
            // 创建/存在后立即校验非 symlink
            let meta = current.symlink_metadata().map_err(|e| {
                AppError::FileOperation(format!("无法读取目录元数据 '{}'：{e}", current.display()))
            })?;
            if meta.file_type().is_symlink() {
                return Err(AppError::PermissionDenied(format!(
                    "目录是符号链接，已拒绝：'{}'",
                    current.display()
                )));
            }
            if !meta.is_dir() {
                return Err(AppError::InvalidInput(format!(
                    "路径不是目录：'{}'",
                    current.display()
                )));
            }
        }
    }

    Ok(())
}

/// 向后兼容的执行入口。
pub async fn execute(
    input: serde_json::Value,
    invoke_id: &str,
    start: &Instant,
) -> Result<ToolResult, AppError> {
    execute_inner(input, invoke_id, start).await
}
