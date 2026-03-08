//! 学生长期记忆服务模块。
//!
//! 提供目录初始化、月度模板创建、Markdown 解析、记忆读取、追加写入与敏感信息检测能力。

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use regex::Regex;

use crate::error::AppError;
use crate::models::student_memory::{
    AppendMemoryNoteInput, InitStudentMemoryInput, MemoryEntry, MemoryFileMeta,
    ReadCommentMaterialsInput, ReadMemoryByTopicInput, ReadMemoryTimelineInput,
    SensitiveInfoResult,
};

const FIXED_SECTIONS: [&str; 5] = [
    "学习表现观察",
    "错题与薄弱点",
    "家校沟通纪要",
    "干预策略与效果",
    "评语素材池",
];

/// 确保学生长期记忆目录存在：workspace/students/{student_id}/memory/。
pub fn ensure_student_memory_dir(
    workspace_path: &Path,
    student_id: &str,
) -> Result<PathBuf, AppError> {
    if student_id.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("student_id 不能为空")));
    }

    let memory_dir = workspace_path
        .join("students")
        .join(student_id.trim())
        .join("memory");

    fs::create_dir_all(&memory_dir)
        .map_err(|error| AppError::FileOperation(format!("创建学生记忆目录失败：{}", error)))?;

    Ok(memory_dir)
}

/// 确保指定月份的记忆模板文件存在，不存在则按固定模板创建。
pub fn ensure_monthly_file(
    workspace_path: &Path,
    student_id: &str,
    year_month: &str,
    meta: &MemoryFileMeta,
) -> Result<PathBuf, AppError> {
    if !Regex::new(r"^\d{4}-\d{2}$")
        .map_err(|error| AppError::Internal(format!("年月校验正则初始化失败：{}", error)))?
        .is_match(year_month)
    {
        return Err(AppError::InvalidInput(String::from(
            "year_month 格式无效，应为 YYYY-MM",
        )));
    }

    let memory_dir = ensure_student_memory_dir(workspace_path, student_id)?;
    let file_path = memory_dir.join(format!("{}.md", year_month));

    if file_path.exists() {
        return Ok(file_path);
    }

    let now = Local::now().to_rfc3339();
    let content = format!(
        "---\nstudent_id: {}\nstudent_name: {}\nclass_id: {}\nhomeroom_teacher_id: {}\nversion: 1.0\nlast_updated_at: {}\n---\n\n## 学习表现观察\n\n## 错题与薄弱点\n\n## 家校沟通纪要\n\n## 干预策略与效果\n\n## 评语素材池\n",
        student_id,
        meta.student_name.clone().unwrap_or_default(),
        meta.class_id.clone().unwrap_or_default(),
        meta.homeroom_teacher_id.clone().unwrap_or_default(),
        now,
    );

    fs::write(&file_path, content)
        .map_err(|error| AppError::FileOperation(format!("创建月度记忆文件失败：{}", error)))?;

    Ok(file_path)
}

/// 解析单个记忆 Markdown 文件，提取 frontmatter 元数据与章节条目。
pub fn parse_memory_file(file_path: &Path) -> Result<(MemoryFileMeta, Vec<MemoryEntry>), AppError> {
    let content = fs::read_to_string(file_path)
        .map_err(|error| AppError::FileOperation(format!("读取记忆文件失败：{}", error)))?;

    let file_path_text = file_path.to_string_lossy().to_string();
    let meta = parse_frontmatter(&content, &file_path_text)?;

    let date_regex = Regex::new(r"\[(\d{4}-\d{2}-\d{2})\]")
        .map_err(|error| AppError::Internal(format!("日期正则初始化失败：{}", error)))?;
    let subject_regex = Regex::new(r"\[([a-zA-Z_\u4e00-\u9fa5]+)\]")
        .map_err(|error| AppError::Internal(format!("标签正则初始化失败：{}", error)))?;

    let mut entries = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_done = false;
    let mut current_section = String::from("未分类");

    for line in content.lines() {
        let trimmed = line.trim();

        if !frontmatter_done && trimmed == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            }
            in_frontmatter = false;
            frontmatter_done = true;
            continue;
        }

        if in_frontmatter {
            continue;
        }

        if let Some(section_name) = trimmed.strip_prefix("## ") {
            current_section = section_name.trim().to_string();
            continue;
        }

        if let Some(entry_text) = trimmed.strip_prefix("- ") {
            let raw_entry = entry_text.trim().to_string();

            let date = date_regex
                .captures(&raw_entry)
                .and_then(|capture| capture.get(1).map(|m| m.as_str().to_string()));

            let mut subject = None;
            let mut entry_type = None;

            if let Some(date_match) = date_regex.find(&raw_entry) {
                let trailing_text = &raw_entry[date_match.end()..];
                let mut tags = subject_regex
                    .captures_iter(trailing_text)
                    .filter_map(|capture| capture.get(1).map(|m| m.as_str().to_string()));

                subject = tags.next();
                entry_type = tags.next();
            }

            let entry_content = strip_leading_tags(&raw_entry);
            if entry_content.is_empty() {
                continue;
            }

            entries.push(MemoryEntry {
                date,
                subject,
                entry_type,
                content: entry_content,
                section: current_section.clone(),
                source_file: file_path_text.clone(),
            });
        }
    }

    Ok((meta, entries))
}

