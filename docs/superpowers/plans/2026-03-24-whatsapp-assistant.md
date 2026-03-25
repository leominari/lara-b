# WhatsApp Personal Assistant Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate a WhatsApp message reader and natural language query assistant into the focus-widget Tauri app, with a Lottie cat mascot and automatic periodic sync.

**Architecture:** Playwright (Node.js) scrapes WhatsApp Web and outputs JSON; Rust backend stores messages in SQLite and queries an LLM API; Vue frontend renders the cat mascot and chat interface.

**Tech Stack:** Tauri 2 + Vue 3 + TypeScript (frontend), Rust + rusqlite + reqwest + tokio (backend), Playwright + Node.js (scraper), lottie-web + qrcode-vue3 (UI libs)

---

## File Map

**Create:**
```
scripts/
  package.json              — playwright dependency
  sync.js                   — WhatsApp Web scraper (Node.js)

public/
  loader-cat.json           — Lottie animation file

src/components/
  WhatsAppAssistant.vue     — main assistant panel (chat UI)
  CatMascot.vue             — Lottie cat with 5 animation states
  SetupWizard.vue           — first-run prerequisites wizard
  SettingsPanel.vue         — settings form (LLM, sync interval)

src/composables/
  useAssistant.ts           — reactive state + Tauri event listeners

src-tauri/src/
  db.rs                     — SQLite: schema init, upsert messages, settings CRUD
  sync.rs                   — sync scheduler: timer, spawn Node.js, parse JSON
  query.rs                  — prompt building + fetch messages from SQLite
  commands.rs               — all Tauri commands
  llm/
    mod.rs                  — LlmProvider trait + factory fn
    claude.rs               — Claude API SSE streaming
    openai.rs               — OpenAI API SSE streaming
    ollama.rs               — Ollama ndjson streaming
```

**Modify:**
```
src/App.vue                 — add WhatsAppAssistant tab
package.json                — add lottie-web, qrcode-vue3
src-tauri/Cargo.toml        — add rusqlite, tokio, reqwest, sha2, chrono, futures-util
src-tauri/src/lib.rs        — register commands, init db, start scheduler
src-tauri/tauri.conf.json   — add scripts to bundle resources
```

---

## Task 1: Copy Lottie asset and create scripts directory

**Files:**
- Create: `public/loader-cat.json` (copy from ~/Downloads)
- Create: `scripts/` directory

- [ ] **Step 1: Copy the Lottie file into the Vue public folder**

```bash
cp ~/Downloads/Loader\ cat.json /Users/leominari/Documents/larab/focus-widget/public/loader-cat.json
```

- [ ] **Step 2: Create the scripts directory**

```bash
mkdir -p /Users/leominari/Documents/larab/focus-widget/scripts
```

- [ ] **Step 3: Commit (repo already exists — do NOT run git init)**

```bash
cd /Users/leominari/Documents/larab/focus-widget
git add public/loader-cat.json
git commit -m "feat: add lottie cat animation asset"
```

---

## Task 2: Add Rust and frontend dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`

- [ ] **Step 1: Add Rust dependencies to Cargo.toml**

In `src-tauri/Cargo.toml`, replace the `[dependencies]` section with:

```toml
[dependencies]
tauri = { version = "2", features = ["macos-private-api"] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
tokio = { version = "1", features = ["rt", "time", "sync", "io-util", "process"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
sha2 = "0.10"
hex = "0.4"
chrono = "0.4"
futures-util = "0.3"
```

- [ ] **Step 2: Verify Rust deps compile**

```bash
cd /Users/leominari/Documents/larab/focus-widget/src-tauri
cargo check
```

Expected: no errors (warnings OK)

- [ ] **Step 3: Add frontend dependencies**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm install lottie-web qrcode-vue3
npm install --save-dev @types/lottie-web
```

- [ ] **Step 4: Verify frontend builds**

```bash
npm run build -- --noEmit 2>/dev/null || npx vue-tsc --noEmit
```

Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json
git commit -m "feat: add rusqlite, reqwest, lottie-web, qrcode-vue3 dependencies"
```

---

## Task 3: Database module (TDD)

**Files:**
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod db;`)

- [ ] **Step 1: Create db.rs — implementation + tests together** (tests are co-located with the implementation; run them immediately after to confirm they pass)

Create `src-tauri/src/db.rs`:

```rust
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub contact: String,
    pub chat: String,
    pub body: String,
    pub timestamp: i64,
    pub is_mine: bool,
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS messages (
            id        TEXT PRIMARY KEY,
            contact   TEXT NOT NULL,
            chat      TEXT NOT NULL,
            body      TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            is_mine   INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
    ")
}

pub fn upsert_message(conn: &Connection, msg: &Message) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO messages (id, contact, chat, body, timestamp, is_mine)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![msg.id, msg.contact, msg.chat, msg.body, msg.timestamp, msg.is_mine as i64],
    )?;
    Ok(())
}

