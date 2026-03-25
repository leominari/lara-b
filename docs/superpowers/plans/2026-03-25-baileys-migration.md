# Baileys Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Playwright-based `sync.js` with a persistent Baileys WebSocket process that streams WhatsApp messages to Rust in real time.

**Architecture:** A long-running `baileys.js` Node process connects directly to WhatsApp's WebSocket protocol via Baileys. It writes JSON lines to stdout (qr / ready / messages / error / logout). Rust's `start_baileys` reads the stdout stream line-by-line, upserts messages to SQLite, and emits Tauri events to the Vue frontend. The process restarts automatically on non-zero exit.

**Tech Stack:** Vue 3, Tauri 2, Rust (tokio async), `@whiskeysockets/baileys ^6.7`, `qrcode ^1.5`, `pino` (silence Baileys logger)

**Spec:** `docs/superpowers/specs/2026-03-25-baileys-migration-design.md`

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `scripts/package.json` | Modify | Remove Playwright, add Baileys + qrcode |
| `scripts/baileys.js` | Create | Persistent Baileys process, stdout JSON lines |
| `scripts/sync.js` | Delete | Replaced by baileys.js |
| `src-tauri/src/sync.rs` | Rewrite | BaileysLine enum, parse_baileys_line, start_baileys |
| `src-tauri/src/commands.rs` | Modify | Remove check_qr_status + IntervalTx; update check_prerequisites |
| `src-tauri/src/lib.rs` | Modify | Wire start_baileys, remove IntervalTx, update paths + handler |
| `src/components/SetupWizard.vue` | Modify | Remove playwright check row |

---

## Task 1: Update scripts/package.json and install Baileys

**Files:**
- Modify: `scripts/package.json`

- [ ] **Step 1: Replace dependencies**

Replace the full content of `scripts/package.json` with:

```json
{
  "name": "whatsapp-sync",
  "version": "2.0.0",
  "type": "commonjs",
  "dependencies": {
    "@whiskeysockets/baileys": "^6.7.0",
    "qrcode": "^1.5.3",
    "pino": "^8.0.0"
  }
}
```

- [ ] **Step 2: Install deps**

```bash
cd /Users/leominari/Documents/larab/focus-widget/scripts
rm -rf node_modules package-lock.json
npm install
```

Expected: installs without errors. `node_modules/@whiskeysockets/baileys` exists.

- [ ] **Step 3: Commit**

```bash
git add scripts/package.json scripts/package-lock.json
git commit -m "chore: replace playwright with baileys in scripts/package.json"
```

---

## Task 2: Write `scripts/baileys.js`

**Files:**
- Create: `scripts/baileys.js`
- Delete: `scripts/sync.js`

- [ ] **Step 1: Create `scripts/baileys.js`**

```js
'use strict';

const {
  makeWASocket,
  useMultiFileAuthState,
  DisconnectReason,
  fetchLatestBaileysVersion,
} = require('@whiskeysockets/baileys');
const qrcode = require('qrcode');
const pino = require('pino');
const path = require('path');
const os = require('os');
const fs = require('fs');

const AUTH_DIR = path.join(os.homedir(), '.whatsapp-assistant', 'baileys-auth');

function output(data) {
  process.stdout.write(JSON.stringify(data) + '\n');
}

const groupNameCache = {};

async function connectToWhatsApp() {
  fs.mkdirSync(AUTH_DIR, { recursive: true });

  const { state, saveCreds } = await useMultiFileAuthState(AUTH_DIR);
  const { version } = await fetchLatestBaileysVersion();

  const sock = makeWASocket({
    version,
    auth: state,
    printQRInTerminal: false,
    logger: pino({ level: 'silent' }),
  });

  sock.ev.on('creds.update', saveCreds);

  sock.ev.on('connection.update', async ({ connection, lastDisconnect, qr }) => {
    if (qr) {
      try {
        const url = await qrcode.toDataURL(qr);
        output({ type: 'qr', qr_data: url });
      } catch (e) {
        output({ type: 'error', message: 'QR generation failed: ' + e.message });
      }
    }

    if (connection === 'open') {
      output({ type: 'ready' });
    }

    if (connection === 'close') {
      const statusCode = lastDisconnect?.error?.output?.statusCode;

      if (statusCode === DisconnectReason.loggedOut) {
        output({ type: 'logout' });
        process.exit(0);
      }

      if (statusCode === DisconnectReason.badSession) {
        fs.rmSync(AUTH_DIR, { recursive: true, force: true });
        output({ type: 'error', message: 'bad_session' });
        process.exit(1);
      }

      // All other reasons: reconnect
      connectToWhatsApp();
    }
  });

  sock.ev.on('messages.upsert', async ({ messages, type }) => {
    if (type !== 'notify') return;

    const formatted = [];
    for (const msg of messages) {
      try {
        if (msg.key.fromMe) continue;

        const body =
          msg.message?.conversation ||
          msg.message?.extendedTextMessage?.text ||
          '';
        if (!body) continue;

        const isGroup = msg.key.remoteJid?.endsWith('@g.us');
        const senderJid = isGroup ? msg.key.participant : msg.key.remoteJid;
        const contact = msg.pushName || (senderJid ? senderJid.split('@')[0] : 'Unknown');

        let chat = contact;
        if (isGroup) {
          const gid = msg.key.remoteJid;
          if (!groupNameCache[gid]) {
            try {
              const meta = await sock.groupMetadata(gid);
              groupNameCache[gid] = meta.subject;
            } catch {
              groupNameCache[gid] = gid.split('@')[0];
            }
          }
          chat = groupNameCache[gid];
        }

        formatted.push({
          id: msg.key.id,
          contact,
          chat,
          body,
          timestamp: Number(msg.messageTimestamp),
          is_mine: false,
        });
      } catch { /* skip malformed message */ }
    }

    if (formatted.length > 0) {
      output({ type: 'messages', messages: formatted });
    }
  });
}

connectToWhatsApp().catch(e => {
  output({ type: 'error', message: e.message });
  process.exit(1);
});
```