/// 解析 Markdown frontmatter（简单 key: value 行级解析，不依赖 YAML 库）。
pub fn parse_frontmatter(content: &str, file_path: &str) -> Result<MemoryFileMeta, AppError> {
    let mut lines = content.lines();
    let Some(first_line) = lines.next() else {
        return Err(AppError::InvalidInput(format!(
            "记忆文件为空：{}",
            file_path
        )));
    };

    if first_line.trim() != "---" {
        return Err(AppError::InvalidInput(format!(
            "记忆文件缺少 frontmatter 起始标记：{}",
            file_path
        )));
    }

    let mut student_id: Option<String> = None;
    let mut student_name: Option<String> = None;
    let mut class_id: Option<String> = None;
    let mut homeroom_teacher_id: Option<String> = None;
    let mut version: Option<String> = None;
    let mut last_updated_at: Option<String> = None;

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, ':');
        let key = parts.next().unwrap_or_default().trim();
        let value = parts.next().unwrap_or_default().trim().to_string();
        let value = if value.is_empty() { None } else { Some(value) };

        match key {
            "student_id" => student_id = value,
            "student_name" => student_name = value,
            "class_id" => class_id = value,
            "homeroom_teacher_id" => homeroom_teacher_id = value,
            "version" => version = value,
            "last_updated_at" => last_updated_at = value,
            _ => {}
        }
    }

    let Some(student_id) = student_id else {
        return Err(AppError::InvalidInput(format!(
            "frontmatter 缺少 student_id：{}",
            file_path
        )));
    };

    Ok(MemoryFileMeta {
        student_id,
        student_name,
        class_id,
        homeroom_teacher_id,
        version,
        last_updated_at,
        file_path: file_path.to_string(),
    })
}

/// 读取学生记忆时间线，支持按日期区间和章节过滤，结果按日期倒序。
pub fn read_memory_timeline(
    workspace_path: &Path,
    input: &ReadMemoryTimelineInput,
) -> Result<Vec<MemoryEntry>, AppError> {
    if input.student_id.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("student_id 不能为空")));
    }

    if let Some(limit) = input.limit {
        if limit <= 0 {
            return Err(AppError::InvalidInput(String::from("limit 必须大于 0")));
        }
    }

    let markdown_files = list_memory_markdown_files(workspace_path, &input.student_id)?;
    let mut all_entries = Vec::new();
    for file in markdown_files {
        let (_, entries) = parse_memory_file(&file)?;
        all_entries.extend(entries);
    }

    let filtered = all_entries
        .into_iter()
        .filter(|entry| {
            if let Some(section_filter) = &input.section_filter {
                if !section_filter.is_empty()
                    && !section_filter.iter().any(|name| name == &entry.section)
                {
                    return false;
                }
            }

            if input.from_date.is_some() || input.to_date.is_some() {
                let Some(date) = &entry.date else {
                    return false;
                };

                if let Some(from_date) = &input.from_date {
                    if date < from_date {
                        return false;
                    }
                }

                if let Some(to_date) = &input.to_date {
                    if date > to_date {
                        return false;
                    }
                }
            }

            true
        })
        .collect::<Vec<_>>();

    let mut sorted = filtered;
    sorted.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| right.source_file.cmp(&left.source_file))
    });

    if let Some(limit) = input.limit {
        sorted.truncate(limit as usize);
    }

    Ok(sorted)
}

