pub mod db;
pub mod query;
pub mod sync;
pub mod llm;
pub mod commands;

use tauri::Manager;
use commands::{DbPath, ScriptPath};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Resolve paths
            let app_data = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&app_data).expect("failed to create app data dir");
            let db_path = app_data.join("messages.db");

            let script_path = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent().expect("no parent of CARGO_MANIFEST_DIR")
                    .join("scripts/baileys.js")
            } else {
                app.path().resource_dir()
                    .expect("no resource dir")
                    .join("scripts/baileys.js")
            };

            // Init DB
            let conn = rusqlite::Connection::open(&db_path).expect("failed to open DB");
            db::init_db(&conn).expect("failed to init DB");

            // Start persistent Baileys process
            sync::start_baileys(app_handle, db_path.clone(), script_path.clone());

            app.manage(DbPath(db_path));
            app.manage(ScriptPath(script_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ask_question,
            commands::get_settings,
            commands::save_settings,
            commands::check_prerequisites,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
