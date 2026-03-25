use std::time::Duration;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use rusqlite::Connection;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::db::Message;

#[derive(Debug, PartialEq)]
pub enum BaileysLine {
    Messages(Vec<Message>),
    Qr(String),
    Ready,
    Logout,
    Error(String),
}

pub fn parse_baileys_line(line: &str) -> Result<BaileysLine, String> {
    let v: serde_json::Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    match v["type"].as_str() {
        Some("messages") => {
            let msgs: Vec<Message> = serde_json::from_value(v["messages"].clone())
                .map_err(|e| format!("Failed to parse messages: {}", e))?;
            Ok(BaileysLine::Messages(msgs))
        }
        Some("qr") => {
            let qr_data = v["qr_data"].as_str().unwrap_or("").to_string();
            Ok(BaileysLine::Qr(qr_data))
        }
        Some("ready") => Ok(BaileysLine::Ready),
        Some("logout") => Ok(BaileysLine::Logout),
        Some("error") => {
            let msg = v["message"].as_str().unwrap_or("unknown error").to_string();
            Ok(BaileysLine::Error(msg))
        }
        _ => Err(format!("Unknown baileys line type: {}", v["type"])),
    }
}

pub fn start_baileys(
    app: AppHandle,
    db_path: std::path::PathBuf,
    script_path: std::path::PathBuf,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            let mut child = match tokio::process::Command::new("node")
                .arg(&script_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[baileys] failed to spawn: {}", e);
                    let _ = app.emit("sync_error", e.to_string());
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            // Forward stderr to eprintln for diagnostics
            if let Some(stderr) = child.stderr.take() {
                let mut lines = BufReader::new(stderr).lines();
                tauri::async_runtime::spawn(async move {
                    while let Ok(Some(line)) = lines.next_line().await {
                        eprintln!("[baileys stderr] {}", line);
                    }
                });
            }

            // Read stdout line by line
            if let Some(stdout) = child.stdout.take() {
                let app_inner = app.clone();
                let db_path_inner = db_path.clone();

                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(raw)) = lines.next_line().await {
                    let line = raw.trim().to_string();
                    if line.is_empty() { continue; }

                    match parse_baileys_line(&line) {
                        Ok(BaileysLine::Qr(data)) => {
                            let _ = app_inner.emit("qr_required", data);
                        }
                        Ok(BaileysLine::Ready) => {
                            let _ = app_inner.emit("sync_start", ());
                        }
                        Ok(BaileysLine::Messages(msgs)) => {
                            if let Ok(conn) = Connection::open(&db_path_inner) {
                                for msg in &msgs {
                                    crate::db::upsert_message(&conn, msg).ok();
                                }
                            }
                            let _ = app_inner.emit("sync_complete", msgs.len());
                        }
                        Ok(BaileysLine::Logout) => {
                            let _ = app_inner.emit("sync_error", "Sessão encerrada. Reinicie o app para fazer login novamente.");
                            return; // no restart on logout
                        }
                        Ok(BaileysLine::Error(e)) => {
                            let _ = app_inner.emit("sync_error", e);
                        }
                        Err(e) => {
                            eprintln!("[baileys] parse error: {} — line: {}", e, line);
                        }
                    }
                }
            }

            // Process exited — check exit code
            match child.wait().await {
                Ok(status) if status.success() => {
                    eprintln!("[baileys] process exited cleanly (logout). Not restarting.");
                    return;
                }
                Ok(status) => {
                    eprintln!("[baileys] process exited with {}. Restarting in 5s...", status);
                }
                Err(e) => {
                    eprintln!("[baileys] wait error: {}. Restarting in 5s...", e);
                }
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_messages() {
        let json = r#"{"type":"messages","messages":[{"id":"abc","contact":"João","chat":"João","body":"Oi","timestamp":1000,"is_mine":false}]}"#;
        let result = parse_baileys_line(json).unwrap();
        match result {
            BaileysLine::Messages(msgs) => {
                assert_eq!(msgs.len(), 1);
                assert_eq!(msgs[0].contact, "João");
                assert_eq!(msgs[0].id, "abc");
            }
            _ => panic!("expected Messages"),
        }
    }

    #[test]
    fn test_parse_qr() {
        let json = r#"{"type":"qr","qr_data":"data:image/png;base64,abc123"}"#;
        assert_eq!(
            parse_baileys_line(json).unwrap(),
            BaileysLine::Qr("data:image/png;base64,abc123".to_string())
        );
    }

    #[test]
    fn test_parse_ready() {
        let json = r#"{"type":"ready"}"#;
        assert_eq!(parse_baileys_line(json).unwrap(), BaileysLine::Ready);
    }

    #[test]
    fn test_parse_logout() {
        let json = r#"{"type":"logout"}"#;
        assert_eq!(parse_baileys_line(json).unwrap(), BaileysLine::Logout);
    }

    #[test]
    fn test_parse_error() {
        let json = r#"{"type":"error","message":"bad_session"}"#;
        assert_eq!(
            parse_baileys_line(json).unwrap(),
            BaileysLine::Error("bad_session".to_string())
        );
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_baileys_line("not json").is_err());
    }

    #[test]
    fn test_parse_unknown_type() {
        let json = r#"{"type":"unknown"}"#;
        assert!(parse_baileys_line(json).is_err());
    }
}
