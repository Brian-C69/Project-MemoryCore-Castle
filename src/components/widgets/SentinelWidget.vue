<script setup>
// HOME's SENTINEL card: the shield's state and the companion's sentence, with the two lines that
// say the watch is really working - the last patrol and the five-minute watch (amber when it has
// missed its ticks). A click opens the same Sentinel sheet as the top bar's shield.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { onSentinelChanged, sentinelStatus } from '../../api/castle.js'
import { LOOK_WORDS, patrolAgo, shieldLook, watchAgo } from '../sentinel.js'

// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const emit = defineEmits(['open'])
const status = ref(null)
let timer
let stop

const shown = computed(() => shieldLook(status.value))
const word = computed(() => LOOK_WORDS[shown.value.look])
const ready = computed(() => status.value?.supported && status.value?.on && status.value?.installed)
const patrol = computed(() => patrolAgo(status.value))
const watch = computed(() => watchAgo(status.value))

async function read() {
  try {
    status.value = await sentinelStatus()
  } catch {
    // the card stays as it was
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
</script>

<template>
  <div
    class="widget sentinel"
    :class="[shown.look, { stale: shown.stale }]"
    role="button"
    tabindex="0"
    :aria-label="`Sentinel: ${word}. ${shown.line} Open the Sentinel.`"
    :title="`Sentinel: ${word}. ${shown.line}`"
    @click="emit('open')"
    @keydown.enter.prevent="emit('open')"
    @keydown.space.prevent="emit('open')"
  >
    <div class="head">
      <span class="title">SENTINEL</span>
      <span class="state">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" aria-hidden="true">
          <path d="M8 1.5 13.5 3.5v4c0 3.4-2.3 5.9-5.5 7-3.2-1.1-5.5-3.6-5.5-7v-4z" />
          <path v-if="shown.look === 'quiet'" d="M5.6 8.1 7.3 9.8 10.6 6.3" />
          <path v-else-if="shown.look === 'look' || shown.look === 'alarm'" d="M8 5v3.6M8 10.6v.4" />
        </svg>
        {{ word }}
      </span>
    </div>
    <p class="said">{{ shown.line }}</p>
    <div v-if="ready" class="lines">
      <span class="line">{{ patrol }}</span>
      <span class="line" :class="{ late: watch.late }">{{ watch.line }}</span>
    </div>
  </div>
</template>

<style scoped>
.sentinel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 230px;
  max-width: 290px;
  padding: 14px 16px;
  cursor: pointer;
}
.sentinel:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
}
.title {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}
.state {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  font-weight: 600;
}
.quiet .state {
  color: var(--good);
}
.look .state {
  color: var(--warn);
}
.alarm .state {
  color: var(--bad);
}
.alarm {
  animation: sentinel-glow 1.6s ease-in-out infinite;
}
.unset,
.away {
  opacity: 0.6;
}
.stale:not(.alarm) .state {
  opacity: 0.7;
}
.said {
  margin: 0;
  font-size: 13px;
  line-height: 1.4;
}
.lines {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.line {
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
.line.late {
  color: var(--warn);
}
@keyframes sentinel-glow {
  0%,
  100% {
    box-shadow: 0 0 0 0 transparent;
  }
  50% {
    box-shadow: 0 0 14px 2px color-mix(in srgb, var(--bad) 60%, transparent);
  }
}
@media (prefers-reduced-motion: reduce) {
  .alarm {
    animation: none;
    box-shadow: 0 0 8px 1px color-mix(in srgb, var(--bad) 50%, transparent);
  }
}
</style>