- [ ] **Step 2: Smoke-test the script syntax**

```bash
cd /Users/leominari/Documents/larab/focus-widget/scripts
node --check baileys.js
```

Expected: no output (syntax OK).

- [ ] **Step 3: Delete sync.js**

```bash
rm /Users/leominari/Documents/larab/focus-widget/scripts/sync.js
```

- [ ] **Step 4: Commit**

```bash
git add scripts/baileys.js scripts/sync.js
git commit -m "feat: add baileys.js persistent WhatsApp sync, remove sync.js"
```

---

## Task 3: Rewrite `src-tauri/src/sync.rs`

**Files:**
- Modify: `src-tauri/src/sync.rs`

This is the core Rust change. We follow TDD: write tests for `parse_baileys_line` first, then implement it, then implement `start_baileys`.

- [ ] **Step 1: Write failing tests**

Replace the full content of `src-tauri/src/sync.rs` with just the test module and the type stubs needed to compile it:

```rust
use crate::db::Message;

#[derive(Debug, PartialEq)]
pub enum BaileysLine {
    Messages(Vec<Message>),
    Qr(String),
    Ready,
    Logout,
    Error(String),
}

pub fn parse_baileys_line(_line: &str) -> Result<BaileysLine, String> {
    todo!()
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
```

- [ ] **Step 2: Run tests — confirm they panic on `todo!()`**

```bash
cd /Users/leominari/Documents/larab/focus-widget/src-tauri
cargo test sync 2>&1 | tail -15
```

Expected: tests compile but fail/panic on `todo!()`.

- [ ] **Step 3: Implement `parse_baileys_line`**

Replace the `parse_baileys_line` stub with the real implementation:

```rust
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
```

- [ ] **Step 4: Run tests — confirm all pass**

```bash
cargo test sync 2>&1 | tail -15
```

Expected: `test result: ok. 7 passed`.

- [ ] **Step 5: Implement `start_baileys`**

Add the following imports at the **top of the file** (module scope, before `BaileysLine`):

```rust
use std::time::Duration;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use rusqlite::Connection;
use tokio::io::{AsyncBufReadExt, BufReader};
```

Then add `start_baileys` after `parse_baileys_line` (before the `#[cfg(test)]` block):

```rust
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
```

- [ ] **Step 6: Verify Rust compiles**

```bash
cargo check 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 7: Run all Rust tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass (including the 7 new ones).

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/sync.rs
git commit -m "feat: rewrite sync.rs with BaileysLine enum and start_baileys streaming"
```

---

## Task 4: Update `src-tauri/src/commands.rs`

**Files:**
- Modify: `src-tauri/src/commands.rs`

Remove `check_qr_status`, remove `IntervalTx` from `save_settings`, update `check_prerequisites`.

