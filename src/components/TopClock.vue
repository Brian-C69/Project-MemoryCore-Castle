<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { formatClock } from './clock.js'

const props = defineProps({ format: { type: String, default: '24h' } })
const now = ref(new Date())
const text = computed(() => formatClock(now.value, props.format))
const date = computed(() => now.value.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' }))
let timer
function tick() {
  clearTimeout(timer)
  now.value = new Date()
  timer = setTimeout(tick, 60000 - Date.now() % 60000 + 50)
}
function resume() { if (!document.hidden) tick() }
onMounted(() => { tick(); document.addEventListener('visibilitychange', resume) })
onBeforeUnmount(() => { clearTimeout(timer); document.removeEventListener('visibilitychange', resume) })
</script>

<template>
  <time class="top-clock" :datetime="now.toISOString()" :title="date" :aria-label="`Local time ${text}, ${date}`">{{ text }}</time>
</template>

<style scoped>
.top-clock { align-self: center; flex-shrink: 0; padding: 0 10px; font-size: 13px; font-variant-numeric: tabular-nums; white-space: nowrap; color: var(--text); }
</style>
