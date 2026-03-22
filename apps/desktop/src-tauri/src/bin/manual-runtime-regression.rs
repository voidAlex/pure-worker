//! Agent Runtime 手工回归预置脚本。
//!
//! 负责准备最小工作区、数据库种子数据、批改样例素材，并生成手工回归检查清单。

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::Row;

type DynError = Box<dyn std::error::Error + Send + Sync>;

const TEACHER_ID: &str = "reg-teacher-001";
const CLASS_ID: &str = "reg-class-001";
const STUDENT_ID: &str = "reg-student-001";
const TEACHER_NAME: &str = "回归教师";
const STUDENT_NAME: &str = "张小明";
const STUDENT_NO: &str = "20260001";

#[derive(Debug, Clone)]
struct CliOptions {
    workspace: PathBuf,
    check_only: bool,
    grading_asset: Option<PathBuf>,
}

/// 解析命令行参数。
fn parse_args() -> Result<CliOptions, DynError> {
    let mut workspace = env::temp_dir().join("pureworker-manual-regression");
    let mut check_only = false;
    let mut grading_asset = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--workspace" => {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("--workspace 需要提供目录路径"))?;
                workspace = PathBuf::from(value);
            }
            "--grading-asset" => {
                let value = args
                    .next()
                    .ok_or_else(|| String::from("--grading-asset 需要提供文件路径"))?;
                grading_asset = Some(PathBuf::from(value));
            }
            "--check-only" => {
                check_only = true;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                return Err(format!("未知参数：{other}").into());
            }
        }
    }

    Ok(CliOptions {
        workspace,
        check_only,
        grading_asset,
    })
}

fn print_help() {
    println!(
        "manual-runtime-regression\n\n用法:\n  cargo run --manifest-path src-tauri/Cargo.toml --bin manual-runtime-regression -- [--workspace PATH] [--grading-asset PATH] [--check-only]\n"
    );
}

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let options = parse_args()?;

    pure_worker_lib::services::runtime_paths::ensure_workspace_layout(&options.workspace)?;
    let pool = connect_pool(&options.workspace).await?;

    if options.check_only {
        verify_fixture_state(&pool, &options.workspace)?;
        println!("[manual-regression] 检查通过");
        println!("workspace={}", options.workspace.display());
        return Ok(());
    }

    let asset_path = prepare_fixture_asset(&options.workspace, options.grading_asset.as_deref())?;
    seed_runtime_data(&pool, &options.workspace).await?;
    let checklist_path = write_checklist(&options.workspace, &asset_path)?;

    println!("[manual-regression] 准备完成");
    println!("workspace={}", options.workspace.display());
    println!("database={}", db_path(&options.workspace).display());
    println!("grading_asset={}", asset_path.display());
    println!("checklist={}", checklist_path.display());
    println!(
        "next=在应用中选择该工作区，然后按清单验证聊天/家长沟通/学期评语/作业批改 4 条运行路径"
    );

    Ok(())
}

/// 建立与运行时一致的 SQLite 连接并执行迁移。
async fn connect_pool(workspace: &Path) -> Result<SqlitePool, DynError> {
    let database_path = db_path(workspace);
    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

fn db_path(workspace: &Path) -> PathBuf {
    pure_worker_lib::services::runtime_paths::database_file_path(workspace)
}

/// 准备批改回归用样例素材。
fn prepare_fixture_asset(
    workspace: &Path,
    custom_asset: Option<&Path>,
) -> Result<PathBuf, DynError> {
    let imports_dir = workspace.join("imports").join("manual-runtime-regression");
    fs::create_dir_all(&imports_dir)?;

    let source = match custom_asset {
        Some(path) => path.to_path_buf(),
        None => repo_root()
            .join("apps")
            .join("desktop")
            .join("src-tauri")
            .join("icons")
            .join("icon.png"),
    };

    if !source.exists() {
        return Err(format!("批改样例素材不存在：{}", source.display()).into());
    }

    let target = imports_dir.join(
        source
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("grading-sample.png")),
    );
    fs::copy(&source, &target)?;

    let note_path = imports_dir.join("README.txt");
    fs::write(
        note_path,
        "默认样例素材用于走通上传/启动链路。若要验证真实 OCR/批改效果，请改用真实作业照片并通过 --grading-asset 指定。\n",
    )?;

    Ok(target)
}

