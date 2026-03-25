# Floating Cat UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace the tabbed dark-background widget with a transparent floating cat mascot that lives directly on the desktop — no app chrome, hover controls, speech bubble, and click-to-ask input.

**Architecture:** The window becomes a fully transparent 320×320 fixed-size container. `WhatsAppAssistant.vue` is rewritten as the root UI (cat + bubble + input bar), with hover controls wired to `@tauri-apps/api/window`. `useAssistant.ts` gains bubble state management. `App.vue` is simplified to a transparent pass-through. The Rust settings backend gains one new string field.

**Tech Stack:** Vue 3 Composition API (script setup), Tauri 2 (`@tauri-apps/api/window`), lottie-web, TypeScript, Rust (existing SQLite settings pattern)

**Spec:** `docs/superpowers/specs/2026-03-25-cat-floating-ui-design.md`

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src-tauri/tauri.conf.json` | Modify | Window size 320×320 |
| `src-tauri/src/commands.rs` | Modify | Add `bubble_timeout_seconds` to SettingsPayload + get/save |
| `src/composables/useAssistant.ts` | Modify | Bubble state, timer, Settings interface |
| `src/components/SettingsPanel.vue` | Modify | Add "Tempo do balão" field |
| `src/components/CatMascot.vue` | Modify | Add click emit |
| `src/components/WhatsAppAssistant.vue` | Rewrite | Floating cat UI (bubble, input, hover controls, QR/setup preserved) |
| `src/App.vue` | Modify | Remove tabs, dark background, drag region |

---

## Task 1: Resize window to 320×320

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [x] **Step 1: Update window dimensions**

In `tauri.conf.json`, find the `"windows"` array entry and change:
```json
"width": 320,
"height": 320,
```
(was `"width": 350, "height": 500`)

- [x] **Step 2: Verify build compiles**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run build
```
Expected: build succeeds (no Tauri config errors)

- [x] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "chore: resize window to 320x320 for floating cat UI"
```

---

## Task 2: Add `bubble_timeout_seconds` to Rust settings

**Files:**
- Modify: `src-tauri/src/commands.rs` (lines 14–22 `SettingsPayload`, lines 92–124 `get_settings`/`save_settings`)

- [x] **Step 1: Add field to `SettingsPayload` struct**

In `commands.rs`, add to the `SettingsPayload` struct (after `ollama_model`):
```rust
pub bubble_timeout_seconds: String,
```

- [x] **Step 2: Return the field in `get_settings`**

In the `get_settings` command, add after the `ollama_model` line:
```rust
bubble_timeout_seconds: db::get_setting_or(&conn, "bubble_timeout_seconds", "10"),
```

- [x] **Step 3: Save the field in `save_settings`**

In the `save_settings` command, add after the `ollama_model` set call:
```rust
db::set_setting(&conn, "bubble_timeout_seconds", &payload.bubble_timeout_seconds)?;
```

- [x] **Step 4: Verify Rust compiles**

```bash
cd /Users/leominari/Documents/larab/focus-widget/src-tauri
cargo check
```
Expected: no errors

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add bubble_timeout_seconds to settings"
```

---

## Task 3: Add bubble timeout field to SettingsPanel.vue

**Files:**
- Modify: `src/components/SettingsPanel.vue` (lines 12–22 form defaults, lines 48–102 template)

- [x] **Step 1: Add to Settings type and form defaults**

In `useAssistant.ts`, add to the `Settings` interface (after `ollama_model`):
```ts
bubble_timeout_seconds: string
```

In `SettingsPanel.vue`, add to the `form` ref initial value (after `ollama_model: 'llama3'`):
```ts
bubble_timeout_seconds: '10',
```

- [x] **Step 2: Add the input field to the template**

In `SettingsPanel.vue` template, add before the error/save messages section (after the Ollama inputs block, around line 98):
```html
<div class="form-group">
  <label>Tempo do balão (segundos)</label>
  <input
    v-model="form.bubble_timeout_seconds"
    type="number"
    min="3"
    max="60"
    placeholder="10"
  />
</div>
```

