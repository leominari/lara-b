<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";

async function closeWidget() {
  await getCurrentWindow().close();
}
</script>

<template>
  <main class="container">
    <div class="widget">
      <!-- Drag Region Header -->
      <div class="header" data-tauri-drag-region>
        <span class="title" data-tauri-drag-region>Meu Foco</span>
        <button class="close-btn" @click="closeWidget">✕</button>
      </div>

      <!-- Content Area -->
      <div class="content">
        <h1>Widget Ativo - Tarefas</h1>
      </div>
    </div>
  </main>
</template>

<style>
/* Global resets for transparent widget */
body,
html,
#app {
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
  overflow: hidden; /* Ensures header stays within rounded corners */
}

.header {
  height: 40px;
  background-color: rgba(255, 255, 255, 0.05);
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 15px;
  /* Make the exact region draggable */
  cursor: grab;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.header:active {
  cursor: grabbing;
}

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

.content {
  flex: 1;
  display: flex;
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
