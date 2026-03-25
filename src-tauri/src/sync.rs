use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use serde_json::Value;
use crate::db::{self, Message};
use rusqlite::Connection;

/// Compute the --since timestamp for the Node.js script.
/// If last_synced_at is None (first run), use now - lookback_days * 86400.
pub fn compute_since(last_synced_at: Option<i64>, lookback_days: i64, now: i64) -> i64 {
    last_synced_at.unwrap_or(now - lookback_days * 86400)
}

/// Parse the Node.js script stdout into a list of messages.
/// Returns Err with a human-readable message on failure or script error.
pub fn parse_sync_output(stdout: &str) -> Result<Vec<Message>, String> {
    let v: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("Invalid JSON from sync script: {}", e))?;
    match v["status"].as_str() {
        Some("ok") => {
            let msgs: Vec<Message> = serde_json::from_value(v["messages"].clone())
                .map_err(|e| format!("Failed to parse messages: {}", e))?;
            Ok(msgs)
        }
        Some("qr_required") => Err("qr_required".to_string()),
        Some("error") => Err(v["message"].as_str().unwrap_or("Unknown error").to_string()),
        _ => Err("Unknown status in sync script output".to_string()),
    }
}

pub fn start_scheduler(
    app: AppHandle,
    db_path: std::path::PathBuf,
    script_path: std::path::PathBuf,
    sync_in_progress: Arc<AtomicBool>,
    interval_rx: tokio::sync::watch::Receiver<u64>,
) {
    tauri::async_runtime::spawn(async move {
        let mut rx = interval_rx;
        let mut interval = tokio::time::interval(Duration::from_secs(*rx.borrow() * 60));
        interval.tick().await; // first tick fires immediately — skip it

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if sync_in_progress.swap(true, Ordering::SeqCst) {
                        continue; // already running
                    }
                    let _ = app.emit("sync_start", ());
                    run_sync(&app, &db_path, &script_path).await;
                    sync_in_progress.store(false, Ordering::SeqCst);
                }
                _ = rx.changed() => {
                    let secs = *rx.borrow() * 60;
                    interval = tokio::time::interval(Duration::from_secs(secs));
                    interval.tick().await; // consume the immediate tick
                }
            }
        }
    });
}

async fn run_sync(app: &AppHandle, db_path: &std::path::PathBuf, script_path: &std::path::PathBuf) {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => { let _ = app.emit("sync_error", e.to_string()); return; }
    };
    db::init_db(&conn).ok();

    let last_synced: Option<i64> = db::get_setting(&conn, "last_synced_at")
        .ok().flatten()
        .and_then(|s| s.parse().ok());
    let lookback: i64 = db::get_setting(&conn, "initial_lookback_days")
        .ok().flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let now = chrono::Utc::now().timestamp();
    let since = compute_since(last_synced, lookback, now);

    let output = tokio::process::Command::new("node")
        .arg(script_path)
        .arg("--since")
        .arg(since.to_string())
        .output()
        .await;

    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => { let _ = app.emit("sync_error", e.to_string()); return; }
    };

    match parse_sync_output(&stdout) {
        Ok(messages) => {
            for msg in &messages {
                db::upsert_message(&conn, msg).ok();
            }
            db::set_setting(&conn, "last_synced_at", &now.to_string()).ok();
            let _ = app.emit("sync_complete", messages.len());
        }
        Err(e) if e == "qr_required" => {
            let v: Value = serde_json::from_str(&stdout).unwrap_or_default();
            let qr_data = v["qr_data"].as_str().unwrap_or("").to_string();
            let _ = app.emit("qr_required", qr_data);
        }
        Err(e) => {
            let _ = app.emit("sync_error", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_since_first_run() {
        let now = 1_711_234_567i64;
        let since = compute_since(None, 7, now);
        assert_eq!(since, now - 7 * 86400);
    }

    #[test]
    fn test_compute_since_subsequent_run() {
        let last = 1_711_000_000i64;
        let since = compute_since(Some(last), 7, 1_711_234_567);
        assert_eq!(since, last);
    }

    #[test]
    fn test_parse_ok_output() {
        let json = r#"{"status":"ok","messages":[{"id":"abc","contact":"João","chat":"João","body":"Oi","timestamp":1000,"is_mine":false}]}"#;
        let msgs = parse_sync_output(json).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].contact, "João");
    }

    #[test]
    fn test_parse_qr_required() {
        let json = r#"{"status":"qr_required","qr_data":"somedata"}"#;
        assert_eq!(parse_sync_output(json).unwrap_err(), "qr_required");
    }

    #[test]
    fn test_parse_error_status() {
        let json = r#"{"status":"error","message":"Node not found"}"#;
        assert!(parse_sync_output(json).is_err());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_sync_output("not json").is_err());
    }
}
