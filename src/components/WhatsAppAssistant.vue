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
const inputEl = ref<HTMLInputElement | null>(null)

watch(inputOpen, async (open) => {
  if (open) {
    await nextTick()
    inputEl.value?.focus()
  }
})

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

    <!-- Cat mascot + bubble anchored above it -->
    <div class="cat-zone">
      <div v-if="bubbleText" class="speech-bubble">{{ bubbleText }}</div>
      <CatMascot :state="catState" @click="handleCatClick" />
    </div>

    <!-- Input bar -->
    <div class="input-bar" :class="{ open: inputOpen }">
      <input
        ref="inputEl"
        v-model="inputText"
        type="text"
        placeholder="Digite sua pergunta..."
        :disabled="isStreaming"
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
  justify-content: flex-end;
  width: 100%;
  height: 100%;
  padding: 28px 0 8px;
}

/* Wrapper que mantém o gato fixo e ancora o balão acima dele */
.cat-zone {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
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

/* Speech bubble — posicionado absolutamente acima do gato, cresce para cima */
.speech-bubble {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-bottom: 6px;
  background: white;
  color: #222;
  border-radius: 12px;
  padding: 8px 12px;
  font-size: 0.75rem;
  width: 280px;
  line-height: 1.4;
  text-align: center;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.25);
  font-family: Inter, sans-serif;
  word-break: break-word;
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
  opacity: 0;
  transform: translateY(-8px) scale(0.97);
  pointer-events: none;
  transition: opacity 180ms ease-out, transform 180ms ease-out;
}
.input-bar.open {
  opacity: 1;
  transform: translateY(0) scale(1);
  pointer-events: all;
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
