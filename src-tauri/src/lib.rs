pub mod db;
pub mod query;
pub mod sync;
pub mod llm;
pub mod commands;

use std::sync::{Arc, atomic::AtomicBool};
use tauri::Manager;
use commands::{DbPath, ScriptPath, IntervalTx};

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
            // In dev mode resource_dir points to target/debug/ where scripts aren't copied.
            // Use the project root (known at compile time) in debug builds.
            let script_path = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent().expect("no parent of CARGO_MANIFEST_DIR")
                    .join("scripts/sync.js")
            } else {
                app.path().resource_dir()
                    .expect("no resource dir")
                    .join("scripts/sync.js")
            };

            // Init DB
            let conn = rusqlite::Connection::open(&db_path).expect("failed to open DB");
            db::init_db(&conn).expect("failed to init DB");

            // Sync scheduler
            let sync_in_progress = Arc::new(AtomicBool::new(false));
            let (interval_tx, interval_rx) = tokio::sync::watch::channel(5u64);

            sync::start_scheduler(
                app_handle,
                db_path.clone(),
                script_path.clone(),
                sync_in_progress,
                interval_rx,
            );

            app.manage(DbPath(db_path));
            app.manage(ScriptPath(script_path));
            app.manage(IntervalTx(interval_tx));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ask_question,
            commands::check_qr_status,
            commands::get_settings,
            commands::save_settings,
            commands::check_prerequisites,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
