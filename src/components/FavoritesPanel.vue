<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

const props = defineProps<{
  getContacts: () => Promise<string[]>
  getFavorites: () => Promise<string[]>
  addFavorite: (contact: string) => Promise<void>
  removeFavorite: (contact: string) => Promise<void>
}>()

const emit = defineEmits<{ close: [] }>()

const contacts = ref<string[]>([])
const favorites = ref<Set<string>>(new Set())
const search = ref('')

onMounted(async () => {
  const [c, f] = await Promise.all([props.getContacts(), props.getFavorites()])
  contacts.value = c
  favorites.value = new Set(f)
})

const filtered = computed(() => {
  const q = search.value.toLowerCase()
  return q ? contacts.value.filter(c => c.toLowerCase().includes(q)) : contacts.value
})

async function toggle(contact: string) {
  if (favorites.value.has(contact)) {
    await props.removeFavorite(contact)
    favorites.value.delete(contact)
  } else {
    await props.addFavorite(contact)
    favorites.value.add(contact)
  }
  favorites.value = new Set(favorites.value)
}
</script>

<template>
  <div class="favorites">
    <div class="fav-header">
      <span>Favoritos</span>
      <button class="close-btn" @click="emit('close')">✕</button>
    </div>

    <input
      v-model="search"
      class="fav-search"
      type="text"
      placeholder="Buscar contato..."
    />

    <div class="fav-list">
      <div v-if="contacts.length === 0" class="fav-empty">
        Nenhum contato ainda. Mensagens sincronizadas aparecem aqui.
      </div>
      <div v-else-if="filtered.length === 0" class="fav-empty">
        Nenhum contato encontrado.
      </div>
      <div
        v-for="contact in filtered"
        :key="contact"
        class="fav-item"
        @click="toggle(contact)"
      >
        <span class="fav-name">{{ contact }}</span>
        <span class="fav-star" :class="{ active: favorites.has(contact) }">
          {{ favorites.has(contact) ? '★' : '☆' }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.favorites {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: rgba(18, 18, 18, 0.97);
  backdrop-filter: blur(6px);
  display: flex;
  flex-direction: column;
  padding: 12px;
  font-family: Inter, sans-serif;
  color: white;
  gap: 10px;
}

.fav-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 500;
}

.close-btn {
  background: transparent;
  border: none;
  color: rgba(255,255,255,0.6);
  cursor: pointer;
  font-size: 1rem;
}

.fav-search {
  background: rgba(255,255,255,0.1);
  border: 1px solid rgba(255,255,255,0.2);
  border-radius: 8px;
  padding: 6px 10px;
  color: white;
  font-size: 0.8rem;
  outline: none;
}
.fav-search::placeholder { color: #666; }

.fav-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fav-empty {
  font-size: 0.75rem;
  color: #666;
  text-align: center;
  padding: 20px 0;
}

.fav-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 120ms;
}
.fav-item:hover { background: rgba(255,255,255,0.07); }

.fav-name {
  font-size: 0.8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fav-star {
  font-size: 1rem;
  color: #555;
  flex-shrink: 0;
  transition: color 120ms;
}
.fav-star.active { color: #f5c518; }
</style>
