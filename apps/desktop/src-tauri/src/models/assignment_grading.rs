//! M4 作业批改与题库数据模型
//!
//! 定义批改任务、作业资产、OCR 结果、错题记录、练习卷、题库等数据结构。

use serde::{Deserialize, Serialize};
use specta::Type;


/// 批改任务记录，关联一次批改的配置和状态。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct GradingJob {
    pub id: String,
    pub class_id: String,
    pub title: String,
    /// 批改模式：basic（纯 OCR）或 enhanced（多模态 LLM）
    pub grading_mode: String,
    /// 任务状态：pending / running / completed / failed
    pub status: String,
    /// 标准答案 JSON
    pub answer_key_json: Option<String>,
    /// 评分规则 JSON
    pub scoring_rules_json: Option<String>,
    pub total_assets: i32,
    pub processed_assets: i32,
    pub failed_assets: i32,
    pub conflict_count: i32,
    /// 关联的异步任务 ID
    pub task_id: Option<String>,
    /// 导出文件路径
    pub output_path: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建批改任务输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateGradingJobInput {
    pub class_id: String,
    pub title: String,
    /// basic 或 enhanced
    pub grading_mode: Option<String>,
    /// 标准答案 JSON
    pub answer_key_json: Option<String>,
    /// 评分规则 JSON
    pub scoring_rules_json: Option<String>,
    /// 关联的异步任务 ID
    pub task_id: Option<String>,
    /// 导出文件路径
    pub output_path: Option<String>,
}

/// 更新批改任务输入。
#[derive(Debug, Deserialize, Type)]
pub struct UpdateGradingJobInput {
    pub id: String,
    pub title: Option<String>,
    pub grading_mode: Option<String>,
    pub status: Option<String>,
    pub answer_key_json: Option<String>,
    pub scoring_rules_json: Option<String>,
    pub task_id: Option<String>,
    pub output_path: Option<String>,
}


/// 作业资产记录（含 M4 扩展字段）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AssignmentAsset {
    pub id: String,
    pub class_id: String,
    pub file_path: String,
    pub hash: Option<String>,
    pub captured_at: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    /// 关联的批改任务 ID
    pub job_id: Option<String>,
    /// 原始文件名
    pub original_filename: Option<String>,
    /// 文件大小（字节）
    pub file_size: Option<i64>,
    /// MIME 类型
    pub mime_type: Option<String>,
    /// 图片宽度（像素）
    pub image_width: Option<i32>,
    /// 图片高度（像素）
    pub image_height: Option<i32>,
    /// 预处理状态：pending / done / failed
    pub preprocess_status: String,
    /// 预处理后的文件路径
    pub preprocessed_path: Option<String>,
    pub updated_at: String,
}

/// 批量添加作业资产输入。
#[derive(Debug, Deserialize, Type)]
pub struct AddAssignmentAssetsInput {
    pub job_id: String,
    pub class_id: String,
    /// 本地文件路径列表（前端拖拽获取）
    pub file_paths: Vec<String>,
}


/// OCR 识别结果记录（含 M4 扩展字段）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct AssignmentOcrResult {
    pub id: String,
    pub asset_id: String,
    pub job_id: Option<String>,
    pub student_id: Option<String>,
    pub question_no: Option<String>,
    pub answer_text: Option<String>,
    pub confidence: Option<f64>,
    pub score: Option<f64>,
    pub created_at: String,
    /// OCR 原始识别文本
    pub ocr_raw_text: Option<String>,
    /// 多模态 LLM 评分
    pub multimodal_score: Option<f64>,
    /// 多模态 LLM 反馈
    pub multimodal_feedback: Option<String>,
    /// 冲突标记：0=无冲突 1=有冲突
    pub conflict_flag: i32,
    /// 复核状态：pending / approved / rejected
    pub review_status: String,
    /// 复核人
    pub reviewed_by: Option<String>,
    /// 复核时间
    pub reviewed_at: Option<String>,
    /// 最终得分（教师确认后）
    pub final_score: Option<f64>,
    pub is_deleted: i32,
    pub updated_at: String,
}

/// 复核 OCR 结果输入。
#[derive(Debug, Deserialize, Type)]
pub struct ReviewOcrResultInput {
    pub id: String,
    /// approved 或 rejected
    pub review_status: String,
    /// 教师确认的最终得分
    pub final_score: Option<f64>,
    /// 复核人标识
    pub reviewed_by: String,
}

/// 批量复核 OCR 结果输入。
#[derive(Debug, Deserialize, Type)]
pub struct BatchReviewOcrResultsInput {
    pub ids: Vec<String>,
    pub review_status: String,
    pub reviewed_by: String,
    /// 教师确认的最终得分（可选，批量设定同一分数）
    pub final_score: Option<f64>,
}