- [x] **Step 3: Verify TypeScript**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run build
```
Expected: no TypeScript errors

- [x] **Step 4: Commit**

```bash
git add src/components/SettingsPanel.vue src/composables/useAssistant.ts
git commit -m "feat: add bubble timeout seconds to settings panel"
```

---

## Task 4: Add bubble state to `useAssistant.ts`

**Files:**
- Modify: `src/composables/useAssistant.ts`

This is the core state addition. We add three refs (`bubbleText`, `inputOpen`, `lastCatMessage`), update the `sync_complete` handler to set `bubbleText`, and add a timer system that respects the `inputOpen` state.

- [x] **Step 1: Add new refs**

After the existing ref definitions (around line 27), add:
```ts
const bubbleText = ref('')
const inputOpen = ref(false)
const lastCatMessage = ref('')
```

- [x] **Step 2: Add bubble timer helper**

After `setError()` (around line 37), add:
```ts
let bubbleTimer: ReturnType<typeof setTimeout> | null = null
let bubbleTimerPending = false

function startBubbleTimer(seconds: number) {
  if (inputOpen.value) {
    bubbleTimerPending = true
    return
  }
  bubbleTimer = setTimeout(() => {
    bubbleText.value = ''
    bubbleTimerPending = false
  }, seconds * 1000)
}

function clearBubbleTimer() {
  if (bubbleTimer !== null) {
    clearTimeout(bubbleTimer)
    bubbleTimer = null
  }
}
```

- [x] **Step 3: Update `sync_complete` handler**

Replace the existing `sync_complete` listener body (around line 64–67):
```ts
listen<number>('sync_complete', (e) => {
  catState.value = 'idle'
  const count = e.payload
  bubbleText.value = count > 0
    ? `${count} nova${count > 1 ? 's' : ''} mensagem${count > 1 ? 's' : ''} 📬`
    : 'Sync completo — sem novidades 👌'
  const timeout = parseInt(settings.value?.bubble_timeout_seconds ?? '10', 10) || 10
  startBubbleTimer(timeout)
  syncStatus.value = `Último sync: ${count} mensagens`
}).then(fn => unlisteners.push(fn))
```

Note: `settings` needs to be a ref accessible here. Add it before the listeners block:
```ts
const settings = ref<Settings | null>(null)
```
And in `loadSettings()`, update to also set this ref:
```ts
async function loadSettings() {
  const s = await invoke<Settings>('get_settings')
  settings.value = s
  return s
}
```

- [x] **Step 4: Wire `inputOpen` watcher for deferred timer**

After the listeners block (around line 96), add:
```ts
watch(inputOpen, (open) => {
  if (!open && bubbleTimerPending) {
    bubbleTimerPending = false
    const timeout = parseInt(settings.value?.bubble_timeout_seconds ?? '10', 10) || 10
    startBubbleTimer(timeout)
  }
})
```

- [x] **Step 5: Update `llm_done` handler to set `lastCatMessage`**

In the `llm_done` listener, after pushing the message and before resetting state, add:
```ts
lastCatMessage.value = currentResponse.value
```

- [x] **Step 6: Clean up timer in `onUnmounted`**

In the `onUnmounted` cleanup block, add:
```ts
clearBubbleTimer()
```

- [x] **Step 7: Export new refs**

Add `bubbleText`, `inputOpen`, `lastCatMessage` to the return object of `useAssistant()`.

- [x] **Step 8: TypeScript check**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run build
```
Expected: no errors

- [x] **Step 9: Commit**

```bash
git add src/composables/useAssistant.ts
git commit -m "feat: add bubble state management to useAssistant"
```

---

## Task 5: Add click emit to CatMascot.vue

**Files:**
- Modify: `src/components/CatMascot.vue`

- [x] **Step 1: Add `defineEmits`**

In `CatMascot.vue` script setup, add after the existing prop definitions:
```ts
const emit = defineEmits<{
  click: []
}>()
```

- [x] **Step 2: Wire click on root element**

In the template, update the `.cat-wrapper` div to forward click:
```html
<div class="cat-wrapper" @click="emit('click')">
```

- [x] **Step 3: TypeScript check**

```bash
npm run build
```
Expected: no errors

- [x] **Step 4: Commit**

```bash
git add src/components/CatMascot.vue
git commit -m "feat: add click emit to CatMascot"
```

---

## Task 6: Rewrite WhatsAppAssistant.vue

**Files:**
- Rewrite: `src/components/WhatsAppAssistant.vue`

This is the main UI task. The component becomes: SetupWizard gate → QR overlay → floating cat with optional bubble above + optional input bar below + hover controls.

**Hover controls:** visible only when `isHovered` is true. The move icon calls `window.startDragging()` on mousedown. Settings icon toggles `showSettings`. Close icon calls `window.close()`.

