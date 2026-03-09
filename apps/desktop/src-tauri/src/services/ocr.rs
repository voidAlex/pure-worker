//! OCR 识别服务模块
//!
//! 提供图片预处理（灰度化、去噪、纠偏）和 OCR 文字识别能力。
//! 底层使用 ONNX Runtime + PaddleOCR 模型。

use std::path::{Path, PathBuf};

use chrono::Utc;
use image::{DynamicImage, ImageReader};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::assignment_grading::{AssignmentAsset, AssignmentOcrResult};
use crate::services::audit::AuditService;

/// OCR 识别服务，处理图片预处理和文字提取。
pub struct OcrService;

struct PreprocessOutcome {
    output_path: PathBuf,
    output_path_for_db: String,
    image_width: i32,
    image_height: i32,
}

const PREPROCESSED_SUBDIR: &str = "preprocessed";
const MAX_IMAGE_LONGEST_SIDE: u32 = 4096;
const PREPROCESS_STATUS_DONE: &str = "done";
const PREPROCESS_STATUS_FAILED: &str = "failed";

impl OcrService {
    pub async fn preprocess_image(
        pool: &SqlitePool,
        asset_id: &str,
        workspace_path: &Path,
    ) -> Result<PathBuf, AppError> {
        let result = async {
            let asset = Self::get_asset_by_id(pool, asset_id).await?;
            let source_path = Self::build_source_image_path(&asset);

            let preprocessed_dir = workspace_path.join(PREPROCESSED_SUBDIR);
            Self::ensure_dir_exists(&preprocessed_dir)?;

            let output_path = Self::build_preprocessed_output_path(&preprocessed_dir, asset_id);
            let process_result = Self::run_preprocess_blocking(source_path, output_path)
            .await
            .map_err(|error| AppError::TaskExecution(format!("图片预处理任务执行失败：{error}")))?;

            let outcome = process_result?;
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                "UPDATE assignment_asset SET preprocessed_path = ?, preprocess_status = ?, image_width = ?, image_height = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
            )
            .bind(&outcome.output_path_for_db)
            .bind(PREPROCESS_STATUS_DONE)
            .bind(outcome.image_width)
            .bind(outcome.image_height)
            .bind(&now)
            .bind(asset_id)
            .execute(pool)
            .await?;

            Ok(outcome.output_path)
        }
        .await;

        if result.is_err() {
            Self::mark_preprocess_failed(pool, asset_id).await?;
        }