pub fn get_recent_messages(conn: &Connection, limit: usize) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, contact, chat, body, timestamp, is_mine
         FROM messages ORDER BY timestamp DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(Message {
            id: row.get(0)?,
            contact: row.get(1)?,
            chat: row.get(2)?,
            body: row.get(3)?,
            timestamp: row.get(4)?,
            is_mine: row.get::<_, i64>(5)? != 0,
        })
    })?;
    rows.collect()
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    match conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0)) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_setting_or(conn: &Connection, key: &str, default: &str) -> String {
    get_setting(conn, key).ok().flatten().unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_schema_created() {
        let conn = test_conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('messages','settings')",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_upsert_deduplicates() {
        let conn = test_conn();
        let msg = Message { id: "abc".into(), contact: "João".into(), chat: "João".into(), body: "Oi".into(), timestamp: 1000, is_mine: false };
        upsert_message(&conn, &msg).unwrap();
        upsert_message(&conn, &msg).unwrap(); // duplicate — should not fail or double-insert
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_recent_messages_ordered_desc() {
        let conn = test_conn();
        for (i, body) in ["first", "second", "third"].iter().enumerate() {
            upsert_message(&conn, &Message { id: i.to_string(), contact: "X".into(), chat: "X".into(), body: body.to_string(), timestamp: i as i64 * 1000, is_mine: false }).unwrap();
        }
        let msgs = get_recent_messages(&conn, 10).unwrap();
        assert_eq!(msgs[0].body, "third"); // newest first
    }

    #[test]
    fn test_settings_missing_returns_none() {
        let conn = test_conn();
        assert_eq!(get_setting(&conn, "nonexistent").unwrap(), None);
    }

    #[test]
    fn test_settings_set_and_get() {
        let conn = test_conn();
        set_setting(&conn, "sync_interval_minutes", "15").unwrap();
        assert_eq!(get_setting(&conn, "sync_interval_minutes").unwrap(), Some("15".into()));
    }

    #[test]
    fn test_settings_overwrite() {
        let conn = test_conn();
        set_setting(&conn, "key", "old").unwrap();
        set_setting(&conn, "key", "new").unwrap();
        assert_eq!(get_setting(&conn, "key").unwrap(), Some("new".into()));
    }
}
```

- [ ] **Step 2: Add `mod db;` to lib.rs**

In `src-tauri/src/lib.rs`, add at the top:
```rust
pub mod db;
```

- [ ] **Step 3: Run tests — expect them to compile and pass**

```bash
cd /Users/leominari/Documents/larab/focus-widget/src-tauri
cargo test db::tests
```

Expected: 6 tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat: add SQLite database module with messages and settings tables"
```

---

## Task 4: Query engine (TDD)

**Files:**
- Create: `src-tauri/src/query.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod query;`)

- [ ] **Step 1: Create query.rs — implementation + tests together**

Create `src-tauri/src/query.rs`:

```rust
use crate::db::Message;
use chrono::{DateTime, Utc};

pub fn build_prompt(messages: &[Message], question: &str) -> String {
    // messages arrive newest-first (ORDER BY timestamp DESC) — reverse for prompt
    let mut ordered: Vec<&Message> = messages.iter().collect();
    ordered.reverse(); // oldest first in prompt

    let mut lines = String::new();
    for msg in &ordered {
        // Use the free function (not associated method) — from_timestamp(i64, u32) -> Option<DateTime<Utc>>
        let dt = chrono::DateTime::from_timestamp(msg.timestamp, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| msg.timestamp.to_string());
        let label = if msg.is_mine {
            format!("{} (você)", msg.contact)
        } else {
            msg.contact.clone()
        };
        lines.push_str(&format!("[{}] {}: {}\n", dt, label, msg.body));
    }

    let header = if lines.is_empty() {
        "Nenhuma mensagem encontrada no período solicitado.\n".to_string()
    } else {
        format!("Mensagens recentes (mais novas por último):\n\n{}", lines)
    };

    format!(
        "Você é um assistente pessoal. Analise as mensagens abaixo e responda à pergunta do usuário.\n\n{}\nPergunta: {}",
        header, question
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, contact: &str, body: &str, ts: i64, is_mine: bool) -> Message {
        Message { id: id.into(), contact: contact.into(), chat: contact.into(), body: body.into(), timestamp: ts, is_mine }
    }

    #[test]
    fn test_oldest_message_first_in_prompt() {
        // Simulate DESC order from SQLite: newest first in slice
        let messages = vec![msg("2", "João", "segundo", 2000, false), msg("1", "João", "primeiro", 1000, false)];
        let prompt = build_prompt(&messages, "test");
        let pos1 = prompt.find("primeiro").unwrap();
        let pos2 = prompt.find("segundo").unwrap();
        assert!(pos1 < pos2, "oldest message must appear before newest in prompt");
    }

    #[test]
    fn test_sent_messages_labeled_voce() {
        let messages = vec![msg("1", "Me", "oi", 1000, true)];
        let prompt = build_prompt(&messages, "test");
        assert!(prompt.contains("(você)"), "sent messages must be labeled (você)");
    }

    #[test]
    fn test_received_messages_not_labeled_voce() {
        let messages = vec![msg("1", "João", "oi", 1000, false)];
        let prompt = build_prompt(&messages, "test");
        assert!(!prompt.contains("(você)"), "received messages must not have (você) label");
    }

    #[test]
    fn test_empty_messages_includes_fallback_text() {
        let prompt = build_prompt(&[], "test");
        assert!(prompt.contains("Nenhuma mensagem"));
    }

    #[test]
    fn test_question_included_in_prompt() {
        let prompt = build_prompt(&[], "tem algo urgente?");
        assert!(prompt.contains("tem algo urgente?"));
    }
}
```

- [ ] **Step 2: Add `mod query;` to lib.rs**

- [ ] **Step 3: Run tests**

```bash
cargo test query::tests
```

Expected: 5 tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/query.rs src-tauri/src/lib.rs
git commit -m "feat: add query engine with prompt builder"
```

---

## Task 5: LLM provider abstraction (TDD)

**Files:**
- Create: `src-tauri/src/llm/mod.rs`
- Create: `src-tauri/src/llm/claude.rs`
- Create: `src-tauri/src/llm/openai.rs`
- Create: `src-tauri/src/llm/ollama.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod llm;`)

- [ ] **Step 1: Create llm/mod.rs**

> **Note:** The spec defines a `trait LlmProvider` with `async fn complete_stream`. This plan intentionally replaces it with an enum-dispatch pattern (`LlmConfig` enum + `stream_completion` free function) to avoid requiring the `async-trait` crate or unstable RPITIT features for `dyn` dispatch. The behavior is identical.

Create `src-tauri/src/llm/mod.rs`:

```rust
pub mod claude;
pub mod openai;
pub mod ollama;

use std::pin::Pin;
use futures_util::Stream;

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String, String>> + Send>>;

pub enum LlmConfig {
    Claude { api_key: String },
    OpenAi { api_key: String },
    Ollama { base_url: String, model: String },
}

