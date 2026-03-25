# WhatsApp Personal Assistant — Design Spec
**Date:** 2026-03-24
**Project:** focus-widget (Tauri + Vue + TypeScript)

---

## Overview

A personal assistant integrated into the `focus-widget` Tauri desktop app that reads WhatsApp messages via WhatsApp Web automation and allows the user to ask free-form natural language questions about their conversations.

---

## Goals

- Automatically read and store incoming WhatsApp messages without user intervention
- Allow the user to ask natural language questions like "tem algo urgente hoje?" or "o que o João falou sobre o projeto?"
- Display responses through a friendly cat mascot UI
- Support multiple LLM providers (Claude, OpenAI, Ollama) — switchable via settings

---

## Non-Goals

- Sending or replying to WhatsApp messages
- Real-time message push notifications
- WhatsApp Business API integration
- Multi-device support (single WhatsApp account only)

---

## Architecture

```
┌─────────────────────────────────────────┐
│  focus-widget (Tauri)                   │
│                                         │
│  ┌─────────┐    ┌──────────────────┐   │
│  │ Vue UI  │◄──►│  Rust (Tauri)    │   │
│  │ (chat)  │    │  - SQLite        │   │
│  └─────────┘    │  - LLM API calls │   │
│                 │  - message filter│   │
│                 └────────┬─────────┘   │
└──────────────────────────┼─────────────┘
                           │ spawn via shell
                    ┌──────▼──────┐
                    │  Node.js    │
                    │  script     │
                    │ (Playwright)│
                    └──────┬──────┘
                           │ reads
                    ┌──────▼──────┐
                    │ WhatsApp    │
                    │    Web      │
                    └─────────────┘
```

### Two main flows

1. **Sync flow** — Tauri timer fires → spawns Node.js script via shell command → script reads new messages from WhatsApp Web → outputs JSON to stdout → Tauri parses and upserts into SQLite
2. **Query flow** — User types question → Rust filters SQLite → builds LLM prompt → calls LLM API → streams response tokens to Vue UI via Tauri events

---

## Components

### 1. Playwright Script (Node.js — system runtime)

**Important:** Playwright cannot be bundled as a self-contained Tauri sidecar binary because it requires a local Node.js runtime and a separately installed Chromium browser. Instead, the script runs via the system Node.js installation:

- **Prerequisite**: user must have Node.js installed (`node` on PATH) and run `npm install` + `npx playwright install chromium` once during setup
- Tauri spawns the script via `Command::new("node").args(["path/to/sync.js", ...])` using Tauri's shell API
- A setup wizard in the UI guides the user through first-time prerequisites
- The Chromium browser profile is persisted at `~/.whatsapp-assistant/profile/` so the WhatsApp Web session survives restarts

**On each sync:**
1. Launch Playwright with the persisted Chromium profile (headless)
2. Navigate to `https://web.whatsapp.com`
3. If QR code is detected → output `{"status": "qr_required", "qr_data": "<qr_string>"}` to stdout and exit
4. If logged in → scrape all chats, collect messages newer than the provided `--since` timestamp argument
5. Output result JSON (see Sidecar Output Contract below) to stdout and exit

### 2. Sidecar Output Contract (JSON)

Every script invocation outputs exactly one JSON object to stdout:

**Success:**
```json
{
  "status": "ok",
  "messages": [
    {
      "id": "sha256hex",
      "contact": "João Silva",
      "chat": "João Silva",
      "body": "Oi, tudo bem?",
      "timestamp": 1711234567,
      "is_mine": false
    }
  ]
}
```

**QR required:**
```json
{
  "status": "qr_required",
  "qr_data": "<base64 or URL string for QR rendering>"
}
```

**Error:**
```json
{
  "status": "error",
  "message": "human-readable error description"
}
```

Field definitions:
- `id`: SHA-256 hex of `contact + "|" + timestamp.toString() + "|" + body`
- `contact`: sender's display name (or phone number if no name)
- `chat`: group chat name, or same as `contact` for individual chats
- `timestamp`: Unix timestamp in seconds (integer)
- `is_mine`: `true` if the message was sent by the authenticated user

### 3. SQLite Database (Rust — rusqlite)

Schema:

```sql
CREATE TABLE messages (
  id        TEXT PRIMARY KEY,   -- SHA-256 hex of (contact|timestamp|body)
  contact   TEXT NOT NULL,      -- sender display name or phone number
  chat      TEXT NOT NULL,      -- group name or individual chat name
  body      TEXT NOT NULL,      -- message content
  timestamp INTEGER NOT NULL,   -- unix timestamp (seconds)
  is_mine   INTEGER NOT NULL    -- 1 = sent by user, 0 = received
);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

**Settings keys:**
- `sync_interval_minutes` — integer string ("1", "5", "15", "30"); default "5"
- `llm_provider` — "claude" | "openai" | "ollama"
- `llm_api_key` — API key for Claude or OpenAI (see Security note below)
- `ollama_base_url` — default "http://localhost:11434"
- `ollama_model` — default "llama3"
- `last_synced_at` — unix timestamp string of last successful sync
- `initial_lookback_days` — how many days to fetch on first sync; default "7"

**Security note:** API keys are stored in plaintext SQLite on disk. This is acceptable for v1 given the personal/local nature of the app, but the implementer should note this limitation in code comments and consider migrating to macOS Keychain via `tauri-plugin-keychain` in a future version.

### 4. Sync Scheduler (Rust)

- Tauri background timer using `tokio::time::interval`
- Interval reloaded from `settings` table when the user saves a new value
- **Concurrency guard**: a `AtomicBool` flag (`sync_in_progress`) is set to `true` when a sync starts and `false` when it ends. If the timer fires while `sync_in_progress` is `true`, the tick is skipped silently.

**First sync behavior:** When `last_synced_at` is absent from settings, the sidecar is called with `--since` set to `(now - initial_lookback_days * 86400)`. Default lookback is 7 days. The user can configure this in the setup wizard before first sync.

**Session expiry / QR code flow:** When the sidecar returns `status: "qr_required"`:
1. Tauri emits a `qr_required` event to the Vue UI with the `qr_data` payload
2. Vue renders the QR code using a `qrcode` library (e.g. `qrcode-vue3`)
3. Vue polls (every 3 seconds, max 5 minutes) via a Tauri command `check_qr_status`
4. `check_qr_status` spawns `node sync.js --check-login-only` (a flag that makes the script open WhatsApp Web and return immediately with login status, without scraping messages). It does NOT set `sync_in_progress` — the QR poll is independent of the sync guard. Returns `{ "logged_in": true }` or `{ "logged_in": false }`.
5. Once Vue receives `{ "logged_in": true }`, it dismisses the QR overlay and triggers an immediate sync
6. If 5 minutes elapse with no successful login, polling stops and UI shows "Scan cancelado. Tente novamente." and returns to idle state

### 5. Query Engine (Rust)

On each user query:
1. Fetch the 200 most recent messages from SQLite ordered by `timestamp DESC`
   - Note: 200 messages is a rough heuristic. Each message averages ~50 tokens; 200 messages ≈ 10k tokens, safely within most models' context windows. The implementer must monitor prompt size and reduce the limit if needed per provider.
2. Build the prompt, including both received and sent messages for conversation context:
   - SQLite query returns `ORDER BY timestamp DESC` (newest first), but the array is **reversed** before building the prompt so messages appear oldest-first in the prompt (more natural for LLM conversation context)
   ```
   Você é um assistente pessoal. Analise as mensagens abaixo e responda à pergunta do usuário.
   Mensagens recentes (mais novas por último):

   [{timestamp_iso}] {contact} {"(você)" if is_mine else ""}: {body}
   ...

   Pergunta: {user_question}
   ```
3. Call configured LLM provider (see Provider Abstraction below)
4. Stream response back to Vue UI

### 6. LLM Provider Abstraction (Rust)

Trait:
```rust
trait LlmProvider {
    async fn complete_stream(&self, prompt: String) -> impl Stream<Item = String>;
}
```

Implementations:
- **Claude**: POST to `https://api.anthropic.com/v1/messages`, model `claude-3-5-sonnet-20241022`, streaming
- **OpenAI**: POST to `https://api.openai.com/v1/chat/completions`, model `gpt-4o`, streaming
- **Ollama**: POST to `{ollama_base_url}/api/generate`, model from settings. If Ollama is unreachable (connection refused), return a friendly error: "Ollama não está rodando. Abra o Ollama e tente novamente."

**LLM Streaming Tauri Events:**

Rust emits to the Vue window during streaming:
- Event `llm_token`: payload `{ "token": "<string>" }` — one per token received
- Event `llm_done`: payload `{ "success": true }` on completion
- Event `llm_error`: payload `{ "message": "<error string>" }` on failure

Vue listens with `listen("llm_token", ...)`, appending each token to the displayed response.

### 7. Vue UI — WhatsApp Assistant Panel

New tab added to the existing `focus-widget` interface.

**Window size:** The current window is 350×500px. The Lottie animation (originally 280×200px) will be scaled down to **140×100px** to fit alongside the chat UI. The panel layout:

```
┌────────────────────────────────────────┐  ← 350px wide
│ [Cat 140×100]  🟢 sync: 2min ago       │  ← ~100px tall
├────────────────────────────────────────┤
│                                        │
│  Assistente: Olá! Pergunte sobre       │  ← scrollable chat
│  suas mensagens do WhatsApp.           │    history (~270px)
│                                        │
│  Você: tem algo urgente hoje?          │
│                                        │
│  Assistente: Sim! O Carlos pediu...    │
│                                        │
├────────────────────────────────────────┤
│  Digite sua pergunta...          [→]   │  ← ~50px input bar
└────────────────────────────────────────┘
```

Total: ~420px height — slightly over 500px if both animation and history are full. The chat history area uses `overflow-y: scroll` and the animation area compresses gracefully.

---

## Cat Mascot — Lottie Animation

**File:** `Loader cat.json` (25fps, 32 frames, original 280×200px — displayed at 140×100px)
**Library:** `lottie-web` with SVG renderer