- [ ] **Step 1: Remove `check_qr_status` function**

Delete the entire `check_qr_status` function — from `#[tauri::command]` above it through the closing `}`, identified by the `--check-login-only` string inside it.

- [ ] **Step 2: Remove `IntervalTx` from `save_settings`**

Replace the `save_settings` function signature and body:

```rust
#[tauri::command]
pub fn save_settings(
    payload: SettingsPayload,
    db_path: State<'_, DbPath>,
) -> Result<(), String> {
    let conn = Connection::open(&db_path.0).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "sync_interval_minutes", &payload.sync_interval_minutes).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "initial_lookback_days", &payload.initial_lookback_days).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "llm_provider", &payload.llm_provider).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "llm_api_key", &payload.llm_api_key).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "ollama_base_url", &payload.ollama_base_url).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "ollama_model", &payload.ollama_model).map_err(|e| e.to_string())?;
    db::set_setting(&conn, "bubble_timeout_seconds", &payload.bubble_timeout_seconds).map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 3: Remove `IntervalTx` struct and import**

Delete `pub struct IntervalTx(pub watch::Sender<u64>);` from the top of the file.

Delete the `use tokio::sync::watch;` import (no longer needed).

- [ ] **Step 4: Update `check_prerequisites` — remove playwright check**

Replace the `check_prerequisites` function:

```rust
#[tauri::command]
pub async fn check_prerequisites() -> serde_json::Value {
    let node_ok = tokio::process::Command::new("node")
        .arg("--version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    serde_json::json!({ "node": node_ok })
}
```

- [ ] **Step 5: Verify Rust compiles**

```bash
cd /Users/leominari/Documents/larab/focus-widget/src-tauri
cargo check 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: remove check_qr_status and IntervalTx, update check_prerequisites"
```

---

## Task 5: Update `src-tauri/src/lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace the full content of `lib.rs`**

```rust
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
```

- [ ] **Step 2: Verify Rust compiles**

```bash
cargo check 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Run all Rust tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: wire start_baileys in lib.rs, remove IntervalTx and scheduler"
```

---

## Task 6: Update `src/components/SetupWizard.vue`

**Files:**
- Modify: `src/components/SetupWizard.vue`

Remove the `playwright` check — `check_prerequisites` now only returns `{ node: boolean }`.

- [ ] **Step 1: Update script setup**

Replace the script section:

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  checkPrerequisites: () => Promise<{ node: boolean }>
}>()
const emit = defineEmits<{ ready: [] }>()

const nodeOk = ref(false)
const checking = ref(true)

onMounted(async () => {
  const result = await props.checkPrerequisites()
  nodeOk.value = result.node
  checking.value = false
  if (nodeOk.value) {
    emit('ready')
  }
})
</script>
```

- [ ] **Step 2: Update template**

Replace the template section:

```vue
<template>
  <div class="wizard">
    <div v-if="checking">Verificando pré-requisitos...</div>
    <template v-else>
      <p class="wizard-title">Configuração inicial</p>
      <div class="check" :class="nodeOk ? 'ok' : 'fail'">
        {{ nodeOk ? '✓' : '✗' }} Node.js
        <span v-if="!nodeOk" class="hint">Instale em nodejs.org</span>
      </div>
      <button v-if="nodeOk" class="ready-btn" @click="emit('ready')">
        Continuar →
      </button>
    </template>
  </div>
</template>
```

- [ ] **Step 3: TypeScript check**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run build 2>&1 | tail -8
```

Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add src/components/SetupWizard.vue
git commit -m "feat: remove playwright prerequisite check from SetupWizard"
```

---

## Task 7: Integration verification

- [ ] **Step 1: Full frontend build**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run build 2>&1 | tail -8
```

Expected: `✓ built in Xs` — no errors.

- [ ] **Step 2: Full Rust build**

```bash
cd src-tauri
cargo build 2>&1 | tail -5
```

Expected: `Finished` — no errors.

- [ ] **Step 3: Run all Rust tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 4: Smoke test baileys.js**

```bash
cd /Users/leominari/Documents/larab/focus-widget/scripts
node --check baileys.js && echo "syntax OK"
```

Expected: `syntax OK`.

- [ ] **Step 5: Push to GitHub**

```bash
cd /Users/leominari/Documents/larab/focus-widget
git push
```

- [ ] **Step 6: Tag migration complete (optional)**

```bash
git tag v0.2.0-baileys
git push --tags
```
