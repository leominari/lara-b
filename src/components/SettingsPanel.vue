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
const loadError = ref(false)
const saveError = ref(false)

onMounted(async () => {
  try {
    form.value = await props.loadSettings()
  } catch {
    loadError.value = true
  }
})

const showApiKey = computed(() => ['claude', 'openai'].includes(form.value.llm_provider))
const showOllama = computed(() => form.value.llm_provider === 'ollama')

async function save() {
  saveError.value = false
  try {
    await props.saveSettings(form.value)
    saved.value = true
    setTimeout(() => { saved.value = false }, 2000)
  } catch {
    saveError.value = true
    setTimeout(() => { saveError.value = false }, 3000)
  }
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

    <p v-if="loadError" class="error-msg">Erro ao carregar configurações.</p>
    <button class="save-btn" @click="save">{{ saved ? '✓ Salvo!' : 'Salvar' }}</button>
    <p v-if="saveError" class="error-msg">Erro ao salvar. Tente novamente.</p>
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
.error-msg { font-size: 0.75rem; color: #ff5050; margin: 0; }
</style>
