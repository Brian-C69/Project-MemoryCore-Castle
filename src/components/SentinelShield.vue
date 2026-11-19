<script setup>
// The Sentinel's shield on the top bar, on every tab: quiet, a soft warning for "look", a glow for
// an alarm, faded when the last patrol is old, dim when it is not set up. Hovering shows the
// companion's one sentence; a click opens the Sentinel.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { onSentinelChanged, sentinelStatus } from '../api/castle.js'
import { shieldLook } from './sentinel.js'

const emit = defineEmits(['open'])
const status = ref(null)
let timer
let stop

const shown = computed(() => shieldLook(status.value))
const label = computed(() => {
  const words = { quiet: 'All quiet', look: 'Worth a look', alarm: 'Alarm', unset: 'Not set up', away: 'Windows for now' }
  return `Sentinel: ${words[shown.value.look]}. ${shown.value.line}`
})

async function read() {
  try {
    status.value = await sentinelStatus()
  } catch {
    // the shield stays as it was
  }
}

onMounted(async () => {
  read()
  timer = setInterval(read, 60 * 1000)
  stop = await onSentinelChanged(read)
})
onBeforeUnmount(() => {
  clearInterval(timer)
  stop?.()
})
defineExpose({ read })
</script>

<template>
  <button class="action shield" :class="[shown.look, { stale: shown.stale }]" :title="label" :aria-label="label" @click="emit('open')">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" aria-hidden="true">
      <path d="M8 1.5 13.5 3.5v4c0 3.4-2.3 5.9-5.5 7-3.2-1.1-5.5-3.6-5.5-7v-4z" />
      <path v-if="shown.look === 'quiet'" d="M5.6 8.1 7.3 9.8 10.6 6.3" />
      <path v-else-if="shown.look === 'look' || shown.look === 'alarm'" d="M8 5v3.6M8 10.6v.4" />
    </svg>
  </button>
</template>

<style scoped>
.shield {
  display: flex;
  align-items: center;
  justify-content: center;
}
.shield.quiet {
  color: var(--good);
}
.shield.look {
  color: var(--warn);
}
.shield.alarm {
  color: var(--bad);
  animation: sentinel-glow 1.6s ease-in-out infinite;
}
.shield.unset,
.shield.away {
  opacity: 0.55;
}
.shield.stale:not(.alarm) {
  opacity: 0.6;
}
@keyframes sentinel-glow {
  0%,
  100% {
    box-shadow: 0 0 0 0 transparent;
  }
  50% {
    box-shadow: 0 0 12px 2px color-mix(in srgb, var(--bad) 70%, transparent);
  }
}
@media (prefers-reduced-motion: reduce) {
  .shield.alarm {
    animation: none;
    box-shadow: 0 0 8px 1px color-mix(in srgb, var(--bad) 60%, transparent);
  }
}
</style>
