<script setup>
// The helpers' notices as toasts: "Saved as lyrics - add a Suno key in Settings to hear it sung",
// each with a button to add the key. They fade after a while; several stack.
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { onNotice } from '../api/castle.js'

const emit = defineEmits(['keys'])
const toasts = ref([])
let stop
let next = 0

const WORDS = {
  image: { saved: 'the picture idea', key: 'ChatGPT (OpenAI)', made: 'draw it' },
  video: { saved: 'the video idea', key: 'Seedance', made: 'make the video' },
  song: { saved: 'the lyrics', key: 'Suno', made: 'hear it sung' },
  voice: { saved: 'the words', key: 'ElevenLabs', made: 'hear it spoken' },
}

function show(notice) {
  const w = WORDS[notice?.kind] ?? { saved: 'it', key: 'the', made: 'make it' }
  const id = next++
  toasts.value = [...toasts.value, { id, text: `Saved ${w.saved} instead - add the ${w.key} key in Settings to ${w.made}.` }]
  setTimeout(() => (toasts.value = toasts.value.filter((t) => t.id !== id)), 12000)
}

onMounted(async () => {
  try {
    stop = await onNotice(show)
  } catch {
    // outside Castle's window
  }
})
onBeforeUnmount(() => stop?.())
</script>

<template>
  <div class="toasts" aria-live="polite">
    <div v-for="t in toasts" :key="t.id" class="toast">
      <span>{{ t.text }}</span>
      <button @click="emit('keys')">Add key</button>
      <button class="close" aria-label="Dismiss" @click="toasts = toasts.filter((x) => x.id !== t.id)">✕</button>
    </div>
  </div>
</template>

<style scoped>
.toasts {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-width: 460px;
}
.toast {
  display: flex;
  gap: 10px;
  align-items: center;
  padding: 12px 14px;
  border: 1px solid var(--warn);
  border-radius: 12px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
  font-size: 13px;
}
button {
  flex: none;
  padding: 4px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.close {
  border: 0;
}
</style>