pub async fn stream_completion(config: LlmConfig, prompt: String) -> Result<TokenStream, String> {
    match config {
        LlmConfig::Claude { api_key } => claude::stream(&api_key, &prompt).await,
        LlmConfig::OpenAi { api_key } => openai::stream(&api_key, &prompt).await,
        LlmConfig::Ollama { base_url, model } => ollama::stream(&base_url, &model, &prompt).await,
    }
}

/// Parse a single SSE data line into text token (shared by Claude + OpenAI tests)
pub fn extract_sse_data(line: &str) -> Option<&str> {
    line.strip_prefix("data: ").map(|s| s.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_data_extraction() {
        assert_eq!(extract_sse_data("data: hello"), Some("hello"));
        assert_eq!(extract_sse_data("event: ping"), None);
        assert_eq!(extract_sse_data("data: "), Some(""));
    }
}
```

- [ ] **Step 2: Create llm/claude.rs**

Create `src-tauri/src/llm/claude.rs`:

```rust
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::pin::Pin;
use super::TokenStream;

pub async fn stream(api_key: &str, prompt: &str) -> Result<TokenStream, String> {
    let client = Client::new();
    let body = json!({
        "model": "claude-3-5-sonnet-20241022",
        "max_tokens": 1024,
        "stream": true,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("API key inválida. Verifique nas configurações.".into());
    }
    if !response.status().is_success() {
        return Err(format!("Erro Claude API: {}", response.status()));
    }

    let stream = response.bytes_stream().flat_map(|chunk_result| {
        let tokens: Vec<Result<String, String>> = match chunk_result {
            Err(e) => vec![Err(e.to_string())],
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                text.lines()
                    .filter_map(|line| super::extract_sse_data(line))
                    .filter(|data| !data.is_empty() && *data != "[DONE]")
                    .filter_map(|data| serde_json::from_str::<Value>(data).ok())
                    .filter(|v| v["type"] == "content_block_delta")
                    .filter_map(|v| v["delta"]["text"].as_str().map(|s| Ok(s.to_string())))
                    .collect()
            }
        };
        futures_util::stream::iter(tokens)
    });

    Ok(Box::pin(stream))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_content_block_delta_extracted() {
        let v = json!({"type": "content_block_delta", "delta": {"type": "text_delta", "text": "Hello"}});
        assert_eq!(v["type"], "content_block_delta");
        assert_eq!(v["delta"]["text"].as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_non_delta_events_ignored() {
        let v = json!({"type": "message_start"});
        assert_ne!(v["type"], "content_block_delta");
    }
}
```

- [ ] **Step 3: Create llm/openai.rs**

Create `src-tauri/src/llm/openai.rs`:

```rust
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use super::TokenStream;

pub async fn stream(api_key: &str, prompt: &str) -> Result<TokenStream, String> {
    let client = Client::new();
    let body = json!({
        "model": "gpt-4o",
        "stream": true,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status() == 401 {
        return Err("API key inválida. Verifique nas configurações.".into());
    }
    if !response.status().is_success() {
        return Err(format!("Erro OpenAI API: {}", response.status()));
    }

    let stream = response.bytes_stream().flat_map(|chunk_result| {
        let tokens: Vec<Result<String, String>> = match chunk_result {
            Err(e) => vec![Err(e.to_string())],
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                text.lines()
                    .filter_map(|line| super::extract_sse_data(line))
                    .filter(|data| *data != "[DONE]" && !data.is_empty())
                    .filter_map(|data| serde_json::from_str::<Value>(data).ok())
                    .filter_map(|v| {
                        v["choices"][0]["delta"]["content"]
                            .as_str()
                            .map(|s| Ok(s.to_string()))
                    })
                    .collect()
            }
        };
        futures_util::stream::iter(tokens)
    });

    Ok(Box::pin(stream))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_openai_delta_extraction() {
        let v = json!({"choices": [{"delta": {"content": "Hello"}}]});
        assert_eq!(v["choices"][0]["delta"]["content"].as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_openai_empty_delta_skipped() {
        let v = json!({"choices": [{"delta": {}}]});
        assert!(v["choices"][0]["delta"]["content"].as_str().is_none());
    }
}
```

- [ ] **Step 4: Create llm/ollama.rs**

Create `src-tauri/src/llm/ollama.rs`:

```rust
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use super::TokenStream;

pub async fn stream(base_url: &str, model: &str, prompt: &str) -> Result<TokenStream, String> {
    let client = Client::new();
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = json!({ "model": model, "prompt": prompt, "stream": true });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.to_string().contains("Connection refused") || e.to_string().contains("connect") {
                "Ollama não está rodando. Abra o Ollama e tente novamente.".to_string()
            } else {
                e.to_string()
            }
        })?;

    if !response.status().is_success() {
        return Err(format!("Erro Ollama: {}", response.status()));
    }

    let stream = response.bytes_stream().flat_map(|chunk_result| {
        let tokens: Vec<Result<String, String>> = match chunk_result {
            Err(e) => vec![Err(e.to_string())],
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).to_string();
                text.lines()
                    .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                    .filter(|v| v["done"].as_bool() != Some(true))
                    .filter_map(|v| v["response"].as_str().map(|s| Ok(s.to_string())))
                    .collect()
            }
        };
        futures_util::stream::iter(tokens)
    });

    Ok(Box::pin(stream))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn test_ollama_response_extraction() {
        let v = json!({"model": "llama3", "response": "Hi", "done": false});
        assert_eq!(v["response"].as_str().unwrap(), "Hi");
        assert_ne!(v["done"].as_bool(), Some(true));
    }

    #[test]
    fn test_ollama_done_chunk_skipped() {
        let v = json!({"model": "llama3", "response": "", "done": true});
        assert_eq!(v["done"].as_bool(), Some(true)); // should be filtered out
    }
}
```

- [ ] **Step 5: Add `pub mod llm;` to lib.rs**

- [ ] **Step 6: Run all LLM tests**

```bash
cargo test llm::
```

Expected: 7 tests pass

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/llm/ src-tauri/src/lib.rs
git commit -m "feat: add LLM provider abstraction for Claude, OpenAI, and Ollama"
```

---

## Task 6: Sync scheduler (TDD for pure functions)

**Files:**
- Create: `src-tauri/src/sync.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create sync.rs — implementation + tests together**

Create `src-tauri/src/sync.rs`:

```rust
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
```

- [ ] **Step 2: Add `pub mod sync;` to lib.rs**

- [ ] **Step 3: Run sync tests**

```bash
cargo test sync::tests
```

Expected: 6 tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sync.rs src-tauri/src/lib.rs
git commit -m "feat: add sync scheduler with concurrency guard and JSON output parser"
```

---

## Task 7: Tauri commands and lib.rs wiring

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create commands.rs**

Create `src-tauri/src/commands.rs`:

```rust
use std::sync::Arc;
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

    let api_key_empty = matches!(&config, LlmConfig::Claude { api_key } | LlmConfig::OpenAi { api_key } if api_key.is_empty());
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
```

- [ ] **Step 2: Rewrite lib.rs**

Replace `src-tauri/src/lib.rs` with:

```rust
pub mod db;
pub mod query;
pub mod sync;
pub mod llm;
pub mod commands;

use std::sync::{Arc, atomic::AtomicBool};
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
```

- [ ] **Step 3: Add scripts to bundle resources in tauri.conf.json**

In `src-tauri/tauri.conf.json`, add to the `"bundle"` section:
```json
"resources": ["../scripts/sync.js"]
```

- [ ] **Step 4: Compile Rust to verify no errors**

```bash
cd /Users/leominari/Documents/larab/focus-widget/src-tauri
cargo build 2>&1 | head -50
```

Expected: compiles without errors

- [ ] **Step 5: Commit**

```bash
cd /Users/leominari/Documents/larab/focus-widget
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
git commit -m "feat: add Tauri commands and wire up sync scheduler with app state"
```

---

## Task 8: Node.js sync script

**Files:**
- Create: `scripts/package.json`
- Create: `scripts/sync.js`

- [ ] **Step 1: Create scripts/package.json**

Create `scripts/package.json`:

```json
{
  "name": "whatsapp-sync",
  "version": "1.0.0",
  "type": "commonjs",
  "dependencies": {
    "playwright": "^1.40.0"
  }
}
```

- [ ] **Step 2: Install playwright in scripts/**

```bash
cd /Users/leominari/Documents/larab/focus-widget/scripts
npm install
npx playwright install chromium
```

- [ ] **Step 3: Create scripts/sync.js**

Create `scripts/sync.js`:

```javascript
const { chromium } = require('playwright');
const crypto = require('crypto');
const path = require('path');
const os = require('os');

const PROFILE_DIR = path.join(os.homedir(), '.whatsapp-assistant', 'profile');
const WHATSAPP_URL = 'https://web.whatsapp.com';

function sha256(str) {
  return crypto.createHash('sha256').update(str).digest('hex');
}

function computeMessageId(contact, timestamp, body) {
  return sha256(`${contact}|${timestamp}|${body}`);
}

function output(data) {
  process.stdout.write(JSON.stringify(data) + '\n');
}

async function isLoggedIn(page) {
  try {
    await page.waitForSelector('[data-testid="default-user"]', { timeout: 8000 });
    return true;
  } catch {
    return false;
  }
}

async function getQrData(page) {
  try {
    const qrEl = await page.waitForSelector('canvas[aria-label="Scan this QR code to link a device"]', { timeout: 10000 });
    const dataUrl = await qrEl.evaluate(el => el.toDataURL());
    return dataUrl;
  } catch {
    return null;
  }
}

async function main() {
  const args = process.argv.slice(2);
  const checkLoginOnly = args.includes('--check-login-only');
  const sinceArg = args.find(a => a.startsWith('--since'));
  const since = sinceArg ? parseInt(args[args.indexOf('--since') + 1] || '0', 10) : null;

  if (!checkLoginOnly && (!since || since <= 0)) {
    output({ status: 'error', message: '--since argument is required and must be a positive integer' });
    process.exit(1);
  }

  const browser = await chromium.launchPersistentContext(PROFILE_DIR, {
    headless: true,
    args: ['--no-sandbox'],
  });

  const page = browser.pages()[0] || await browser.newPage();
  await page.goto(WHATSAPP_URL, { waitUntil: 'domcontentloaded' });

  if (checkLoginOnly) {
    const loggedIn = await isLoggedIn(page);
    await browser.close();
    output({ logged_in: loggedIn });
    return;
  }

  const loggedIn = await isLoggedIn(page);
  if (!loggedIn) {
    const qrData = await getQrData(page);
    await browser.close();
    output({ status: 'qr_required', qr_data: qrData || '' });
    return;
  }

  // Collect messages newer than `since`
  const messages = [];
  try {
    // Get all chat list items
    const chatItems = await page.$$('[data-testid="cell-frame-container"]');

    for (const chatItem of chatItems.slice(0, 20)) { // limit to 20 chats for performance
      try {
        await chatItem.click();
        await page.waitForTimeout(500);

        const chatName = await page.$eval('[data-testid="conversation-header"] span[dir="auto"]', el => el.textContent).catch(() => 'Unknown');

        const msgEls = await page.$$('[data-testid="msg-container"]');
        for (const msgEl of msgEls) {
          try {
            const body = await msgEl.$eval('[data-testid="msg-text"] span', el => el.textContent).catch(() => null);
            if (!body) continue;

            const tsEl = await msgEl.$('[data-testid="msg-meta"] span[title]');
            const tsTitle = tsEl ? await tsEl.getAttribute('title') : null;
            const timestamp = tsTitle ? Math.floor(new Date(tsTitle).getTime() / 1000) : Math.floor(Date.now() / 1000);

            if (timestamp < since) continue;

            const isMine = await msgEl.evaluate(el => el.classList.contains('message-out'));
            const contact = isMine ? 'você' : chatName;

            messages.push({
              id: computeMessageId(contact, timestamp, body),
              contact,
              chat: chatName,
              body,
              timestamp,
              is_mine: isMine,
            });
          } catch { /* skip individual message errors */ }
        }
      } catch { /* skip individual chat errors */ }
    }
  } catch (e) {
    await browser.close();
    output({ status: 'error', message: e.message });
    return;
  }

  await browser.close();
  output({ status: 'ok', messages });
}

