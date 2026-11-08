<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { systemMeters } from '../../api/castle.js'

defineProps({ size: { type: String, default: 'm' } })
// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const RADIUS = 30
const CIRCUMFERENCE = 2 * Math.PI * RADIUS
const meters = ref({ cpu: 0, memory: 0 })
let timer

async function read() {
  if (document.visibilityState !== 'visible') return
  try {
    meters.value = await systemMeters()
  } catch {
    // The last reading stays.
  }
}

const dash = (value) => `${(value / 100) * CIRCUMFERENCE} ${CIRCUMFERENCE}`

onMounted(() => {
  read()
  timer = setInterval(read, 3000)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <div class="widget gauges">
    <div v-for="gauge in [
      { key: 'cpu', label: 'CPU', value: meters.cpu },
      { key: 'memory', label: 'RAM', value: meters.memory },
    ]" :key="gauge.key" class="gauge" role="meter" :aria-label="gauge.label" :aria-valuenow="gauge.value" aria-valuemin="0" aria-valuemax="100">
      <svg viewBox="0 0 80 80" aria-hidden="true">
        <circle cx="40" cy="40" :r="RADIUS" class="track" />
        <circle cx="40" cy="40" :r="RADIUS" class="fill" :class="gauge.key" :stroke-dasharray="dash(gauge.value)" />
      </svg>
      <span class="value">{{ gauge.value }}%</span>
      <span class="label">{{ gauge.label }}</span>
    </div>
  </div>
</template>

<style scoped>
.gauges {
  display: flex;
  gap: 14px;
  padding: 12px 14px;
}

.gauge {
  position: relative;
  width: 84px;
  height: 84px;
}

svg {
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
}

circle {
  fill: none;
  stroke-width: 5;
}

.track {
  stroke: var(--line);
  opacity: 0.45;
}

.fill {
  stroke: var(--accent);
  stroke-linecap: round;
  transition: stroke-dasharray 0.6s ease;
}

.fill.memory {
  stroke: var(--accent-2, #8cc8ff);
}

.value,
.label {
  position: absolute;
  left: 0;
  right: 0;
  text-align: center;
}

.value {
  top: 26px;
  font-size: 17px;
}

.label {
  top: 49px;
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}
</style>