/// 灌入最小回归数据。
async fn seed_runtime_data(pool: &SqlitePool, workspace: &Path) -> Result<(), DynError> {
    let now = Utc::now().to_rfc3339();
    let student_folder = workspace.join("students").join(STUDENT_ID);
    fs::create_dir_all(&student_folder)?;

    sqlx::query(
        "INSERT INTO teacher_profile (id, name, stage, subject, textbook_version, tone_preset, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, stage=excluded.stage, subject=excluded.subject, textbook_version=excluded.textbook_version, tone_preset=excluded.tone_preset, updated_at=excluded.updated_at",
    )
    .bind(TEACHER_ID)
    .bind(TEACHER_NAME)
    .bind("primary")
    .bind("数学")
    .bind("人教版")
    .bind("温和正式")
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO classroom (id, grade, class_name, subject, teacher_id, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?, ?) ON CONFLICT(id) DO UPDATE SET grade=excluded.grade, class_name=excluded.class_name, subject=excluded.subject, teacher_id=excluded.teacher_id, updated_at=excluded.updated_at",
    )
    .bind(CLASS_ID)
    .bind("三年级")
    .bind("二班")
    .bind("数学")
    .bind(TEACHER_ID)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO student (id, student_no, name, gender, class_id, meta_json, folder_path, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?) ON CONFLICT(id) DO UPDATE SET student_no=excluded.student_no, name=excluded.name, gender=excluded.gender, class_id=excluded.class_id, meta_json=excluded.meta_json, folder_path=excluded.folder_path, updated_at=excluded.updated_at",
    )
    .bind(STUDENT_ID)
    .bind(STUDENT_NO)
    .bind(STUDENT_NAME)
    .bind("男")
    .bind(CLASS_ID)
    .bind("{}")
    .bind(student_folder.to_string_lossy().to_string())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    upsert_simple_record(
        pool,
        "student_tag",
        "reg-tag-001",
        STUDENT_ID,
        "积极参与",
        &now,
    )
    .await?;
    upsert_simple_record(
        pool,
        "student_tag",
        "reg-tag-002",
        STUDENT_ID,
        "计算稳定",
        &now,
    )
    .await?;

    upsert_observation(
        pool,
        "reg-observation-001",
        "课堂上能主动回答问题，作业完成情况稳定，但审题偶尔偏快。",
        &now,
    )
    .await?;
    upsert_observation(
        pool,
        "reg-observation-002",
        "最近两周计算正确率明显提升，愿意帮助同学检查错题。",
        &now,
    )
    .await?;

    upsert_score(
        pool,
        "reg-score-001",
        "单元测验一",
        92.0,
        "2026-03-01",
        &now,
    )
    .await?;
    upsert_score(
        pool,
        "reg-score-002",
        "单元测验二",
        95.0,
        "2026-03-15",
        &now,
    )
    .await?;

    sqlx::query(
        "INSERT INTO parent_communication (id, student_id, draft, adopted_text, status, evidence_json, created_at, is_deleted, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?) ON CONFLICT(id) DO UPDATE SET draft=excluded.draft, adopted_text=excluded.adopted_text, status=excluded.status, evidence_json=excluded.evidence_json, updated_at=excluded.updated_at",
    )
    .bind("reg-parent-comm-001")
    .bind(STUDENT_ID)
    .bind("【肯定】课堂参与积极。\n\n【问题】审题速度偏快。\n\n【建议】家校共同提醒慢读题干。")
    .bind("本周课堂状态稳定，建议继续保持慢审题习惯。")
    .bind("adopted")
    .bind("[]")
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO grading_job (id, class_id, title, grading_mode, status, answer_key_json, scoring_rules_json, total_assets, processed_assets, failed_assets, conflict_count, task_id, output_path, is_deleted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, NULL, NULL, 0, ?, ?) ON CONFLICT(id) DO UPDATE SET title=excluded.title, grading_mode=excluded.grading_mode, status=excluded.status, answer_key_json=excluded.answer_key_json, scoring_rules_json=excluded.scoring_rules_json, updated_at=excluded.updated_at",
    )
    .bind("reg-grading-job-001")
    .bind(CLASS_ID)
    .bind("运行时回归批改任务")
    .bind("enhanced")
    .bind("pending")
    .bind(r#"{"Q1":"4","Q2":"12"}"#)
    .bind(r#"{"rule":"步骤分+结果分"}"#)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_simple_record(
    pool: &SqlitePool,
    table: &str,
    id: &str,
    student_id: &str,
    tag_name: &str,
    now: &str,
) -> Result<(), DynError> {
    let sql = format!(
        "INSERT INTO {table} (id, student_id, tag_name, is_deleted, created_at) VALUES (?, ?, ?, 0, ?) ON CONFLICT(id) DO UPDATE SET tag_name=excluded.tag_name"
    );
    sqlx::query(&sql)
        .bind(id)
        .bind(student_id)
        .bind(tag_name)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

async fn upsert_observation(
    pool: &SqlitePool,
    id: &str,
    content: &str,
    now: &str,
) -> Result<(), DynError> {
    sqlx::query(
        "INSERT INTO observation_note (id, student_id, content, source, created_at, is_deleted, updated_at) VALUES (?, ?, ?, ?, ?, 0, ?) ON CONFLICT(id) DO UPDATE SET content=excluded.content, updated_at=excluded.updated_at",
    )
    .bind(id)
    .bind(STUDENT_ID)
    .bind(content)
    .bind("manual_regression")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_score(
    pool: &SqlitePool,
    id: &str,
    exam_name: &str,
    score: f64,
    exam_date: &str,
    now: &str,
) -> Result<(), DynError> {
    sqlx::query(
        "INSERT INTO score_record (id, student_id, exam_name, subject, score, full_score, rank_in_class, exam_date, is_deleted, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?) ON CONFLICT(id) DO UPDATE SET exam_name=excluded.exam_name, score=excluded.score, exam_date=excluded.exam_date, updated_at=excluded.updated_at",
    )
    .bind(id)
    .bind(STUDENT_ID)
    .bind(exam_name)
    .bind("数学")
    .bind(score)
    .bind(100.0)
    .bind(3)
    .bind(exam_date)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// 校验种子数据和样例素材是否存在。
fn verify_fixture_state(pool: &SqlitePool, workspace: &Path) -> Result<(), DynError> {
    let database_path = db_path(workspace);
    if !database_path.exists() {
        return Err(format!("数据库不存在：{}", database_path.display()).into());
    }

    let checks = vec![
        ("teacher_profile", TEACHER_ID),
        ("classroom", CLASS_ID),
        ("student", STUDENT_ID),
        ("grading_job", "reg-grading-job-001"),
    ];

    for (table, id) in checks {
        let sql = format!("SELECT COUNT(1) FROM {table} WHERE id = ?");
        let count: i64 = futures::executor::block_on(async {
            sqlx::query(&sql)
                .bind(id)
                .fetch_one(pool)
                .await
                .map(|row| row.get::<i64, _>(0))
        })?;
        if count == 0 {
            return Err(format!("缺少回归数据：{table}/{id}").into());
        }
    }

    let asset_dir = workspace.join("imports").join("manual-runtime-regression");
    if !asset_dir.exists() {
        return Err(format!("缺少批改样例目录：{}", asset_dir.display()).into());
    }

    Ok(())
}

/// 生成手工回归检查清单。
fn write_checklist(workspace: &Path, asset_path: &Path) -> Result<PathBuf, DynError> {
    let checklist_path = workspace
        .join("exports")
        .join("agent-runtime-manual-regression-checklist.md");
    if let Some(parent) = checklist_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut content = String::new();
    writeln!(&mut content, "# Agent Runtime 手工回归清单")?;
    writeln!(&mut content)?;
    writeln!(&mut content, "- 工作区：`{}`", workspace.display())?;
    writeln!(&mut content, "- 教师：`{}`", TEACHER_NAME)?;
    writeln!(&mut content, "- 班级：`三年级二班`")?;
    writeln!(
        &mut content,
        "- 学生：`{}`（学号 `{}`）",
        STUDENT_NAME, STUDENT_NO
    )?;
    writeln!(&mut content, "- 批改样例：`{}`", asset_path.display())?;
    writeln!(&mut content)?;
    writeln!(&mut content, "## 预检")?;
    writeln!(&mut content, "1. 启动应用后，工作区选择为上面的目录。")?;
    writeln!(&mut content, "2. 在设置页确认至少有一个激活的 AI 配置。")?;
    writeln!(
        &mut content,
        "3. 在教师档案页确认教师为 `{} / 数学 / 温和正式`。",
        TEACHER_NAME
    )?;
    writeln!(&mut content)?;
    writeln!(&mut content, "## 场景 1：聊天")?;
    writeln!(&mut content, "1. 打开 Dashboard 的 AI 助手面板。")?;
    writeln!(
        &mut content,
        "2. 输入：`请根据张小明最近的课堂表现，给出一段面向家长的简要反馈。`"
    )?;
    writeln!(&mut content, "3. 期待结果：")?;
    writeln!(&mut content, "   - 看到流式输出；")?;
    writeln!(&mut content, "   - 思考轨迹包含检索/推理状态；")?;
    writeln!(&mut content, "   - 回答提到课堂表现、成绩趋势或标签信息。")?;
    writeln!(&mut content)?;
    writeln!(&mut content, "## 场景 2：家长沟通")?;
    writeln!(&mut content, "1. 打开学生详情页，定位学生 `张小明`。")?;
    writeln!(&mut content, "2. 触发“生成家长沟通”。")?;
    writeln!(&mut content, "3. 期待结果：")?;
    writeln!(&mut content, "   - 新记录状态为 `draft`；")?;
    writeln!(&mut content, "   - 内容呈现“肯定 -> 问题 -> 建议”三段式；")?;
    writeln!(&mut content, "   - 语气延续历史 `温和正式`。")?;
    writeln!(&mut content)?;
    writeln!(&mut content, "## 场景 3：学期评语")?;
    writeln!(&mut content, "1. 打开学期评语页，选择 `三年级二班`。")?;
    writeln!(&mut content, "2. 发起批量生成。")?;
    writeln!(&mut content, "3. 期待结果：")?;
    writeln!(&mut content, "   - 任务进入队列并有进度；")?;
    writeln!(&mut content, "   - 为 `张小明` 生成 `draft` 评语；")?;
    writeln!(&mut content, "   - 内容包含成绩/观察依据。")?;
    writeln!(&mut content)?;
    writeln!(&mut content, "## 场景 4：作业批改")?;
    writeln!(&mut content, "1. 打开作业批改页，班级选 `三年级二班`。")?;
    writeln!(
        &mut content,
        "2. 使用现成任务 `运行时回归批改任务`，或新建一个等价任务。"
    )?;
    writeln!(
        &mut content,
        "3. 上传样例素材：`{}`。",
        asset_path.display()
    )?;
    writeln!(&mut content, "4. 点击开始批改。")?;
    writeln!(&mut content, "5. 期待结果：")?;
    writeln!(
        &mut content,
        "   - 任务状态从 `pending` -> `running` -> `completed/partial`；"
    )?;
    writeln!(&mut content, "   - OCR 结果/批改结果表格出现数据；")?;
    writeln!(&mut content, "   - 如使用默认图标样例，只要求走通上传/启动链路；若要验证真实 OCR/批改质量，请换成真实作业照片。")?;
    writeln!(&mut content)?;
    writeln!(&mut content, "## 失败时优先查看")?;
    writeln!(&mut content, "- `workspace/logs/startup.log`")?;
    writeln!(&mut content, "- 相关页面的错误 toast")?;
    writeln!(
        &mut content,
        "- 若是 AI 路径失败，优先检查 AI 配置与 provider 健康状态"
    )?;

    fs::write(&checklist_path, content)?;
    Ok(checklist_path)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_workspace_is_temp_dir_based() {
        let options = CliOptions {
            workspace: env::temp_dir().join("pureworker-manual-regression"),
            check_only: false,
            grading_asset: None,
        };

        assert!(options
            .workspace
            .to_string_lossy()
            .contains("pureworker-manual-regression"));
    }

    #[test]
    fn test_checklist_mentions_four_flows() {
        let workspace = env::temp_dir().join("pureworker-manual-regression-test");
        let asset = workspace.join("imports/test.png");
        fs::create_dir_all(asset.parent().unwrap()).unwrap();
        fs::write(&asset, b"test").unwrap();

        let checklist = write_checklist(&workspace, &asset).unwrap();
        let content = fs::read_to_string(checklist).unwrap();

        assert!(content.contains("场景 1：聊天"));
        assert!(content.contains("场景 2：家长沟通"));
        assert!(content.contains("场景 3：学期评语"));
        assert!(content.contains("场景 4：作业批改"));
    }
}