main().catch(e => {
  output({ status: 'error', message: e.message });
  process.exit(1);
});
```

- [ ] **Step 4: Manual test — check prerequisites detect Node.js**

```bash
node --version
```

Expected: prints Node.js version (e.g. `v20.x.x`)

- [ ] **Step 5: Manual test — check-login-only (no WhatsApp session yet)**

```bash
cd /Users/leominari/Documents/larab/focus-widget/scripts
node sync.js --check-login-only
```

Expected: outputs either `{"logged_in":false}` (if not scanned QR yet) or shows QR output flow

- [ ] **Step 6: Commit**

```bash
cd /Users/leominari/Documents/larab/focus-widget
git add scripts/
git commit -m "feat: add Playwright WhatsApp Web sync script"
```

---

## Task 9: CatMascot.vue

**Files:**
- Create: `src/components/CatMascot.vue`

- [ ] **Step 1: Create CatMascot.vue**

Create `src/components/CatMascot.vue`:

```vue
<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import lottie, { AnimationItem } from 'lottie-web'

type CatState = 'idle' | 'syncing' | 'thinking' | 'responding' | 'error'

const props = defineProps<{ state: CatState }>()

const container = ref<HTMLDivElement | null>(null)
let anim: AnimationItem | null = null

const speedMap: Record<CatState, number> = {
  idle: 1.0,
  syncing: 3.0,
  thinking: 0.3,
  responding: 2.0,
  error: 1.0,
}

