//! PureWorker 核心库模块
//!
//! 包含所有业务模块的导出：命令、数据库、错误定义、数据模型、服务层

pub mod commands;
pub mod database;
pub mod error;
pub mod models;
pub mod services;

use specta_typescript::BigIntExportBehavior;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

fn create_specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::settings::get_app_settings,
        commands::settings::get_setting,
        commands::settings::update_setting,
        commands::settings::get_settings_by_category,
        commands::initialization::check_initialization_status,
        commands::initialization::select_directory,
        commands::ai_config::list_ai_configs,
        commands::ai_config::create_ai_config,
        commands::ai_config::update_ai_config,
        commands::ai_config::delete_ai_config,
        commands::ai_config::fetch_provider_models,
        commands::ai_config::get_provider_presets,
        commands::ai_param_preset::list_ai_param_presets,
        commands::ai_param_preset::get_active_ai_param_preset,
        commands::ai_param_preset::create_ai_param_preset,
        commands::ai_param_preset::update_ai_param_preset,
        commands::ai_param_preset::delete_ai_param_preset,
        commands::ai_param_preset::activate_ai_param_preset,
        commands::profile::get_teacher_profile,
        commands::task::list_tasks,
        commands::task::get_task,
        commands::task::recover_task,
        commands::task::list_recoverable_tasks,
        commands::task::renew_task_lease,
        commands::approval::list_pending_approvals,
        commands::approval::list_pending_approvals_for_restore,
        commands::approval::resolve_approval,
        commands::approval::cleanup_expired_approvals,
        commands::approval::list_task_approvals,
        commands::export::health_check,
        commands::export::export_semester_comments,
        commands::global_shortcut::list_global_shortcuts,
        commands::global_shortcut::get_global_shortcut,
        commands::global_shortcut::create_global_shortcut,
        commands::global_shortcut::update_global_shortcut,
        commands::global_shortcut::delete_global_shortcut,
        commands::student_import::import_students,
        commands::classroom::list_classrooms,
        commands::classroom::get_classroom,
        commands::classroom::create_classroom,
        commands::classroom::update_classroom,
        commands::classroom::delete_classroom,
        commands::student::list_students,
        commands::student::get_student,
        commands::student::create_student,
        commands::student::update_student,
        commands::student::delete_student,
        commands::student_tag::list_student_tags,
        commands::student_tag::add_student_tag,
        commands::student_tag::remove_student_tag,
        commands::student_tag::update_student_tag,
        commands::score_record::list_student_scores,
        commands::score_record::create_score_record,
        commands::score_record::update_score_record,
        commands::score_record::delete_score_record,
        commands::observation_note::list_student_observations,
        commands::observation_note::create_observation_note,
        commands::observation_note::update_observation_note,
        commands::observation_note::delete_observation_note,
        commands::parent_communication::list_parent_communications,
        commands::parent_communication::create_parent_communication,
        commands::parent_communication::update_parent_communication,
        commands::parent_communication::delete_parent_communication,
        commands::schedule_event::list_schedule_events,
        commands::schedule_event::get_schedule_event,
        commands::schedule_event::create_schedule_event,
        commands::schedule_event::update_schedule_event,
        commands::schedule_event::delete_schedule_event,
        commands::lesson_record::list_lesson_records,
        commands::lesson_record::get_lesson_record,
        commands::lesson_record::create_lesson_record,
        commands::lesson_record::update_lesson_record,
        commands::lesson_record::delete_lesson_record,
        commands::lesson_record::get_lesson_summary,
        commands::schedule_file::list_schedule_files,
        commands::schedule_file::create_schedule_file,
        commands::schedule_file::delete_schedule_file,
        commands::semester_comment::list_semester_comments,
        commands::semester_comment::create_semester_comment,
        commands::semester_comment::update_semester_comment,
        commands::semester_comment::delete_semester_comment,
        commands::semester_comment::batch_adopt_semester_comments,
        commands::activity_announcement::list_activity_announcements,
        commands::activity_announcement::create_activity_announcement,
        commands::activity_announcement::update_activity_announcement,
        commands::activity_announcement::delete_activity_announcement,
        commands::ai_generation::generate_parent_communication,
        commands::ai_generation::regenerate_parent_communication,
        commands::ai_generation::generate_semester_comment,
        commands::ai_generation::generate_semester_comments_batch,
        commands::ai_generation::get_batch_task_progress,
        commands::ai_generation::generate_activity_announcement,
        commands::chat::chat_with_ai,
        commands::chat::chat_stream,
        commands::chat::list_chat_conversations,
        commands::chat::get_chat_conversation,
        commands::chat::delete_chat_conversation,
        commands::conversation::create_conversation,
        commands::conversation::list_conversations,
        commands::conversation::get_conversation,
        commands::conversation::update_conversation,
        commands::conversation::delete_conversation,
        commands::conversation::list_conversation_messages,
        commands::student::get_student_profile_360,
        commands::memory_search::search_evidence,
        commands::student_memory::init_student_memory,
        commands::student_memory::read_student_memory_timeline,
        commands::student_memory::read_student_memory_by_topic,
        commands::student_memory::read_student_comment_materials,
        commands::student_memory::append_student_memory_note,
        commands::student_memory::check_sensitive_content,
        commands::template_file::list_template_files,
        commands::template_file::get_template_file,
        commands::template_file::create_template_file,
        commands::template_file::update_template_file,
        commands::template_file::delete_template_file,
        commands::watch_folder::list_watch_folders,
        commands::watch_folder::get_watch_folder,
        commands::watch_folder::create_watch_folder,
        commands::watch_folder::update_watch_folder,
        commands::watch_folder::delete_watch_folder,
        commands::storage_lifecycle::get_storage_stats,
        commands::storage_lifecycle::export_workspace,
        commands::storage_lifecycle::archive_workspace,
        commands::storage_lifecycle::erase_workspace,
        commands::assignment_grading::create_grading_job,
        commands::assignment_grading::get_grading_job,
        commands::assignment_grading::list_grading_jobs,
        commands::assignment_grading::update_grading_job,
        commands::assignment_grading::delete_grading_job,
        commands::assignment_grading::add_assignment_assets,
        commands::assignment_grading::list_job_assets,
        commands::assignment_grading::delete_assignment_asset,
        commands::assignment_grading::start_grading,
        commands::assignment_grading::list_job_ocr_results,
        commands::assignment_grading::review_ocr_result,
        commands::assignment_grading::batch_review_ocr_results,
        commands::assignment_grading::list_conflict_results,
        commands::assignment_grading::list_wrong_answers,
        commands::assignment_grading::resolve_wrong_answer,
        commands::assignment_grading::get_practice_sheet,
        commands::assignment_grading::list_student_practice_sheets,
        commands::assignment_grading::generate_practice_sheet,
        commands::assignment_grading::delete_practice_sheet,
        commands::assignment_grading::export_grading_results,
        commands::assignment_grading::list_question_bank,
        commands::assignment_grading::create_question_bank_item,
        commands::skill::list_skills,
        commands::skill::get_skill,
        commands::skill::create_skill,
        commands::skill::update_skill,
        commands::skill::delete_skill,
        commands::skill::check_skill_health,
        commands::skill_executor::execute_skill,
        commands::skill_executor::discover_skills,
        commands::skill_store::list_store_skills,
        commands::skill_store::install_store_skill,
        commands::skill_store::install_store_skill_from_git,
        commands::skill_store::uninstall_store_skill,
        commands::uv_manager::check_uv_health,
        commands::uv_manager::create_skill_env,
        commands::uv_manager::install_uv,
        commands::uv_manager::repair_uv,
        commands::mcp_server::list_mcp_servers,
        commands::mcp_server::get_mcp_server,
        commands::mcp_server::create_mcp_server,
        commands::mcp_server::update_mcp_server,
        commands::mcp_server::delete_mcp_server,
        commands::mcp_server::check_mcp_health,
        commands::teacher_memory::get_teacher_preferences,
        commands::teacher_memory::set_teacher_preference,
        commands::teacher_memory::delete_teacher_preference,
        commands::teacher_memory::list_memory_candidates,
        commands::teacher_memory::confirm_memory_candidate,
        commands::teacher_memory::reject_memory_candidate,
        commands::teacher_memory::load_soul_md,
        commands::teacher_memory::load_user_md,
        commands::teacher_memory::reload_soul_md,
        commands::teacher_memory::build_system_prompt_context,
        commands::teacher_memory::record_preference_pattern,
    ])
}

