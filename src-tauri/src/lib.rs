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
            let db_path = app.path().app_data_dir()
                .expect("no app data dir")
                .join("messages.db");
            let script_path = app.path().resource_dir()
                .expect("no resource dir")
                .join("scripts/sync.js");

            // Init DB
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                db::init_db(&conn).ok();
            }

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