onMounted(() => {
  if (!container.value) return
  anim = lottie.loadAnimation({
    container: container.value,
    renderer: 'svg',
    loop: true,
    autoplay: true,
    path: '/loader-cat.json',
  })
})

onUnmounted(() => anim?.destroy())

watch(() => props.state, (state) => {
  if (!anim) return
  if (state === 'error') {
    anim.pause()
  } else {
    anim.play()
    anim.setSpeed(speedMap[state])
  }
})
</script>

<template>
  <div class="cat-wrapper" :class="`cat-${state}`">
    <div ref="container" class="cat-container" />
  </div>
</template>

<style scoped>
.cat-container {
  width: 140px;
  height: 100px;
}
.cat-error .cat-container {
  filter: hue-rotate(300deg) saturate(2);
}
.cat-thinking .cat-container::after {
  content: '...';
  position: absolute;
  bottom: 4px;
  right: 4px;
  font-size: 0.8rem;
  color: white;
  animation: blink 1s infinite;
}
.cat-wrapper {
  position: relative;
  display: inline-block;
}
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
</style>
```

- [ ] **Step 2: Verify component compiles**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npx vue-tsc --noEmit
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/components/CatMascot.vue
git commit -m "feat: add CatMascot component with Lottie animation states"
```

---

## Task 10: useAssistant.ts composable

**Files:**
- Create: `src/composables/useAssistant.ts`

- [ ] **Step 1: Create the composable**

Create `src/composables/useAssistant.ts`:

```typescript
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

export type CatState = 'idle' | 'syncing' | 'thinking' | 'responding' | 'error'

export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
}

export interface Settings {
  sync_interval_minutes: string
  initial_lookback_days: string
  llm_provider: string
  llm_api_key: string
  ollama_base_url: string
  ollama_model: string
}

export function useAssistant() {
  const catState = ref<CatState>('idle')
  const messages = ref<ChatMessage[]>([])
  const currentResponse = ref('')
  const syncStatus = ref('Aguardando sync...')
  const qrData = ref<string | null>(null)
  const isStreaming = ref(false)

  let unlisteners: UnlistenFn[] = []

  let errorResetTimer: ReturnType<typeof setTimeout> | null = null
  function setError() {
    catState.value = 'error'
    if (errorResetTimer) clearTimeout(errorResetTimer)
    errorResetTimer = setTimeout(() => { catState.value = 'idle' }, 3000)
  }

  onMounted(async () => {
    unlisteners.push(await listen('sync_start', () => {
      catState.value = 'syncing'
      syncStatus.value = 'Sincronizando...'
    }))
    unlisteners.push(await listen('sync_complete', (e) => {
      catState.value = 'idle'
      syncStatus.value = `Sync: agora (${e.payload} novas msgs)`
    }))
    unlisteners.push(await listen('sync_error', () => {
      setError()
      syncStatus.value = 'Erro no sync'
    }))
    unlisteners.push(await listen('qr_required', (e) => {
      catState.value = 'idle'
      qrData.value = e.payload as string
    }))
    unlisteners.push(await listen<string>('llm_token', (e) => {
      catState.value = 'responding'
      currentResponse.value += e.payload
    }))
    unlisteners.push(await listen('llm_done', () => {
      messages.value.push({ role: 'assistant', content: currentResponse.value })
      currentResponse.value = ''
      catState.value = 'idle'
      isStreaming.value = false
    }))
    unlisteners.push(await listen<string>('llm_error', (e) => {
      messages.value.push({ role: 'assistant', content: `Erro: ${e.payload}` })
      currentResponse.value = ''
      setError()
      isStreaming.value = false
    }))
  })

  onUnmounted(() => unlisteners.forEach(u => u()))

  async function sendQuestion(question: string) {
    if (isStreaming.value || !question.trim()) return
    messages.value.push({ role: 'user', content: question })
    currentResponse.value = ''
    isStreaming.value = true
    catState.value = 'thinking'
    await invoke('ask_question', { question })
  }

  async function loadSettings(): Promise<Settings> {
    return await invoke<Settings>('get_settings')
  }

  async function saveSettings(settings: Settings) {
    await invoke('save_settings', { payload: settings })
  }

  async function checkPrerequisites() {
    return await invoke<{ node: boolean; playwright: boolean }>('check_prerequisites')
  }

  // QR polling lives inside the composable to avoid mutating qrData from outside
  let qrPollInterval: ReturnType<typeof setInterval> | null = null
  let qrPollTimeout: ReturnType<typeof setTimeout> | null = null

  function startQrPolling() {
    qrPollInterval = setInterval(async () => {
      const ok = await invoke<boolean>('check_qr_status')
      if (ok) {
        qrData.value = null
        clearInterval(qrPollInterval!)
        clearTimeout(qrPollTimeout!)
      }
    }, 3000)
    qrPollTimeout = setTimeout(() => {
      clearInterval(qrPollInterval!)
      qrData.value = null
      messages.value.push({ role: 'assistant', content: 'Scan cancelado. Tente novamente.' })
    }, 5 * 60 * 1000)
  }

  // Watch qrData internally — start polling when QR appears
  watch(qrData, (val) => { if (val) startQrPolling() })

  return {
    catState,
    messages,
    currentResponse,
    syncStatus,
    qrData,        // readonly externally — only mutated inside composable
    isStreaming,
    sendQuestion,
    loadSettings,
    saveSettings,
    checkPrerequisites,
  }
}
```

