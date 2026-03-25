<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import lottie, { AnimationItem } from 'lottie-web'
import type { CatState } from '../composables/useAssistant'

const props = defineProps<{ state: CatState }>()

const container = ref<HTMLDivElement | null>(null)
let anim: AnimationItem | null = null

const speedMap: Record<CatState, number> = {
  idle: 1.0,
  syncing: 3.0,
  thinking: 0.3,
  responding: 2.0,
  error: 1.0,
}

onMounted(() => {
  if (!container.value) return
  anim = lottie.loadAnimation({
    container: container.value,
    renderer: 'svg',
    loop: true,
    autoplay: true,
    path: '/loader-cat.json',
  })
})

onUnmounted(() => anim?.destroy())

watch(() => props.state, (state) => {
  if (!anim) return
  if (state === 'error') {
    anim.pause()
  } else {
    anim.play()
    anim.setSpeed(speedMap[state])
  }
})
</script>

<template>
  <div class="cat-wrapper" :class="`cat-${state}`">
    <div ref="container" class="cat-container" />
  </div>
</template>

<style scoped>
.cat-container {
  width: 140px;
  height: 100px;
  position: relative;
}
.cat-error .cat-container {
  filter: hue-rotate(300deg) saturate(2);
}
.cat-thinking .cat-container::after {
  content: '...';
  position: absolute;
  bottom: 4px;
  right: 4px;
  font-size: 0.8rem;
  color: white;
  animation: blink 1s infinite;
}
.cat-wrapper {
  position: relative;
  display: inline-block;
}
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
</style>
