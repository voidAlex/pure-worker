pub mod commands;
pub mod database;
pub mod error;

use tauri_specta::{collect_commands, Builder};

pub fn run() {
    let builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::settings::get_app_settings,
        commands::profile::get_teacher_profile,
        commands::task::list_tasks,
        commands::approval::list_pending_approvals,
        commands::export::health_check,
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
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