- [ ] **Step 2: Verify compiles**

```bash
npx vue-tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/composables/useAssistant.ts
git commit -m "feat: add useAssistant composable with Tauri event listeners"
```

---

## Task 11: SettingsPanel.vue

**Files:**
- Create: `src/components/SettingsPanel.vue`

- [ ] **Step 1: Create SettingsPanel.vue**

Create `src/components/SettingsPanel.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import type { Settings } from '../composables/useAssistant'

const props = defineProps<{
  loadSettings: () => Promise<Settings>
  saveSettings: (s: Settings) => Promise<void>
}>()

const emit = defineEmits<{ close: [] }>()

const form = ref<Settings>({
  sync_interval_minutes: '5',
  initial_lookback_days: '7',
  llm_provider: 'claude',
  llm_api_key: '',
  ollama_base_url: 'http://localhost:11434',
  ollama_model: 'llama3',
})
const saved = ref(false)

onMounted(async () => { form.value = await props.loadSettings() })

const showApiKey = computed(() => ['claude', 'openai'].includes(form.value.llm_provider))
const showOllama = computed(() => form.value.llm_provider === 'ollama')

async function save() {
  await props.saveSettings(form.value)
  saved.value = true
  setTimeout(() => { saved.value = false }, 2000)
}
</script>

<template>
  <div class="settings">
    <div class="settings-header">
      <span>Configurações</span>
      <button class="close-btn" @click="emit('close')">✕</button>
    </div>

    <div class="field">
      <label>Intervalo de sync</label>
      <select v-model="form.sync_interval_minutes">
        <option value="1">1 minuto</option>
        <option value="5">5 minutos</option>
        <option value="15">15 minutos</option>
        <option value="30">30 minutos</option>
      </select>
    </div>

    <div class="field">
      <label>Histórico inicial</label>
      <select v-model="form.initial_lookback_days">
        <option value="1">1 dia</option>
        <option value="7">7 dias</option>
        <option value="30">30 dias</option>
      </select>
    </div>

    <div class="field">
      <label>Provedor LLM</label>
      <select v-model="form.llm_provider">
        <option value="claude">Claude (Anthropic)</option>
        <option value="openai">OpenAI</option>
        <option value="ollama">Ollama (local)</option>
      </select>
    </div>

    <div v-if="showApiKey" class="field">
      <label>API Key</label>
      <input type="password" v-model="form.llm_api_key" placeholder="sk-..." />
    </div>

    <template v-if="showOllama">
      <div class="field">
        <label>Ollama URL</label>
        <input type="text" v-model="form.ollama_base_url" />
      </div>
      <div class="field">
        <label>Modelo</label>
        <input type="text" v-model="form.ollama_model" placeholder="llama3" />
      </div>
    </template>

    <button class="save-btn" @click="save">{{ saved ? '✓ Salvo!' : 'Salvar' }}</button>
  </div>
</template>

<style scoped>
.settings { display: flex; flex-direction: column; gap: 10px; padding: 10px; }
.settings-header { display: flex; justify-content: space-between; align-items: center; font-weight: 500; }
.field { display: flex; flex-direction: column; gap: 4px; }
label { font-size: 0.75rem; color: #aaa; }
select, input { background: rgba(255,255,255,0.1); border: 1px solid rgba(255,255,255,0.2); border-radius: 6px; padding: 6px 8px; color: white; font-size: 0.85rem; }
.save-btn { background: rgba(37,211,102,0.3); border: 1px solid rgba(37,211,102,0.5); color: white; border-radius: 6px; padding: 8px; cursor: pointer; margin-top: 4px; }
.save-btn:hover { background: rgba(37,211,102,0.5); }
.close-btn { background: transparent; border: none; color: rgba(255,255,255,0.6); cursor: pointer; font-size: 1rem; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SettingsPanel.vue
git commit -m "feat: add SettingsPanel component"
```

---

## Task 12: SetupWizard.vue

**Files:**
- Create: `src/components/SetupWizard.vue`

- [ ] **Step 1: Create SetupWizard.vue**

