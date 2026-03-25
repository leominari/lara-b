# Floating Cat UI — Design Spec

**Date:** 2026-03-25
**Status:** Approved

## Overview

Replace the current tabbed widget (dark background, app chrome) with a transparent floating cat mascot that lives directly on the desktop. The window has no background, no decorations — only the Lottie cat animation and its contextual UI elements are visible.

---

## Visual States

### 1. Idle
- Only the Lottie cat animation is visible on a fully transparent background.
- No input, no bubble, no controls.

### 2. After Sync (bubble visible)
- A speech bubble appears above the cat with a summary of new messages (e.g. "3 msgs novas de João 👀").
- Bubble disappears automatically after **10 seconds** (default, configurable in settings).
- If `sync_complete` fires while the input is already open, the bubble text updates to the sync summary but the input bar stays open. The auto-dismiss timer does **not** start while the input is open — it begins only after `inputOpen` returns to false.

### 3. Input Open (clicked cat)
- The bubble shows the **last message the cat sent** (`lastCatMessage`), or `"Oi! Pode perguntar!"` if no prior message exists in this session.
- `lastCatMessage` is ephemeral (in-memory only, resets on app restart).
- An input bar appears **below the cat**.
- Input bar has a text field ("Digite...") and a send button (→).
- Bubble stays visible while the input is open.
- Pressing **Esc** or the window losing focus closes the input and returns to idle. Window focus change (`onFocusChanged` from `@tauri-apps/api/window`) is used to detect "click outside" since the window is transparent and decoration-free.

### 4. Responding (streaming)
- The bubble content updates in real-time as the Claude response streams in.
- The input field is **disabled** (not accepting new submissions) while streaming.
- After the response completes, `lastCatMessage` is updated to the full response text and the bubble persists.

### 5. Hover
- Three controls appear at the top of the window with a **200ms ease-out fade-in**:
  - `⠿` **Move icon** (left) — the **only drag region**. Uses `window.startDragging()` from `@tauri-apps/api/window` called programmatically on `mousedown` of this icon (not via `data-tauri-drag-region` attribute, which is removed from the root element).
  - `⚙` **Settings icon** (center) — opens the settings panel overlay.
  - `✕` **Close icon** (right) — calls `window.close()` to quit the app.
- Controls fade out (200ms ease-out) when the mouse leaves the window.
- Clicking the settings icon does **not** close the input bar if it is open.

---

## Behavior Table

| Action | Result |
|---|---|
| Sync completes | Bubble appears with message summary. Dismisses after N seconds (default 10). |
| Sync fires while input open | Bubble text updates to sync summary. Timer deferred until input closes. |
| Click cat | Bubble shows `lastCatMessage` (or greeting). Input bar opens below cat. |
| Click outside / Esc / window blur | Input closes. Returns to idle. |
| Submit question while sync bubble showing | Sync summary is overwritten by the streaming response. This is intentional — the user chose to ask a question. |
| Submit question | Streaming starts. Bubble updates token by token. Input is disabled. |
| Response complete | `lastCatMessage` updated. Bubble persists. Input re-enabled. |
| Hover window | Three icons fade in: move (⠿), settings (⚙), close (✕). |
| Mousedown on move icon (⠿) | `window.startDragging()` — window repositions. |
| Click settings icon (⚙) | Settings panel opens as overlay. |
| Click close icon (✕) | `window.close()`. |

---

## Window Configuration

- **Size:** `width: 320, height: 320` (exact values for `tauri.conf.json`)
  - Height accounts for: bubble (~60px) + cat (200px) + input bar (~44px) + padding (~16px)
  - The window size is **fixed** and does not resize between states. Empty areas are transparent and invisible to the user.
- **Transparent:** `true`
- **Decorations:** `false`
- **Always on top:** `true`
- **Drag region:** Only the move icon via programmatic `startDragging()`. Remove `data-tauri-drag-region` from root element.
- **Initial position:** Bottom-right of primary monitor

---

## QR Code and Setup Wizard

