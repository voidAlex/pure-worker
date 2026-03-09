//! PureWorker 核心库模块
//!
//! 包含所有业务模块的导出：命令、数据库、错误定义、数据模型、服务层

pub mod commands;
pub mod database;
pub mod error;
pub mod models;
pub mod services;

use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

pub fn run() {
    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::settings::get_app_settings,
        commands::settings::get_setting,
        commands::settings::update_setting,
        commands::settings::get_settings_by_category,
        commands::ai_config::list_ai_configs,
        commands::ai_config::create_ai_config,
        commands::ai_config::update_ai_config,
        commands::ai_config::delete_ai_config,
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
        // M4 作业批改与题库命令
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
    ]);

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("Failed to export TypeScript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let pool = tauri::async_runtime::block_on(database::init_pool(&app_handle))
                .unwrap_or_else(|error| {
                    eprintln!("[Startup] 数据库初始化失败：{}", error);
                    panic!("数据库初始化失败，应用无法启动");
                });

            app.manage(pool);
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