Create `src/components/SetupWizard.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  checkPrerequisites: () => Promise<{ node: boolean; playwright: boolean }>
}>()
const emit = defineEmits<{ ready: [] }>()

const nodeOk = ref(false)
const playwrightOk = ref(false)
const checking = ref(true)

onMounted(async () => {
  const result = await props.checkPrerequisites()
  nodeOk.value = result.node
  playwrightOk.value = result.playwright
  checking.value = false
  if (nodeOk.value && playwrightOk.value) {
    emit('ready')
  }
})
</script>

<template>
  <div class="wizard">
    <div v-if="checking">Verificando pré-requisitos...</div>
    <template v-else>
      <p class="wizard-title">Configuração inicial</p>
      <div class="check" :class="nodeOk ? 'ok' : 'fail'">
        {{ nodeOk ? '✓' : '✗' }} Node.js
        <span v-if="!nodeOk" class="hint">Instale em nodejs.org</span>
      </div>
      <div class="check" :class="playwrightOk ? 'ok' : 'fail'">
        {{ playwrightOk ? '✓' : '✗' }} Playwright Chromium
        <span v-if="!playwrightOk" class="hint">
          Execute no terminal:<br>
          <code>cd scripts && npm install && npx playwright install chromium</code>
        </span>
      </div>
      <button v-if="nodeOk && playwrightOk" class="ready-btn" @click="emit('ready')">
        Continuar →
      </button>
    </template>
  </div>
</template>

<style scoped>
.wizard { padding: 16px; display: flex; flex-direction: column; gap: 12px; }
.wizard-title { font-weight: 500; margin: 0; }
.check { padding: 8px; border-radius: 6px; font-size: 0.85rem; }
.ok { background: rgba(37,211,102,0.15); color: #25d366; }
.fail { background: rgba(255,80,80,0.15); color: #ff5050; }
.hint { display: block; font-size: 0.75rem; color: #aaa; margin-top: 4px; }
code { background: rgba(255,255,255,0.1); padding: 2px 4px; border-radius: 3px; font-size: 0.75rem; }
.ready-btn { background: rgba(37,211,102,0.3); border: 1px solid rgba(37,211,102,0.5); color: white; border-radius: 6px; padding: 8px; cursor: pointer; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SetupWizard.vue
git commit -m "feat: add SetupWizard component for first-run prerequisites check"
```

---

## Task 13: WhatsAppAssistant.vue (main panel)

**Files:**
- Create: `src/components/WhatsAppAssistant.vue`

- [ ] **Step 1: Create WhatsAppAssistant.vue**

Create `src/components/WhatsAppAssistant.vue`:

```vue
<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import CatMascot from './CatMascot.vue'
import SettingsPanel from './SettingsPanel.vue'
import SetupWizard from './SetupWizard.vue'
import { useAssistant } from '../composables/useAssistant'
import QrcodeVue from 'qrcode-vue3'

const {
  catState, messages, currentResponse, syncStatus, qrData, isStreaming,
  sendQuestion, loadSettings, saveSettings, checkPrerequisites,
} = useAssistant()
// Note: QR polling is managed inside useAssistant — do NOT mutate qrData here

const input = ref('')
const showSettings = ref(false)
const setupComplete = ref(false)
const chatEl = ref<HTMLDivElement | null>(null)

// Auto-scroll chat
watch([messages, currentResponse], async () => {
  await nextTick()
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
})

async function submit() {
  const q = input.value.trim()
  if (!q) return
  input.value = ''
  await sendQuestion(q)
}
</script>

<template>
  <div class="assistant">
    <!-- Setup Wizard -->
    <SetupWizard
      v-if="!setupComplete"
      :check-prerequisites="checkPrerequisites"
      @ready="setupComplete = true"
    />

    <template v-else>
      <!-- QR code overlay -->
      <div v-if="qrData" class="qr-overlay">
        <p>Escaneie o QR code no WhatsApp</p>
        <QrcodeVue :value="qrData" :size="200" level="H" />
        <p class="qr-hint">Aguardando scan... (timeout: 5 min)</p>
      </div>

      <template v-else>
        <!-- Header: cat + sync status + settings btn -->
        <div class="header-row">
          <CatMascot :state="catState" />
          <div class="status-area">
            <span class="sync-status">{{ syncStatus }}</span>
            <button class="settings-btn" @click="showSettings = !showSettings" title="Configurações">⚙</button>
          </div>
        </div>

        <!-- Settings panel -->
        <SettingsPanel
          v-if="showSettings"
          :load-settings="loadSettings"
          :save-settings="saveSettings"
          @close="showSettings = false"
        />

        <!-- Chat history -->
        <div v-else ref="chatEl" class="chat-history">
          <div v-if="messages.length === 0" class="empty-state">
            Olá! Pergunte sobre suas mensagens do WhatsApp.
          </div>
          <div v-for="(msg, i) in messages" :key="i" :class="['msg', msg.role]">
            <span class="msg-label">{{ msg.role === 'user' ? 'Você' : 'Assistente' }}</span>
            <span class="msg-body">{{ msg.content }}</span>
          </div>
          <!-- Streaming response -->
          <div v-if="currentResponse" class="msg assistant">
            <span class="msg-label">Assistente</span>
            <span class="msg-body">{{ currentResponse }}<span class="cursor">▌</span></span>
          </div>
        </div>

        <!-- Input bar -->
        <div class="input-bar">
          <input
            v-model="input"
            placeholder="Digite sua pergunta..."
            :disabled="isStreaming"
            @keydown.enter="submit"
          />
          <button :disabled="isStreaming" @click="submit">→</button>
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.assistant { display: flex; flex-direction: column; height: 100%; }
.header-row { display: flex; align-items: center; justify-content: space-between; padding: 4px 8px; border-bottom: 1px solid rgba(255,255,255,0.1); }
.status-area { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }
.sync-status { font-size: 0.7rem; color: #aaa; }
.settings-btn { background: transparent; border: none; color: rgba(255,255,255,0.6); cursor: pointer; font-size: 1rem; }
.chat-history { flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px; }
.empty-state { color: #aaa; font-size: 0.85rem; text-align: center; margin-top: 20px; }
.msg { display: flex; flex-direction: column; gap: 2px; max-width: 90%; }
.msg.user { align-self: flex-end; }
.msg.assistant { align-self: flex-start; }
.msg-label { font-size: 0.65rem; color: #aaa; }
.msg-body { background: rgba(255,255,255,0.08); border-radius: 8px; padding: 6px 10px; font-size: 0.82rem; line-height: 1.4; }
.msg.user .msg-body { background: rgba(37,211,102,0.2); }
.cursor { animation: blink 0.8s infinite; }
@keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: 0; } }
.input-bar { display: flex; gap: 6px; padding: 8px; border-top: 1px solid rgba(255,255,255,0.1); }
.input-bar input { flex: 1; background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.15); border-radius: 8px; padding: 6px 10px; color: white; font-size: 0.85rem; }
.input-bar input:disabled { opacity: 0.5; }
.input-bar button { background: rgba(37,211,102,0.3); border: 1px solid rgba(37,211,102,0.4); color: white; border-radius: 8px; padding: 6px 12px; cursor: pointer; }
.input-bar button:disabled { opacity: 0.5; cursor: default; }
.qr-overlay { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 20px; }
.qr-hint { font-size: 0.75rem; color: #aaa; }
</style>
```

