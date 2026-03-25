# Baileys Migration Design Spec

**Date:** 2026-03-25
**Status:** Approved (v2 — post spec-review)

## Overview

Replace the Playwright-based `sync.js` (spawn-per-run, DOM scraping) with a persistent Baileys process that connects directly to WhatsApp's WebSocket protocol and streams new messages to Rust in real time.

---

## Architecture

```
App launch
  → Rust spawns baileys.js (persistent, stdout + stderr piped)
  → Rust reads stdout line by line (BufReader loop)
  → On QR line      → emit "qr_required" to frontend
  → On ready line   → emit "sync_start" to frontend
  → On messages     → upsert to SQLite, emit "sync_complete"
  → On error line   → emit "sync_error"
  → On process exit → wait 5s, restart (unless loggedOut/badSession)
```

---

## Script: `scripts/baileys.js`

### Stdout protocol — JSON lines

```json
{"type":"qr","qr_data":"data:image/png;base64,..."}
{"type":"ready"}
{"type":"messages","messages":[...]}
{"type":"error","message":"..."}
{"type":"logout"}
```

### QR generation

Baileys emits QR as a **string** (raw QR text) via `connection.update`:

```js
sock.ev.on('connection.update', ({ qr }) => {
  if (qr) {
    qrcode.toDataURL(qr, (err, url) => {
      if (!err) output({ type: 'qr', qr_data: url })
    })
  }
})
```

Use the `qrcode` npm package. `qr` is a string — do NOT treat it as a Buffer.

### DisconnectReason handling

```js
const { DisconnectReason } = require('@whiskeysockets/baileys')

// In connection.update handler:
if (lastDisconnect?.error?.output?.statusCode === DisconnectReason.loggedOut) {
  output({ type: 'logout' })
  process.exit(0)           // Rust will NOT restart on clean exit 0
}
if (lastDisconnect?.error?.output?.statusCode === DisconnectReason.badSession) {
  // Delete corrupted auth and exit — Rust restarts, fresh QR will appear
  fs.rmSync(AUTH_DIR, { recursive: true, force: true })
  output({ type: 'error', message: 'bad_session' })
  process.exit(1)
}
// All other reasons (restartRequired, connectionClosed, timedOut, etc.)
// → reconnect internally by calling connectToWhatsApp() again
```

Rust distinguishes clean exit (code 0 = logged out, no restart) vs crash (code ≠ 0 = restart after 5s).

### Message shape

Baileys `messages.upsert` provides `msg.key.id` (globally unique per message) — use it directly as the message ID instead of computing a SHA-256 hash.