The Lottie file has minimal animation (only the two ear layers animate via rotation, frames 4–11). All other body parts are static. State changes are achieved via Lottie playback speed control and CSS applied to the container element.

Animation states:

| State | Trigger | Lottie speed | CSS |
|-------|---------|-------------|-----|
| **idle** | No activity | `1.0` (normal loop) | none |
| **syncing** | Fetching messages | `3.0` (fast loop) | none |
| **thinking** | Awaiting LLM response | `0.3` (very slow loop) | `::after` pseudo-element showing animated "..." dots |
| **responding** | Streaming tokens | `2.0` | none |
| **error** | Sync or LLM failure | `1.0` (paused via `lottie.pause()`) | `filter: hue-rotate(300deg) saturate(2)` (red tint) |

Note: "thinking" (slow loop) and "error" (paused + red tint) are visually distinct — the ear animation in "thinking" is still visible (just slow), while "error" is fully static with a color shift.

State is controlled by a `catState` reactive variable in the Vue component, driving a computed `lottiSpeed` and a CSS class on the wrapper `<div>`.

---

## Settings Panel

Settings panel is a secondary view (not a modal) within the assistant tab. Changes take effect on **explicit "Salvar" button click** only. Saving:
- Writes updated values to the `settings` SQLite table
- If `sync_interval_minutes` changed: cancels the current timer and creates a new one with the new interval
- If LLM provider or API key changed: the new provider is used from the next query onward

Configurable options:
| Setting | Type | Default |
|---------|------|---------|
| Sync interval | Select (1/5/15/30 min) | 5 min |
| Initial lookback | Select (1/7/30 days) | 7 days |
| LLM provider | Select (Claude/OpenAI/Ollama) | Claude |
| API key | Password input — shown only when provider is Claude or OpenAI | — |
| Ollama base URL | Text input — shown only when provider is Ollama | http://localhost:11434 |
| Ollama model | Text input — shown only when provider is Ollama | llama3 |

---

## Data Flow — Sync

```
Timer fires (every N minutes)
  → Check sync_in_progress flag → skip if true
  → Set sync_in_progress = true
  → Emit sync_start event → Vue sets catState = "syncing"
  → Spawn: node sync.js --since {last_synced_at}  (if last_synced_at is absent, compute as now - initial_lookback_days * 86400; script must treat missing/zero/invalid --since as a hard error and output status:"error")
  → Read stdout JSON
  → If status == "qr_required": emit qr_required event to UI, set sync_in_progress = false, Vue sets catState = "idle", return
  → If status == "error": log, set sync_in_progress = false, emit sync_error to Vue → catState = "error" (resets to "idle" after 3s), return
  → Upsert messages into SQLite (INSERT OR IGNORE by primary key)
  → Update last_synced_at = now
  → Set sync_in_progress = false
  → Emit sync_complete event to Vue UI → Vue sets catState = "idle"
```

## Data Flow — Query

```
User submits question
  → Vue calls ask_question Tauri command
  → Rust sets cat state → "thinking"
  → Fetch 200 most recent messages from SQLite
  → Build prompt string (received + sent messages, labeled)
  → Call LLM provider.complete_stream(prompt)
  → On each token: emit llm_token event → Vue appends to response
  → Cat state → "responding"
  → On completion: emit llm_done event → cat state → "idle"
  → On error: emit llm_error event → cat state → "error"
```

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| WhatsApp Web session expired | Sidecar outputs `qr_required` → UI renders QR code via `qrcode-vue3` → polling resumes sync after scan |
| Node.js not installed | Setup check on app start → UI shows install instructions |
| Playwright/Chromium not installed | Setup check on app start → UI shows `npx playwright install chromium` instruction |
| LLM API key missing | Query returns inline error: "Configure sua API key nas configurações." |
| LLM API key invalid (401) | Query returns inline error: "API key inválida. Verifique nas configurações." |
| LLM API timeout (>30s) | Retry once after 5s; on second failure, return error message in chat |
| Ollama not running | Connection refused → inline error: "Ollama não está rodando." |
| SQLite write failure | Log error, skip sync cycle, retry next interval |
| Prompt exceeds context limit | Reduce message limit by 50 and retry once; if still failing, return error |
| No messages found for query | LLM prompt includes note: "Nenhuma mensagem encontrada no período solicitado." |

---

## Setup Wizard (First Run)

On first launch of the assistant panel, if prerequisites are not met, a setup wizard guides the user through:

1. Check if `node` is on PATH → show install link if not
2. Check if Playwright Chromium is installed via `npx playwright --version` (not `playwright` directly, since the binary lives in `node_modules/.bin/` after local install) → show `npx playwright install chromium` if not
3. Configure LLM provider and API key
4. Configure initial lookback period
5. Trigger first sync → show QR code for WhatsApp Web login if needed

---

## Out of Scope (v1)

- Message search with vector embeddings
- Push notifications for new important messages
- Voice input/output
- Message export or sharing
- Multiple WhatsApp accounts
- macOS Keychain integration for API keys (v2 improvement)