        result
    }

    pub async fn extract_text(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        let asset = Self::get_asset_by_id(pool, asset_id).await?;
        let preprocessed_path = asset.preprocessed_path.ok_or_else(|| {
            AppError::InvalidInput(format!("预处理图片路径缺失，无法执行 OCR：{asset_id}"))
        })?;

        let attempt_id = Uuid::new_v4().to_string();
        let detail = serde_json::json!({
            "attempt_id": attempt_id,
            "asset_id": asset_id,
            "job_id": job_id,
            "preprocessed_path": preprocessed_path,
            "attempt_at": Utc::now().to_rfc3339(),
            "stage": "ocr_model_loading",
            "message": "开始尝试加载 OCR 模型",
        });

        let _ = AuditService::log_with_detail(
            pool,
            "system",
            "ocr_model_loading_attempt",
            "assignment_asset",
            Some(asset_id),
            "low",
            false,
            Some(&detail.to_string()),
        )
        .await;

        // TODO: M4 Phase 2 — 完整 OCR 流程
        // 1. 加载 PaddleOCR 检测模型 (ch_PP-OCRv4_det_infer.onnx)
        // 2. 运行文本检测，获取文本区域边界框
        // 3. 对每个文本区域进行方向分类 (ch_ppocr_mobile_v2.0_cls_infer.onnx)
        // 4. 运行文本识别 (ch_PP-OCRv4_rec_infer.onnx)
        // 5. 按题号区域分组识别结果
        // 6. 将结果写入 assignment_ocr_result 表

        Err(AppError::TaskExecution(
            "OCR 模型尚未配置，请在 workspace/models/ 目录放置 PaddleOCR ONNX 模型文件".into(),
        ))
    }

    pub async fn run_ocr_pipeline(
        pool: &SqlitePool,
        asset_id: &str,
        job_id: &str,
        workspace_path: &Path,
    ) -> Result<Vec<AssignmentOcrResult>, AppError> {
        Self::preprocess_image(pool, asset_id, workspace_path)
            .await
            .map_err(|error| {
                AppError::TaskExecution(format!("OCR 预处理失败（asset_id={asset_id}）：{error}"))
            })?;

        Self::extract_text(pool, asset_id, job_id)
            .await
            .map_err(|error| {
                AppError::TaskExecution(format!(
                    "OCR 文本提取失败（asset_id={asset_id}, job_id={job_id}）：{error}"
                ))
            })
    }

    fn ensure_dir_exists(dir: &Path) -> Result<(), AppError> {
        std::fs::create_dir_all(dir)
            .map_err(|error| AppError::FileOperation(format!("预处理目录创建失败：{error}")))
    }

    fn build_source_image_path(asset: &AssignmentAsset) -> PathBuf {
        PathBuf::from(asset.file_path.clone())
    }

    fn build_preprocessed_output_path(preprocessed_dir: &Path, asset_id: &str) -> PathBuf {
        preprocessed_dir.join(format!("{asset_id}.png"))
    }

    async fn run_preprocess_blocking(
        source_path: PathBuf,
        output_path: PathBuf,
    ) -> Result<Result<PreprocessOutcome, AppError>, tokio::task::JoinError> {
        tokio::task::spawn_blocking(move || {
            if !source_path.exists() {
                return Err(AppError::FileOperation(format!(
                    "图片文件不存在：{}",
                    source_path.display()
                )));
            }

            let image = Self::decode_image(&source_path)?;
            let normalized = Self::apply_preprocess_pipeline(image);
            let (image_width, image_height) = Self::dimensions_to_i32(&normalized)?;

            normalized
                .save(&output_path)
                .map_err(|error| AppError::FileOperation(format!("预处理图片保存失败：{error}")))?;

            Ok(PreprocessOutcome {
                output_path_for_db: output_path.to_string_lossy().to_string(),
                output_path,
                image_width,
                image_height,
            })
        })
        .await
    }

    fn decode_image(path: &Path) -> Result<DynamicImage, AppError> {
        ImageReader::open(path)
            .map_err(|error| AppError::FileOperation(format!("图片文件读取失败：{error}")))?
            .decode()
            .map_err(|error| AppError::FileOperation(format!("图片解码失败：{error}")))
    }

    fn apply_preprocess_pipeline(image: DynamicImage) -> DynamicImage {
        let grayscale = image.grayscale();

        // TODO: M4 Phase 2 - 增加去噪（如中值滤波）以提升拍照噪点场景识别稳定性。
        // TODO: M4 Phase 2 - 增加自动纠偏（deskew）以修正倾斜拍照导致的 OCR 降准。
        Self::resize_if_needed(grayscale, MAX_IMAGE_LONGEST_SIDE)
    }

    fn dimensions_to_i32(image: &DynamicImage) -> Result<(i32, i32), AppError> {
        let width = image.width();
        let height = image.height();
        let image_width = i32::try_from(width)
            .map_err(|_| AppError::InvalidInput(format!("图片宽度超出范围：{width}")))?;
        let image_height = i32::try_from(height)
            .map_err(|_| AppError::InvalidInput(format!("图片高度超出范围：{height}")))?;
        Ok((image_width, image_height))
    }

    async fn mark_preprocess_failed(pool: &SqlitePool, asset_id: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE assignment_asset SET preprocess_status = ?, updated_at = ? WHERE id = ? AND is_deleted = 0",
        )
        .bind(PREPROCESS_STATUS_FAILED)
        .bind(&now)
        .bind(asset_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn get_asset_by_id(
        pool: &SqlitePool,
        asset_id: &str,
    ) -> Result<AssignmentAsset, AppError> {
        let asset = sqlx::query_as::<_, AssignmentAsset>(
            "SELECT * FROM assignment_asset WHERE id = ? AND is_deleted = 0",
        )
        .bind(asset_id)
        .fetch_optional(pool)
        .await?;

        asset.ok_or_else(|| AppError::NotFound(format!("作业资产不存在或已删除：{asset_id}")))
    }

    fn resize_if_needed(image: DynamicImage, max_longest_side: u32) -> DynamicImage {
        let width = image.width();
        let height = image.height();
        let longest = width.max(height);

        if longest <= max_longest_side {
            return image;
        }

        let ratio = (max_longest_side as f64) / (longest as f64);
        let target_width = ((width as f64) * ratio).round().max(1.0) as u32;
        let target_height = ((height as f64) * ratio).round().max(1.0) as u32;

        image.resize(
            target_width,
            target_height,
            image::imageops::FilterType::Lanczos3,
        )
    }
}
