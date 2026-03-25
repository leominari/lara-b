<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import CatMascot from './CatMascot.vue'
import SettingsPanel from './SettingsPanel.vue'
import SetupWizard from './SetupWizard.vue'
import { useAssistant } from '../composables/useAssistant'

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
  if (!q || isStreaming.value) return
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
        <img :src="qrData" width="200" height="200" alt="WhatsApp QR Code" />
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