pub fn export_typescript_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let bindings_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("bindings.ts");

    create_specta_builder().export(
        specta_typescript::Typescript::default().bigint(BigIntExportBehavior::Number),
        bindings_path,
    )?;

    Ok(())
}

/// 启动 Tauri 应用主函数
///
/// 初始化数据库连接池、注册 IPC 命令、挂载事件，然后运行应用
pub fn run() {
    let builder = create_specta_builder();

    #[cfg(debug_assertions)]
    export_typescript_bindings().expect("Failed to export TypeScript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            // 在 release 模式下写入启动日志文件，便于排查安装包崩溃问题
            #[cfg(not(debug_assertions))]
            {
                if let Ok(log_dir) = app.path().app_log_dir() {
                    let _ = std::fs::create_dir_all(&log_dir);
                    let log_path = log_dir.join("startup.log");
                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                    let msg = format!("[{}] PureWorker 启动中...\n", timestamp);
                    let _ = std::fs::write(&log_path, &msg);
                }
            }

            let app_handle = app.handle().clone();
            let pool = tauri::async_runtime::block_on(database::init_pool(&app_handle))
                .unwrap_or_else(|error| {
                    // 数据库初始化失败时，也写入日志
                    #[cfg(not(debug_assertions))]
                    {
                        if let Ok(log_dir) = app.path().app_log_dir() {
                            let log_path = log_dir.join("startup.log");
                            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                            let msg = format!("[{}] 数据库初始化失败：{}\n", timestamp, error);
                            let _ = std::fs::OpenOptions::new()
                                .append(true)
                                .open(&log_path)
                                .and_then(|mut f| {
                                    std::io::Write::write_all(&mut f, msg.as_bytes())
                                });
                        }
                    }
                    eprintln!("[Startup] 数据库初始化失败：{}", error);
                    panic!("数据库初始化失败，应用无法启动");
                });

            // 数据库初始化成功，记录日志
            #[cfg(not(debug_assertions))]
            {
                if let Ok(log_dir) = app.path().app_log_dir() {
                    let log_path = log_dir.join("startup.log");
                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                    let msg = format!("[{}] 数据库初始化成功，应用启动完成\n", timestamp);
                    let _ = std::fs::OpenOptions::new()
                        .append(true)
                        .open(&log_path)
                        .and_then(|mut f| std::io::Write::write_all(&mut f, msg.as_bytes()));
                }
            }

            app.manage(pool.clone());

            // 初始化 Tool Registry
            let registry = crate::services::tool_registry::init_registry();

            // 从数据库加载启用的 MCP 服务器并注册其工具
            let mcp_count = tauri::async_runtime::block_on(async {
                crate::services::mcp_tool_adapter::register_mcp_tools(&pool, registry).await
            });

            match mcp_count {
                Ok(count) => {
                    #[cfg(not(debug_assertions))]
                    {
                        if let Ok(log_dir) = app.path().app_log_dir() {
                            let log_path = log_dir.join("startup.log");
                            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                            let msg =
                                format!("[{}] MCP 工具注册成功：{} 个工具\n", timestamp, count);
                            let _ = std::fs::OpenOptions::new()
                                .append(true)
                                .open(&log_path)
                                .and_then(|mut f| {
                                    std::io::Write::write_all(&mut f, msg.as_bytes())
                                });
                        }
                    }
                    println!("[Startup] MCP 工具注册成功：{} 个工具", count);
                }
                Err(e) => {
                    #[cfg(not(debug_assertions))]
                    {
                        if let Ok(log_dir) = app.path().app_log_dir() {
                            let log_path = log_dir.join("startup.log");
                            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                            let msg = format!("[{}] MCP 工具注册失败：{}\n", timestamp, e);
                            let _ = std::fs::OpenOptions::new()
                                .append(true)
                                .open(&log_path)
                                .and_then(|mut f| {
                                    std::io::Write::write_all(&mut f, msg.as_bytes())
                                });
                        }
                    }
                    eprintln!("[Startup] MCP 工具注册失败：{}", e);
                    // 不阻断启动，继续运行
                }
            }

            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}