/// 错题记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct WrongAnswerRecord {
    pub id: String,
    pub student_id: String,
    pub job_id: String,
    pub ocr_result_id: String,
    pub question_no: String,
    pub knowledge_point: Option<String>,
    pub difficulty: Option<String>,
    pub student_answer: Option<String>,
    pub correct_answer: Option<String>,
    pub score: Option<f64>,
    pub full_score: Option<f64>,
    pub error_type: Option<String>,
    pub is_resolved: i32,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 查询学生错题输入。
#[derive(Debug, Deserialize, Type)]
pub struct ListWrongAnswersInput {
    pub student_id: Option<String>,
    /// 按批改任务 ID 筛选
    pub job_id: Option<String>,
    /// 按知识点筛选
    pub knowledge_point: Option<String>,
    /// 仅查询未解决的错题
    pub unresolved_only: Option<bool>,
    /// 限制返回数量
    pub limit: Option<i32>,
}

/// 标记错题为已解决输入。
#[derive(Debug, Deserialize, Type)]
pub struct ResolveWrongAnswerInput {
    pub id: String,
}


/// 练习卷记录。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct PracticeSheet {
    pub id: String,
    pub student_id: String,
    pub title: String,
    /// 知识点列表 JSON
    pub knowledge_points_json: Option<String>,
    pub difficulty: Option<String>,
    pub question_count: i32,
    /// 题目内容 JSON
    pub questions_json: Option<String>,
    /// 答案内容 JSON
    pub answers_json: Option<String>,
    /// 练习卷文件路径
    pub file_path: Option<String>,
    /// 答案文件路径
    pub answer_file_path: Option<String>,
    /// 状态：draft / generating / completed / failed
    pub status: String,
    /// 关联异步任务 ID
    pub task_id: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 生成练习卷输入。
#[derive(Debug, Clone, Deserialize, Type)]
pub struct GeneratePracticeSheetInput {
    pub student_id: String,
    pub title: String,
    /// 目标知识点列表
    pub knowledge_points: Option<Vec<String>>,
    /// 难度级别
    pub difficulty: Option<String>,
    /// 题目数量
    pub question_count: Option<i32>,
}


/// 题库记录（含 M4 扩展字段）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Type)]
pub struct QuestionBankItem {
    pub id: String,
    pub source: Option<String>,
    pub knowledge_point: Option<String>,
    pub difficulty: Option<String>,
    pub stem: String,
    pub answer: Option<String>,
    pub explanation: Option<String>,
    pub is_deleted: i32,
    pub created_at: String,
    pub updated_at: String,
    /// 题型：choice / fill / short_answer / calculation 等
    pub question_type: Option<String>,
    /// 学科
    pub subject: Option<String>,
    /// 年级
    pub grade: Option<String>,
    /// 标签 JSON
    pub tags_json: Option<String>,
    /// 模板参数 JSON（用于参数扰动生成变体题）
    pub template_params_json: Option<String>,
    /// 父题 ID（变体题的来源题目）
    pub parent_id: Option<String>,
}

/// 创建题库条目输入。
#[derive(Debug, Deserialize, Type)]
pub struct CreateQuestionBankInput {
    pub source: Option<String>,
    pub knowledge_point: Option<String>,
    pub difficulty: Option<String>,
    pub stem: String,
    pub answer: Option<String>,
    pub explanation: Option<String>,
    pub question_type: Option<String>,
    pub subject: Option<String>,
    pub grade: Option<String>,
    pub tags_json: Option<String>,
    pub template_params_json: Option<String>,
    pub parent_id: Option<String>,
}

/// 查询题库输入。
#[derive(Debug, Deserialize, Type)]
pub struct ListQuestionBankInput {
    pub source: Option<String>,
    pub knowledge_point: Option<String>,
    pub question_type: Option<String>,
    pub subject: Option<String>,
    pub difficulty: Option<String>,
    pub grade: Option<String>,
    pub limit: Option<i32>,
}


/// 启动批量批改任务输入。
#[derive(Debug, Clone, Deserialize, Type)]
pub struct StartGradingInput {
    pub job_id: String,
}


/// 导出批改结果输入。
#[derive(Debug, Deserialize, Type)]
pub struct ExportGradingResultsInput {
    pub job_id: String,
    /// 导出文件路径（前端通过文件对话框获取）
    pub output_path: String,
}

/// 导出批改结果响应。
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ExportGradingResultsResponse {
    pub output_path: String,
    pub total_rows: i32,
}

/// 批改任务进度信息。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GradingProgress {
    pub total: i32,
    pub processed: i32,
    pub failed: i32,
    pub conflicts: i32,
    pub current_filename: Option<String>,
}