/// 按主题关键词检索记忆内容，支持可选学科过滤。
pub fn read_memory_by_topic(
    workspace_path: &Path,
    input: &ReadMemoryByTopicInput,
) -> Result<Vec<MemoryEntry>, AppError> {
    if input.student_id.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("student_id 不能为空")));
    }

    if input.topic.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("topic 不能为空")));
    }

    if let Some(top_k) = input.top_k {
        if top_k <= 0 {
            return Err(AppError::InvalidInput(String::from("top_k 必须大于 0")));
        }
    }

    let markdown_files = list_memory_markdown_files(workspace_path, &input.student_id)?;
    let mut matched = Vec::new();
    for file in markdown_files {
        let (_, entries) = parse_memory_file(&file)?;
        for entry in entries {
            if !entry.content.contains(input.topic.trim()) {
                continue;
            }

            if let Some(subject) = &input.subject {
                if entry.subject.as_deref() != Some(subject.as_str()) {
                    continue;
                }
            }

            matched.push(entry);
        }
    }

    matched.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| right.source_file.cmp(&left.source_file))
    });

    if let Some(top_k) = input.top_k {
        matched.truncate(top_k as usize);
    }

    Ok(matched)
}

/// 读取评语素材池章节条目，支持可选学期与学科过滤。
pub fn read_comment_materials(
    workspace_path: &Path,
    input: &ReadCommentMaterialsInput,
) -> Result<Vec<MemoryEntry>, AppError> {
    if input.student_id.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("student_id 不能为空")));
    }

    let markdown_files = list_memory_markdown_files(workspace_path, &input.student_id)?;
    let mut results = Vec::new();

    for file in markdown_files {
        let file_name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();

        if let Some(term) = &input.term {
            let term = term.trim();
            if !term.is_empty() && !file_name.contains(term) {
                continue;
            }
        }

        let (_, entries) = parse_memory_file(&file)?;
        for entry in entries {
            if entry.section != "评语素材池" {
                continue;
            }

            if let Some(subject) = &input.subject {
                if entry.subject.as_deref() != Some(subject.as_str()) {
                    continue;
                }
            }

            results.push(entry);
        }
    }

    results.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| right.source_file.cmp(&left.source_file))
    });

    Ok(results)
}

/// 追加记忆笔记到当月文件目标章节末尾，并更新 frontmatter 的 last_updated_at。
pub fn append_memory_note(
    workspace_path: &Path,
    input: &AppendMemoryNoteInput,
) -> Result<(), AppError> {
    if input.student_id.trim().is_empty() {
        return Err(AppError::InvalidInput(String::from("student_id 不能为空")));
    }

    let section = input.section.trim();
    if !FIXED_SECTIONS.iter().any(|name| name == &section) {
        return Err(AppError::InvalidInput(format!(
            "section 非法：{}",
            input.section
        )));
    }

    let content = input.content.trim();
    if content.is_empty() {
        return Err(AppError::InvalidInput(String::from("content 不能为空")));
    }

    let sensitive = check_sensitive_info(content);
    if sensitive.has_sensitive {
        return Err(AppError::InvalidInput(format!(
            "内容包含敏感信息：{}",
            sensitive.violations.join("、")
        )));
    }

    let year_month = Local::now().format("%Y-%m").to_string();
    let file_path = ensure_monthly_file(
        workspace_path,
        &input.student_id,
        &year_month,
        &MemoryFileMeta {
            student_id: input.student_id.clone(),
            student_name: None,
            class_id: None,
            homeroom_teacher_id: None,
            version: Some(String::from("1.0")),
            last_updated_at: None,
            file_path: String::new(),
        },
    )?;

    let existing = fs::read_to_string(&file_path)
        .map_err(|error| AppError::FileOperation(format!("读取记忆文件失败：{}", error)))?;

    let now = Local::now().to_rfc3339();
    let date = Local::now().format("%Y-%m-%d").to_string();
    let note_line = format!("- [{}] {}", date, content);

    let mut lines = existing
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let target_header = format!("## {}", section);
    let Some(section_index) = lines.iter().position(|line| line.trim() == target_header) else {
        return Err(AppError::InvalidInput(format!(
            "未找到目标章节：{}",
            section
        )));
    };

    let next_section_index = lines
        .iter()
        .enumerate()
        .skip(section_index + 1)
        .find(|(_, line)| line.trim().starts_with("## "))
        .map(|(index, _)| index)
        .unwrap_or(lines.len());

    let mut insert_index = next_section_index;
    while insert_index > section_index + 1 && lines[insert_index - 1].trim().is_empty() {
        insert_index -= 1;
    }
    lines.insert(insert_index, note_line);
    lines.insert(insert_index + 1, String::new());

    let updated = update_last_updated_at(&lines.join("\n"), &now)?;
    fs::write(&file_path, format!("{}\n", updated))
        .map_err(|error| AppError::FileOperation(format!("写入记忆文件失败：{}", error)))?;

    Ok(())
}