**Click outside / window blur:** uses `onFocusChanged` from `@tauri-apps/api/window`. When the window loses focus and `inputOpen` is true, set `inputOpen = false`.

- [x] **Step 1: Write the new component**

Replace the full content of `WhatsAppAssistant.vue` with:

```vue
<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import CatMascot from './CatMascot.vue'
import SettingsPanel from './SettingsPanel.vue'
import SetupWizard from './SetupWizard.vue'
import { useAssistant } from '../composables/useAssistant'

const {
  catState,
  bubbleText,
  inputOpen,
  lastCatMessage,
  isStreaming,
  qrData,
  sendQuestion,
  loadSettings,
  saveSettings,
  checkPrerequisites,
} = useAssistant()

const setupComplete = ref(false)
const showSettings = ref(false)
const isHovered = ref(false)
const inputText = ref('')

// Close input when window loses focus
let unlistenFocus: (() => void) | null = null
onMounted(async () => {
  const win = getCurrentWindow()
  unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
    if (!focused) {
      inputOpen.value = false
      showSettings.value = false
    }
  })
})
onUnmounted(() => {
  unlistenFocus?.()
})

function handleCatClick() {
  if (!setupComplete.value) return
  inputOpen.value = !inputOpen.value
  if (inputOpen.value && !bubbleText.value) {
    bubbleText.value = lastCatMessage.value || 'Oi! Pode perguntar! 😸'
  }
}

async function handleSubmit() {
  const q = inputText.value.trim()
  if (!q || isStreaming.value) return
  inputText.value = ''
  await sendQuestion(q)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    inputOpen.value = false
    showSettings.value = false
  }
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSubmit()
  }
}

async function handleMoveMousedown(e: MouseEvent) {
  e.preventDefault()
  const win = getCurrentWindow()
  await win.startDragging()
}

async function handleClose() {
  const win = getCurrentWindow()
  await win.close()
}
</script>

<template>
  <!-- Setup wizard gate -->
  <SetupWizard
    v-if="!setupComplete"
    :checkPrerequisites="checkPrerequisites"
    @ready="setupComplete = true"
  />

  <!-- QR overlay -->
  <div v-else-if="qrData" class="qr-overlay">
    <p>Escaneie o QR code no WhatsApp</p>
    <img :src="qrData" alt="QR Code" />
  </div>

  <!-- Main floating cat UI -->
  <div
    v-else
    class="cat-root"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
    @keydown="handleKeydown"
  >
    <!-- Hover controls -->
    <div class="hover-controls" :class="{ visible: isHovered }">
      <div
        class="ctrl-btn move-btn"
        title="Mover"
        @mousedown="handleMoveMousedown"
      >⠿</div>
      <div
        class="ctrl-btn settings-btn"
        title="Configurações"
        @click="showSettings = !showSettings"
      >⚙</div>
      <div
        class="ctrl-btn close-btn"
        title="Fechar"
        @click="handleClose"
      >✕</div>
    </div>

    <!-- Settings panel overlay -->
    <SettingsPanel
      v-if="showSettings"
      :loadSettings="loadSettings"
      :saveSettings="saveSettings"
      @close="showSettings = false"
    />

    <!-- Speech bubble -->
    <div v-if="bubbleText" class="speech-bubble">
      {{ bubbleText }}
    </div>

    <!-- Cat mascot -->
    <CatMascot :state="catState" @click="handleCatClick" />

    <!-- Input bar -->
    <div v-if="inputOpen" class="input-bar">
      <input
        v-model="inputText"
        type="text"
        placeholder="Digite sua pergunta..."
        :disabled="isStreaming"
        autofocus
        @keydown.stop="handleKeydown"
      />
      <button
        class="send-btn"
        :disabled="isStreaming || !inputText.trim()"
        @click="handleSubmit"
      >→</button>
    </div>
  </div>
</template>

<style scoped>
.cat-root {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  height: 100%;
  padding-top: 28px; /* space for hover controls */
}

/* Hover controls */
.hover-controls {
  position: absolute;
  top: 4px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 8px;
  opacity: 0;
  transition: opacity 200ms ease-out;
  pointer-events: none;
  z-index: 10;
}
.hover-controls.visible {
  opacity: 1;
  pointer-events: all;
}

.ctrl-btn {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.65);
  color: #ddd;
  font-size: 0.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  user-select: none;
  backdrop-filter: blur(4px);
}
.ctrl-btn:hover {
  background: rgba(0, 0, 0, 0.85);
  color: white;
}
.move-btn { cursor: grab; }
.move-btn:active { cursor: grabbing; }
.close-btn:hover { background: rgba(200, 40, 40, 0.8); }
.settings-btn:hover { background: rgba(37, 211, 102, 0.5); }

/* Speech bubble */
.speech-bubble {
  background: white;
  color: #222;
  border-radius: 12px;
  padding: 8px 12px;
  font-size: 0.75rem;
  max-width: 280px;
  line-height: 1.4;
  text-align: center;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.25);
  margin-bottom: 6px;
  position: relative;
  font-family: Inter, sans-serif;
}
.speech-bubble::after {
  content: '';
  position: absolute;
  bottom: -7px;
  left: 50%;
  transform: translateX(-50%);
  border: 5px solid transparent;
  border-top-color: white;
}

/* Input bar */
.input-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  background: rgba(0, 0, 0, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.25);
  border-radius: 20px;
  padding: 6px 10px;
  width: 290px;
  backdrop-filter: blur(4px);
}
.input-bar input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: #eee;
  font-size: 0.8rem;
}
.input-bar input::placeholder { color: #888; }
.input-bar input:disabled { color: #666; }

.send-btn {
  background: #25d366;
  border: none;
  border-radius: 50%;
  width: 22px;
  height: 22px;
  color: white;
  font-size: 0.8rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.send-btn:disabled {
  background: #444;
  cursor: default;
}

/* QR overlay */
.qr-overlay {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: white;
  font-size: 0.8rem;
  font-family: Inter, sans-serif;
}
.qr-overlay img {
  width: 200px;
  height: 200px;
  border-radius: 8px;
}
</style>
```

