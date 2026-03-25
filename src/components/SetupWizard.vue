<script setup lang="ts">
import { ref, onMounted } from 'vue'

const props = defineProps<{
  checkPrerequisites: () => Promise<{ node: boolean; playwright: boolean }>
}>()
const emit = defineEmits<{ ready: [] }>()

const nodeOk = ref(false)
const playwrightOk = ref(false)
const checking = ref(true)

onMounted(async () => {
  const result = await props.checkPrerequisites()
  nodeOk.value = result.node
  playwrightOk.value = result.playwright
  checking.value = false
  if (nodeOk.value && playwrightOk.value) {
    emit('ready')
  }
})
</script>

<template>
  <div class="wizard">
    <div v-if="checking">Verificando pré-requisitos...</div>
    <template v-else>
      <p class="wizard-title">Configuração inicial</p>
      <div class="check" :class="nodeOk ? 'ok' : 'fail'">
        {{ nodeOk ? '✓' : '✗' }} Node.js
        <span v-if="!nodeOk" class="hint">Instale em nodejs.org</span>
      </div>
      <div class="check" :class="playwrightOk ? 'ok' : 'fail'">
        {{ playwrightOk ? '✓' : '✗' }} Playwright Chromium
        <span v-if="!playwrightOk" class="hint">
          Execute no terminal:<br>
          <code>cd scripts && npm install && npx playwright install chromium</code>
        </span>
      </div>
      <button v-if="nodeOk && playwrightOk" class="ready-btn" @click="emit('ready')">
        Continuar →
      </button>
    </template>
  </div>
</template>

<style scoped>
.wizard { padding: 16px; display: flex; flex-direction: column; gap: 12px; }
.wizard-title { font-weight: 500; margin: 0; }
.check { padding: 8px; border-radius: 6px; font-size: 0.85rem; }
.ok { background: rgba(37,211,102,0.15); color: #25d366; }
.fail { background: rgba(255,80,80,0.15); color: #ff5050; }
.hint { display: block; font-size: 0.75rem; color: #aaa; margin-top: 4px; }
code { background: rgba(255,255,255,0.1); padding: 2px 4px; border-radius: 3px; font-size: 0.75rem; }
.ready-btn { background: rgba(37,211,102,0.3); border: 1px solid rgba(37,211,102,0.5); color: white; border-radius: 6px; padding: 8px; cursor: pointer; }
</style>
