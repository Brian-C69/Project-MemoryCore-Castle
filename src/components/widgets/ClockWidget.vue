<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { formatClock } from '../clock.js'

// `format` is Settings' clock format ("24h" or "12h"), the same one the top bar's clock uses
const props = defineProps({ size: { type: String, default: 'm' }, format: { type: String, default: '24h' } })
// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const now = ref(new Date())
let timer

// "16:31", or "4:31" with its "PM" set apart
const parts = computed(() => formatClock(now.value, props.format).split(' '))
const time = computed(() => parts.value[0])
const half = computed(() => parts.value[1] ?? '')
const date = computed(() =>
  now.value.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' }),
)

onMounted(() => {
  // Tick on the minute, so the clock never shows a stale minute for long.
  const tick = () => {
    now.value = new Date()
    timer = setTimeout(tick, 60000 - (Date.now() % 60000) + 50)
  }
  tick()
})
onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <div class="widget clock" :class="size" role="timer" :aria-label="`${time}${half ? ' ' + half : ''}, ${date}`">
    <div class="time">
      {{ time }}<span v-if="half" class="half">{{ half }}</span>
    </div>
    <div class="date">{{ date }}</div>
  </div>
</template>

<style scoped>
.clock {
  padding: 10px 22px 14px;
  text-align: center;
}

.time {
  font-size: 56px;
  font-weight: 300;
  line-height: 1.05;
  letter-spacing: 0.02em;
}

.half {
  margin-left: 6px;
  font-size: 15px;
  letter-spacing: 0.1em;
  color: var(--text-dim);
}

.date {
  margin-top: 4px;
  font-size: 15px;
  color: var(--text-dim);
}

.clock.xl {
  text-align: left;
}

.clock.xl .time {
  font-size: 128px;
  font-weight: 200;
}

.clock.xl .date {
  font-size: 20px;
}
</style>