/// 检测文本中的敏感信息（身份证号、手机号、地址关键词、银行卡号）。
pub fn check_sensitive_info(content: &str) -> SensitiveInfoResult {
    let mut violations = Vec::new();

    if Regex::new(r"\b\d{15}(\d{2}[0-9Xx])?\b")
        .map(|regex| regex.is_match(content))
        .unwrap_or(false)
    {
        violations.push(String::from("身份证号"));
    }

    if Regex::new(r"\b1[3-9]\d{9}\b")
        .map(|regex| regex.is_match(content))
        .unwrap_or(false)
    {
        violations.push(String::from("手机号"));
    }

    let address_keywords = ["家庭住址", "家住", "住址", "门牌号"];
    if address_keywords
        .iter()
        .any(|keyword| content.contains(keyword))
    {
        violations.push(String::from("住址信息"));
    }

    if Regex::new(r"\b\d{16,19}\b")
        .map(|regex| regex.is_match(content))
        .unwrap_or(false)
    {
        violations.push(String::from("银行卡号"));
    }

    SensitiveInfoResult {
        has_sensitive: !violations.is_empty(),
        violations,
    }
}

/// 初始化学生长期记忆目录与当月模板文件。
pub fn init_student_memory(
    workspace_path: &Path,
    input: &InitStudentMemoryInput,
) -> Result<PathBuf, AppError> {
    let _ = ensure_student_memory_dir(workspace_path, &input.student_id)?;
    let year_month = Local::now().format("%Y-%m").to_string();

    ensure_monthly_file(
        workspace_path,
        &input.student_id,
        &year_month,
        &MemoryFileMeta {
            student_id: input.student_id.clone(),
            student_name: input.student_name.clone(),
            class_id: input.class_id.clone(),
            homeroom_teacher_id: input.homeroom_teacher_id.clone(),
            version: Some(String::from("1.0")),
            last_updated_at: Some(Local::now().to_rfc3339()),
            file_path: String::new(),
        },
    )
}

fn list_memory_markdown_files(
    workspace_path: &Path,
    student_id: &str,
) -> Result<Vec<PathBuf>, AppError> {
    let memory_dir = workspace_path
        .join("students")
        .join(student_id.trim())
        .join("memory");

    if !memory_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let read_dir = fs::read_dir(&memory_dir)
        .map_err(|error| AppError::FileOperation(format!("读取记忆目录失败：{}", error)))?;

    for entry_result in read_dir {
        let entry = entry_result
            .map_err(|error| AppError::FileOperation(format!("读取记忆目录项失败：{}", error)))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        if is_markdown {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn strip_leading_tags(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    loop {
        if !text.starts_with('[') {
            break;
        }
        let Some(end_index) = text.find(']') else {
            break;
        };
        text = text[end_index + 1..].trim_start().to_string();
    }
    text.trim().to_string()
}

fn update_last_updated_at(content: &str, now: &str) -> Result<String, AppError> {
    let mut lines = content
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    if lines.is_empty() || lines.first().map(|line| line.trim()) != Some("---") {
        return Err(AppError::InvalidInput(String::from(
            "记忆文件缺少 frontmatter，无法更新 last_updated_at",
        )));
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim() == "---")
        .map(|(index, _)| index)
    else {
        return Err(AppError::InvalidInput(String::from(
            "frontmatter 结束标记缺失，无法更新 last_updated_at",
        )));
    };

    if let Some(index) = lines
        .iter()
        .take(end_index)
        .position(|line| line.trim_start().starts_with("last_updated_at:"))
    {
        lines[index] = format!("last_updated_at: {}", now);
    } else {
        lines.insert(end_index, format!("last_updated_at: {}", now));
    }

    Ok(lines.join("\n"))
}
