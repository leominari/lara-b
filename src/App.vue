<script setup lang="ts">
import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import WhatsAppAssistant from './components/WhatsAppAssistant.vue'

type Tab = 'tasks' | 'whatsapp'
const activeTab = ref<Tab>('whatsapp')

async function closeWidget() {
  await getCurrentWindow().close()
}
</script>

<template>
  <main class="container">
    <div class="widget">
      <div class="header" data-tauri-drag-region>
        <span class="title" data-tauri-drag-region>Meu Foco</span>
        <button class="close-btn" @click="closeWidget">✕</button>
      </div>

      <!-- Tab bar -->
      <div class="tabs">
        <button :class="['tab', { active: activeTab === 'tasks' }]" @click="activeTab = 'tasks'">Tarefas</button>
        <button :class="['tab', { active: activeTab === 'whatsapp' }]" @click="activeTab = 'whatsapp'">WhatsApp</button>
      </div>

      <!-- Content -->
      <div class="content">
        <div v-if="activeTab === 'tasks'" class="tab-content">
          <h1>Widget Ativo - Tarefas</h1>
        </div>
        <WhatsAppAssistant v-if="activeTab === 'whatsapp'" class="tab-content" />
      </div>
    </div>
  </main>
</template>

<style>
body, html, #app {
  background-color: transparent !important;
  margin: 0;
  padding: 0;
  overflow: hidden;
  width: 100vw;
  height: 100vh;
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
}

.container {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 20px;
  box-sizing: border-box;
}

.widget {
  background-color: rgba(0, 0, 0, 0.7);
  border-radius: 12px;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  color: white;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.1);
  overflow: hidden;
}

.header {
  height: 40px;
  background-color: rgba(255, 255, 255, 0.05);
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 15px;
  cursor: grab;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.header:active { cursor: grabbing; }

.title {
  font-size: 0.9rem;
  font-weight: 500;
  color: #ccc;
  user-select: none;
}

.close-btn {
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.6);
  font-size: 1.1rem;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  line-height: 1;
  transition: all 0.2s;
}

.close-btn:hover {
  background-color: rgba(255, 0, 0, 0.6);
  color: white;
}

.tabs {
  display: flex;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.tab {
  flex: 1;
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.5);
  padding: 8px;
  cursor: pointer;
  font-size: 0.8rem;
  transition: all 0.2s;
}

.tab.active {
  color: white;
  border-bottom: 2px solid #25d366;
}

.content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.tab-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

h1 {
  font-size: 1.5rem;
  font-weight: 500;
  text-align: center;
  margin: 0;
}
</style>
