use tauri::{AppHandle, Emitter, State};
use tokio::sync::watch;
use serde::{Deserialize, Serialize};
use crate::db;
use crate::query::build_prompt;
use crate::llm::{self, LlmConfig};
use rusqlite::Connection;
use futures_util::StreamExt;

pub struct DbPath(pub std::path::PathBuf);
pub struct ScriptPath(pub std::path::PathBuf);
pub struct IntervalTx(pub watch::Sender<u64>);

#[derive(Serialize, Deserialize)]
pub struct SettingsPayload {
    pub sync_interval_minutes: String,
    pub initial_lookback_days: String,
    pub llm_provider: String,
    pub llm_api_key: String,
    pub ollama_base_url: String,
    pub ollama_model: String,
}

#[tauri::command]
pub async fn ask_question(
    question: String,
    app: AppHandle,
    db_path: State<'_, DbPath>,
) -> Result<(), String> {
    let conn = Connection::open(&db_path.0).map_err(|e| e.to_string())?;

    // Build LLM config from settings
    let provider = db::get_setting_or(&conn, "llm_provider", "claude");
    let config = match provider.as_str() {
        "openai" => LlmConfig::OpenAi {
            api_key: db::get_setting_or(&conn, "llm_api_key", ""),
        },
        "ollama" => LlmConfig::Ollama {
            base_url: db::get_setting_or(&conn, "ollama_base_url", "http://localhost:11434"),
            model: db::get_setting_or(&conn, "ollama_model", "llama3"),
        },
        _ => LlmConfig::Claude {
            api_key: db::get_setting_or(&conn, "llm_api_key", ""),
        },
    };

    let api_key_empty = match &config {
        LlmConfig::Claude { api_key } | LlmConfig::OpenAi { api_key } => api_key.is_empty(),
        _ => false,
    };
    if api_key_empty {
        let _ = app.emit("llm_error", "Configure sua API key nas configurações.");
        return Ok(());
    }

    // Fetch messages and build prompt
    let messages = db::get_recent_messages(&conn, 200).map_err(|e| e.to_string())?;
    let prompt = build_prompt(&messages, &question);
    drop(conn);

    // Stream response
    let mut stream = match llm::stream_completion(config, prompt).await {
        Ok(s) => s,
        Err(e) => { let _ = app.emit("llm_error", e); return Ok(()); }
    };

    while let Some(result) = stream.next().await {
        match result {
            Ok(token) => { let _ = app.emit("llm_token", token); }
            Err(e) => { let _ = app.emit("llm_error", e); return Ok(()); }
        }
    }
    let _ = app.emit("llm_done", true);
    Ok(())
}

#[tauri::command]
pub async fn check_qr_status(
    script_path: State<'_, ScriptPath>,
) -> Result<bool, String> {
    let output = tokio::process::Command::new("node")
        .arg(&script_path.0)
        .arg("--check-login-only")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_default();
    Ok(v["logged_in"].as_bool().unwrap_or(false))
}

#[tauri::command]
pub fn get_settings(db_path: State<'_, DbPath>) -> Result<SettingsPayload, String> {
    let conn = Connection::open(&db_path.0).map_err(|e| e.to_string())?;
    Ok(SettingsPayload {
        sync_interval_minutes: db::get_setting_or(&conn, "sync_interval_minutes", "5"),
        initial_lookback_days: db::get_setting_or(&conn, "initial_lookback_days", "7"),
        llm_provider: db::get_setting_or(&conn, "llm_provider", "claude"),
        llm_api_key: db::get_setting_or(&conn, "llm_api_key", ""),
        ollama_base_url: db::get_setting_or(&conn, "ollama_base_url", "http://localhost:11434"),
        ollama_model: db::get_setting_or(&conn, "ollama_model", "llama3"),
    })
}

#[tauri::command]
pub fn save_settings(
    payload: SettingsPayload,
    db_path: State<'_, DbPath>,
    interval_tx: State<'_, IntervalTx>,
) -> Result<(), String> {
    let conn = Connection::open(&db_path.0).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "sync_interval_minutes", &payload.sync_interval_minutes).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "initial_lookback_days", &payload.initial_lookback_days).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "llm_provider", &payload.llm_provider).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "llm_api_key", &payload.llm_api_key).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "ollama_base_url", &payload.ollama_base_url).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "ollama_model", &payload.ollama_model).map_err(|e| e.to_string())?;

    // Restart scheduler if interval changed
    if let Ok(minutes) = payload.sync_interval_minutes.parse::<u64>() {
        let _ = interval_tx.0.send(minutes);
    }
    Ok(())
}

#[tauri::command]
pub fn check_prerequisites() -> serde_json::Value {
    let node_ok = std::process::Command::new("node").arg("--version").output().is_ok();
    let playwright_ok = std::process::Command::new("npx")
        .args(["playwright", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    serde_json::json!({ "node": node_ok, "playwright": playwright_ok })
}