- [x] **Step 2: TypeScript check**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run build
```
Expected: no TypeScript errors. Fix any import path issues.

- [x] **Step 3: Commit**

```bash
git add src/components/WhatsAppAssistant.vue
git commit -m "feat: rewrite WhatsAppAssistant as transparent floating cat UI"
```

---

## Task 7: Simplify App.vue

**Files:**
- Modify: `src/App.vue`

Remove all app chrome: dark container, tab bar, centering, `data-tauri-drag-region`. The root element becomes a transparent pass-through that just mounts `WhatsAppAssistant`.

- [x] **Step 1: Replace App.vue content**

Replace the full content of `src/App.vue` with:

```vue
<script setup lang="ts">
import WhatsAppAssistant from './components/WhatsAppAssistant.vue'
</script>

<template>
  <WhatsAppAssistant />
</template>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body, #app {
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
}
</style>
```

- [x] **Step 2: TypeScript check**

```bash
npm run build
```
Expected: clean build

- [x] **Step 3: Commit**

```bash
git add src/App.vue
git commit -m "feat: simplify App.vue to transparent pass-through for floating cat"
```

---

## Task 8: Integration test — run the app

- [x] **Step 1: Start dev server**

```bash
cd /Users/leominari/Documents/larab/focus-widget
npm run tauri dev
```

- [x] **Step 2: Verify idle state**
  - Window appears as just the cat on transparent background
  - No dark box, no tabs, no background
  - Cat animation plays normally

- [x] **Step 3: Verify hover controls**
  - Move mouse over cat → three icons appear (⠿ ⚙ ✕) with fade-in
  - Move mouse away → icons fade out
  - Click ✕ → app closes
  - Click ⚙ → settings panel appears

- [x] **Step 4: Verify click to open input**
  - Click cat → input bar appears below, bubble shows greeting or last message
  - Press Esc → input closes, returns to idle
  - Click cat again → input reopens

- [x] **Step 5: Verify drag**
  - Hover → mousedown on ⠿ icon → window drags to new position

- [x] **Step 6: Verify sync bubble**
  - Wait for sync to complete → bubble appears with message count
  - Bubble disappears after ~10 seconds

- [x] **Step 7: Verify question → response**
  - Click cat → type question → press Enter
  - Bubble shows streaming response
  - Input is disabled during streaming
  - After response: `lastCatMessage` is set; next click shows the response in bubble

- [x] **Step 8: Verify settings → bubble timeout**
  - Hover → click ⚙ → settings opens
  - Change "Tempo do balão" to 5 → save
  - Trigger sync → bubble disappears after ~5 seconds

- [x] **Step 9: Commit if all good**

```bash
git add -A
git commit -m "chore: verify floating cat UI integration complete"
```
