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
        commands::profile::get_teacher_profile,
        commands::task::list_tasks,
        commands::approval::list_pending_approvals,
        commands::export::health_check,
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
        commands::student::get_student_profile_360,
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