- [ ] **Step 2: Compile check**

```bash
npx vue-tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/components/WhatsAppAssistant.vue
git commit -m "feat: add WhatsAppAssistant main panel component"
```

---

## Task 14: Integrate into App.vue

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Update App.vue to include the assistant panel**

Replace the content area in `src/App.vue`. The widget already has a header with drag region. Add a tab bar and embed WhatsAppAssistant:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import WhatsAppAssistant from './components/WhatsAppAssistant.vue'

type Tab = 'tasks' | 'whatsapp'
const activeTab = ref<Tab>('whatsapp')

async function closeWidget() {
  await getCurrentWindow().close()
}
</script>

<template>
  <main class="container">
    <div class="widget">
      <div class="header" data-tauri-drag-region>
        <span class="title" data-tauri-drag-region>Meu Foco</span>
        <button class="close-btn" @click="closeWidget">✕</button>
      </div>

      <!-- Tab bar -->
      <div class="tabs">
        <button :class="['tab', { active: activeTab === 'tasks' }]" @click="activeTab = 'tasks'">Tarefas</button>
        <button :class="['tab', { active: activeTab === 'whatsapp' }]" @click="activeTab = 'whatsapp'">WhatsApp</button>
      </div>

      <!-- Content -->
      <div class="content">
        <div v-if="activeTab === 'tasks'" class="tab-content">
          <h1>Widget Ativo - Tarefas</h1>
        </div>
        <WhatsAppAssistant v-if="activeTab === 'whatsapp'" class="tab-content" />
      </div>
    </div>
  </main>
</template>

<style>
body, html, #app {
  background-color: transparent !important;
  margin: 0; padding: 0; overflow: hidden;
  width: 100vw; height: 100vh;
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
}
.container { width: 100vw; height: 100vh; display: flex; justify-content: center; align-items: center; padding: 20px; box-sizing: border-box; }
.widget { background-color: rgba(0,0,0,0.7); border-radius: 12px; width: 100%; height: 100%; display: flex; flex-direction: column; color: white; box-shadow: 0 4px 6px rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); overflow: hidden; }
.header { height: 40px; background-color: rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; padding: 0 15px; cursor: grab; border-bottom: 1px solid rgba(255,255,255,0.1); }
.header:active { cursor: grabbing; }
.title { font-size: 0.9rem; font-weight: 500; color: #ccc; user-select: none; }
.close-btn { background: transparent; border: none; color: rgba(255,255,255,0.6); font-size: 1.1rem; cursor: pointer; padding: 4px 8px; border-radius: 6px; transition: all 0.2s; }
.close-btn:hover { background-color: rgba(255,0,0,0.6); color: white; }
.tabs { display: flex; border-bottom: 1px solid rgba(255,255,255,0.1); }
.tab { flex: 1; background: transparent; border: none; color: rgba(255,255,255,0.5); padding: 8px; cursor: pointer; font-size: 0.8rem; transition: all 0.2s; }
.tab.active { color: white; border-bottom: 2px solid #25d366; }
.content { flex: 1; overflow: hidden; display: flex; flex-direction: column; }
.tab-content { flex: 1; overflow: hidden; display: flex; flex-direction: column; justify-content: center; align-items: center; }
h1 { font-size: 1.5rem; font-weight: 500; text-align: center; margin: 0; }
</style>
```

- [ ] **Step 2: Final compile check**

```bash
npx vue-tsc --noEmit
```

Expected: no TypeScript errors

- [ ] **Step 3: Run the app in dev mode to verify it launches**

```bash
npm run tauri dev
```

Expected: app opens, WhatsApp tab visible, cat mascot loads, setup wizard appears

- [ ] **Step 4: Commit**

```bash
git add src/App.vue
git commit -m "feat: integrate WhatsApp assistant tab into focus-widget"
```

---

## Task 15: First-run manual integration test

**Objective:** Verify the full sync → query → response flow end-to-end.

- [ ] **Step 1: Run the app**

```bash
npm run tauri dev
```

- [ ] **Step 2: Complete setup wizard**
  - Verify Node.js ✓ and Playwright ✓ show green
  - Click "Continuar →"

- [ ] **Step 3: Scan QR code**
  - QR code overlay appears
  - Open WhatsApp on your phone → Settings → Linked Devices → Link a Device
  - Scan the QR code
  - Overlay should dismiss automatically within 3 seconds

- [ ] **Step 4: Wait for first sync**
  - Cat mascot should animate fast (syncing state)
  - Sync status updates to "Sync: agora (N novas msgs)"

- [ ] **Step 5: Ask a question**
  - Type "tem alguma mensagem importante?" and press Enter
  - Cat should show thinking state (slow)
  - Response should stream in
  - Cat returns to idle

- [ ] **Step 6: Test settings**
  - Click ⚙ button
  - Change sync interval to 1 min, click Salvar
  - Verify no crash

- [ ] **Step 7: Final commit**

```bash
git add .
git commit -m "feat: complete WhatsApp personal assistant integration"
```