For display name:
- Direct chats: `msg.pushName` (sender's push name) or JID local part
- Group chats: `msg.key.participant` → strip `@s.whatsapp.net` suffix for display

```js
sock.ev.on('messages.upsert', ({ messages, type }) => {
  if (type !== 'notify') return  // skip history loads
  const formatted = messages
    .filter(msg => !msg.key.fromMe && msg.message)
    .map(msg => {
      const isGroup = msg.key.remoteJid.endsWith('@g.us')
      const senderJid = isGroup ? msg.key.participant : msg.key.remoteJid
      const contact = msg.pushName || senderJid.split('@')[0]
      const chat = isGroup
        ? (groupMetadataCache[msg.key.remoteJid] || msg.key.remoteJid.split('@')[0])
        : contact
      const body = msg.message.conversation
        || msg.message.extendedTextMessage?.text
        || ''
      if (!body) return null
      return {
        id: msg.key.id,
        contact,
        chat,
        body,
        timestamp: msg.messageTimestamp,
        is_mine: false,
      }
    })
    .filter(Boolean)
  if (formatted.length > 0) {
    output({ type: 'messages', messages: formatted })
  }
})
```

For group chat names: maintain a simple in-memory cache `groupMetadataCache`. On first message from a group JID, call `sock.groupMetadata(jid)` to get the subject (group name).

### Auth persistence

Auth stored in `~/.whatsapp-assistant/baileys-auth/` via Baileys `useMultiFileAuthState`.

If the auth directory is missing or empty (fresh install or after `badSession` deletion), `useMultiFileAuthState` creates it and returns empty credentials — Baileys will then emit a QR event normally. No special handling needed; the QR flow is the default when credentials are absent.

### `--check-login-only` flag removal

`baileys.js` does **not** support `--check-login-only`. The `check_qr_status` Tauri command must be removed (see Rust section).

---

## Rust: `src-tauri/src/sync.rs`

### Return type for line parsing

Replace the current `parse_sync_output(stdout: &str) -> Result<Vec<Message>, String>` with:

```rust
pub enum BaileysLine {
    Messages(Vec<Message>),
    Qr(String),      // qr_data string
    Ready,
    Logout,
    Error(String),
}

pub fn parse_baileys_line(line: &str) -> Result<BaileysLine, String>
```

No more string-matching `== "qr_required"`. Each variant is handled cleanly in the streaming loop.

### `start_baileys` function

Replaces `start_scheduler`. No `interval_rx` parameter.

```rust
pub fn start_baileys(
    app: AppHandle,
    db_path: std::path::PathBuf,
    script_path: std::path::PathBuf,  // points to baileys.js
)
```

`sync_in_progress` is removed — it guarded against overlapping periodic runs, which no longer exist with a persistent process.

Internal loop:
1. Spawn `node baileys.js` with `stdout: Stdio::piped()`, `stderr: Stdio::piped()`
2. Spawn a separate task to forward stderr lines to `eprintln!` for diagnostics
3. Read stdout lines via `tokio::io::BufReader::lines()`
4. Match on `parse_baileys_line`:
   - `Qr(data)` → `app.emit("qr_required", data)`
   - `Ready` → `app.emit("sync_start", ())`
   - `Messages(msgs)` → upsert each + `app.emit("sync_complete", msgs.len())`
   - `Logout` → `app.emit("sync_error", "Sessão encerrada")`, stop (no restart)
   - `Error(e)` → `app.emit("sync_error", e)`
5. On process exit code 0 (Logout): do not restart
6. On process exit code ≠ 0: wait 5s, restart

---

## `src-tauri/src/commands.rs`

- **Remove** `check_qr_status` command entirely (QR flow now handled via persistent connection)
- **Remove** `IntervalTx` state and `interval_tx` param from `save_settings`
- Keep `sync_interval_minutes` DB setting (UI stays, no runtime effect for now)

---

## `src-tauri/src/lib.rs`

- Replace `start_scheduler(...)` with `start_baileys(...)`
- Remove `IntervalTx` state registration
- Update `script_path` strings from `sync.js` → `baileys.js` in both dev and release path resolution
- Remove `check_qr_status` from `tauri::generate_handler![...]`

---

## `src/components/SetupWizard.vue`

- Remove the `playwright` check from `check_prerequisites`
- `check_prerequisites` now only checks `node` — response shape: `{ node: boolean }`
- Update UI to remove the Playwright row

---

## `scripts/package.json`

```json
{
  "name": "whatsapp-sync",
  "version": "2.0.0",
  "type": "commonjs",
  "dependencies": {
    "@whiskeysockets/baileys": "^6.7.0",
    "qrcode": "^1.5.3"
  }
}
```

Remove: `playwright`, `playwright-extra`, `puppeteer-extra-plugin-stealth`

---

## Data Flow

```
WhatsApp servers
  → Baileys WebSocket
  → messages.upsert (type=notify)
  → baileys.js formats + writes JSON line to stdout
  → Rust BufReader reads line
  → parse_baileys_line() → BaileysLine::Messages(msgs)
  → upsert_message() × N
  → emit("sync_complete", count)
  → Vue: bubbleText = "N novas mensagens 📬"
```

---

## Out of Scope

- Sending messages
- Message history backfill (Baileys delivers only new messages on `notify`)
- Removing `sync_interval_minutes` from settings UI
- Graceful SIGTERM to baileys.js on app quit
