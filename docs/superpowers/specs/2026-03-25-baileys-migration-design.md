# Baileys Migration Design Spec

**Date:** 2026-03-25
**Status:** Approved

## Overview

Replace the Playwright-based `sync.js` (spawn-per-run, DOM scraping) with a persistent Baileys process that connects directly to WhatsApp's WebSocket protocol and streams new messages to Rust in real time.

---

## Architecture

```
App launch
  → Rust spawns baileys.js (persistent, stdout piped)
  → Rust reads stdout line by line (BufReader loop)
  → On QR line   → emit "qr_required" to frontend
  → On messages  → upsert to SQLite, emit "sync_complete"
  → On crash     → Rust restarts the process after 5s delay
```

---

## Script: `scripts/baileys.js`

**Behavior:**
- Loads Baileys auth state from `~/.whatsapp-assistant/baileys-auth/`
- On first run (no auth): emits QR as base64 PNG data URL, waits for scan
- On connected: listens to `messages.upsert` event
- Filters `upsert.type === 'notify'` (new messages only, not history load)
- Outputs JSON lines to stdout:

```json
{"type":"qr","qr_data":"data:image/png;base64,..."}
{"type":"messages","messages":[...]}
{"type":"ready"}
{"type":"error","message":"..."}
```

**Message shape** (identical to current SQLite schema):
```json
{
  "id": "<sha256(contact|timestamp|body)>",
  "contact": "João Silva",
  "chat": "João Silva",
  "body": "Oi, tudo bem?",
  "timestamp": 1711234567,
  "is_mine": false
}
```

**Auth persistence:** Baileys saves multi-file auth to `~/.whatsapp-assistant/baileys-auth/`. No browser profile needed. Session survives restarts.

**QR flow:** Baileys emits QR as a Buffer via `connection.update`. Convert to base64 PNG using the `qrcode` npm package and emit `{"type":"qr","qr_data":"data:image/png;base64,..."}`.

**Reconnection:** Baileys handles reconnection internally via `DisconnectReason`. Script exits only on `loggedOut` — Rust restarts it otherwise.

---

## Rust: `src-tauri/src/sync.rs`

**Replace `start_scheduler` with `start_baileys`:**

```rust
pub fn start_baileys(app, db_path, script_path, sync_in_progress)
```

- Spawns `node baileys.js` with `stdout: Stdio::piped()`
- Reads stdout with `tokio::io::BufReader` + `lines()` in async loop
- Parses each line with updated `parse_baileys_line(line)`
- On process exit: waits 5s, restarts (unless app is shutting down)
- Removes `interval_rx` / `IntervalTx` — no more polling interval

**Updated JSON parser (`parse_baileys_line`):**
```
type=qr       → return Err("qr_required") with qr_data
type=messages → return Ok(Vec<Message>)
type=ready    → log only, no event emitted
type=error    → return Err(message)
```

**`sync_complete` event payload:** `messages.len()` — unchanged.

**Removed:** `--since` argument, `last_synced_at` setting, `compute_since()`, polling interval logic.

---

## `src-tauri/src/lib.rs`

- Replace `start_scheduler(...)` call with `start_baileys(...)`
- Remove `IntervalTx` state and `interval_tx` from `save_settings` command (no more interval setting needed at runtime)
- Keep `sync_interval_minutes` setting in DB for now (can be cleaned up later)

---

## `scripts/package.json`

```json
{
  "dependencies": {
    "@whiskeysockets/baileys": "^6.7.0",
    "qrcode": "^1.5.3"
  }
}
```

Remove: `playwright`, `playwright-extra`, `puppeteer-extra-plugin-stealth`

---

## `src-tauri/src/commands.rs`

- Remove `IntervalTx` import and `interval_tx: State<'_, IntervalTx>` from `save_settings`
- `sync_interval_minutes` setting still saved to DB (UI field stays, future use)

---

## Data Flow

```
WhatsApp servers
  → Baileys WebSocket
  → messages.upsert event
  → baileys.js formats + writes JSON line to stdout
  → Rust BufReader reads line
  → parse_baileys_line()
  → upsert_message() × N
  → emit("sync_complete", count)
  → Vue useAssistant updates bubbleText
```

---

## Out of Scope

- Sending messages
- Message history backfill on reconnect (Baileys delivers only new messages on `notify`)
- Removing `sync_interval_minutes` from settings UI (low priority)
- Graceful shutdown signal to baileys.js process
