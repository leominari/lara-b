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

  onUnmounted(() => {
    unlisteners.forEach(u => u())
    if (qrPollInterval) clearInterval(qrPollInterval)
    if (qrPollTimeout) clearTimeout(qrPollTimeout)
  })

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

  return {
    catState,
    messages,
    currentResponse,
    syncStatus,
    qrData,
    isStreaming,
    sendQuestion,
    loadSettings,
    saveSettings,
    checkPrerequisites,
  }
}