The `SetupWizard` and QR overlay are **preserved** but displayed differently:
- `setupComplete` is a `ref<boolean>` driven by the `SetupWizard` component's `@ready` emit (same as today). When `setupComplete` is false, `WhatsAppAssistant.vue` renders the `SetupWizard` overlay instead of the cat.
- If `qrRequired` (boolean ref set on `qr_required` event from Rust), the QR overlay appears as a full-window overlay centered over the cat area, same as today.
- Both flows are kept in `WhatsAppAssistant.vue` during the rewrite.

---

## Settings Panel

Opened via the ⚙ hover icon. Appears as a translucent overlay over the cat. Closed by pressing Esc or window losing focus.

Includes all existing fields plus:
- **New:** "Tempo do balão (segundos)" — integer input, default `10`, min `3`, max `60`

The value is stored as `bubble_timeout_seconds: string` in the TypeScript `Settings` interface (consistent with the existing string-based settings pattern — all fields come from the Rust backend as strings). Parsed to integer at point of use with `parseInt(settings.bubble_timeout_seconds, 10) || 10`. Persisted via `invoke('save_settings')`.

---

## `sync_complete` Payload and Bubble Text

The Rust backend emits `sync_complete` with an integer payload (number of new messages). The composable formats this into a human-readable string:

```ts
// e.payload is a number
const count = e.payload as number
bubbleText.value = count > 0
  ? `${count} nova${count > 1 ? 's' : ''} mensagem${count > 1 ? 's' : ''} 📬`
  : 'Sync completo — sem novidades'
```

The payload is confirmed as an integer (`messages.len()` in `sync.rs` line 137). No other format to handle.

---

## Components and Code Changes

| File | Change |
|---|---|
| `App.vue` | Remove dark container, tab bar, and centering. Remove `data-tauri-drag-region` from root. Window root becomes fully transparent. Settings panel triggered by hover ⚙ icon. |
| `WhatsAppAssistant.vue` | Rewrite: Lottie cat + conditional bubble + conditional input bar + QR overlay (preserved) + SetupWizard gate (preserved). Remove scrollable chat history. Add `onFocusChanged` listener for "click outside" detection. |
| `CatMascot.vue` | Add `defineEmits(['click'])`. Wire root div `@click` to emit. Size unchanged (280×200px). |
| `useAssistant.ts` | Add `bubbleText`, `inputOpen`, `lastCatMessage` refs. Add bubble auto-show logic on `sync_complete`. Add bubble dismiss timer (deferred while input open). Add `bubble_timeout_seconds: string` to `Settings` interface (default `"10"`). |
| `SettingsPanel.vue` | Add "Tempo do balão (segundos)" field. Wire to `settings.bubble_timeout_seconds`. |
| `tauri.conf.json` | Change window size to `width: 320, height: 320`. Verify `transparent: true`, `decorations: false`, `alwaysOnTop: true`. Remove any `data-tauri-drag-region` that may be set at the Tauri config level. |
| Rust — `Settings` struct + `SettingsPayload` | Add `bubble_timeout_seconds` as a string key in the SQLite key-value settings table (same as all other settings). Update `get_settings` and `save_settings` commands in `commands.rs` to read/write this key. No struct field type change needed — consistent with the existing string-based pattern. |

---

## Data Flow

```
sync_complete event (payload: number)
  → format bubble text
  → bubbleText = formatted string
  → if inputOpen: defer timer
  → else: start N-second dismiss timer → bubbleText = ""

user clicks cat
  → inputOpen = true
  → stop/defer any active bubble timer
  → bubbleText = lastCatMessage || "Oi! Pode perguntar!"

user submits question
  → isStreaming = true, input disabled
  → bubbleText updates token by token
  → on complete: lastCatMessage = final bubbleText, isStreaming = false

user presses Esc / window loses focus
  → inputOpen = false
  → if deferred timer pending: start it now
  → else: clear bubbleText
```

---

## Out of Scope

- Chat history panel (removed — bubble shows one message at a time)
- Tabs (Tarefas / WhatsApp — removed entirely)
- Persisting `lastCatMessage` across app restarts (ephemeral is acceptable)
