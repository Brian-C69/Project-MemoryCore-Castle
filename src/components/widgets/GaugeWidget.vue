<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { gpuGaugesSave, gpuMeters, meterTrimNow, meterTrips, meterWatchGet, meterWatchSave, systemMeters } from '../../api/castle.js'
import { gpuTitle } from '../gpuGauges.js'
import { DEFAULT_LIMIT, boundLimit, freedLine, isHot, sizeLabel, timeLabel } from '../meterWatch.js'
import HelpTip from '../HelpTip.vue'

defineProps({ size: { type: String, default: 'm' } })
// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const RADIUS = 30
const CIRCUMFERENCE = 2 * Math.PI * RADIUS
const meters = ref({ cpu: 0, memory: 0 })
// the graphics cards: what to draw, whether they are shown, and how many were found
const gpu = ref({ gauges: [], on: true, found: 0 })
const gpuNote = ref('')
let timer

// The gear: watch on or off, the value each meter trips at, Tidy RAM now, and the last trips.
const watch = ref({ on: true, cpu: DEFAULT_LIMIT, memory: DEFAULT_LIMIT })
const gearOpen = ref(false)
const trips = ref([])
const tidying = ref(false)
const tidied = ref('')
const gearNote = ref('')
// The widget slot is scaled, so the panel floats on the page instead of inside the card.
const card = ref(null)
const panel = ref(null)
const panelAt = ref({ top: 0, left: 0 })

async function read() {
  if (document.visibilityState !== 'visible') return
  try {
    meters.value = await systemMeters()
  } catch {
    // The last reading stays.
  }
  try {
    gpu.value = await gpuMeters()
  } catch {
    // The last reading stays.
  }
}

// The graphics-card switch in the gear.
async function saveGpu() {
  try {
    gpu.value = await gpuGaugesSave(!!gpu.value.on)
    gpuNote.value = ''
  } catch (reason) {
    gpuNote.value = String(reason)
  }
}

function place() {
  const box = card.value?.getBoundingClientRect()
  if (!box) return
  const width = 280
  const left = Math.min(Math.max(8, box.left), window.innerWidth - width - 8)
  const below = box.bottom + 6
  // Above the card when there is no room under it.
  const top = below + 300 > window.innerHeight ? Math.max(8, box.top - 306) : below
  panelAt.value = { top, left }
}

async function toggleGear() {
  gearOpen.value = !gearOpen.value
  if (!gearOpen.value) return
  place()
  tidied.value = ''
  gearNote.value = ''
  try {
    trips.value = await meterTrips()
  } catch {
    trips.value = []
  }
}

function closeOnOutside(event) {
  if (!gearOpen.value) return
  if (panel.value?.contains(event.target) || card.value?.contains(event.target)) return
  gearOpen.value = false
}

function closeOnEscape(event) {
  if (event.key === 'Escape') gearOpen.value = false
}

async function save() {
  const next = {
    on: watch.value.on,
    cpu: boundLimit(watch.value.cpu, DEFAULT_LIMIT),
    memory: boundLimit(watch.value.memory, DEFAULT_LIMIT),
  }
  try {
    watch.value = await meterWatchSave(next.on, next.cpu, next.memory)
    gearNote.value = ''
  } catch (reason) {
    gearNote.value = String(reason)
  }
}

async function tidyNow() {
  tidying.value = true
  tidied.value = ''
  try {
    tidied.value = freedLine(await meterTrimNow())
  } catch (reason) {
    tidied.value = String(reason)
  } finally {
    tidying.value = false
  }
}

// every gauge on the card: CPU and RAM, then one for each graphics card shown
const gauges = computed(() => [
  { key: 'cpu', label: 'CPU', value: meters.value.cpu, limit: watch.value.cpu, title: '' },
  { key: 'memory', label: 'RAM', value: meters.value.memory, limit: watch.value.memory, title: '' },
  ...gpu.value.gauges.map((g) => ({ key: 'gpu', id: `gpu${g.index}`, label: g.label, value: g.util, limit: null, title: gpuTitle(g) })),
])

const dash = (value) => `${(value / 100) * CIRCUMFERENCE} ${CIRCUMFERENCE}`

onMounted(async () => {
  read()
  timer = setInterval(read, 3000)
  document.addEventListener('pointerdown', closeOnOutside, true)
  document.addEventListener('keydown', closeOnEscape)
  try {
    watch.value = await meterWatchGet()
  } catch {
    // The defaults stay.
  }
})
onBeforeUnmount(() => {
  clearInterval(timer)
  document.removeEventListener('pointerdown', closeOnOutside, true)
  document.removeEventListener('keydown', closeOnEscape)
})
</script>

<template>
  <div ref="card" class="widget gauges">
    <div v-for="gauge in gauges" :key="gauge.id ?? gauge.key" class="gauge" :style="{ '--load': gauge.value }" :title="gauge.title || undefined" role="meter" :aria-label="gauge.label" :aria-valuenow="gauge.value" aria-valuemin="0" aria-valuemax="100">
      <svg viewBox="0 0 80 80" aria-hidden="true">
        <circle cx="40" cy="40" :r="RADIUS" class="track" />
        <circle cx="40" cy="40" :r="RADIUS" class="fill" :class="[gauge.key, { hot: gauge.limit !== null && isHot(gauge.value, gauge.limit, watch.on) }]" :stroke-dasharray="dash(gauge.value)" />
      </svg>
      <span class="value">{{ gauge.value }}%</span>
      <span class="label">{{ gauge.label }}</span>
    </div>

    <button class="gear" type="button" :class="{ on: gearOpen }" aria-label="Gauge settings" title="Gauge settings: graphics cards, CPU and RAM watch" :aria-expanded="gearOpen" @click="toggleGear">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
      </svg>
    </button>

    <Teleport to="body">
      <div v-if="gearOpen" ref="panel" class="gauge-panel" role="dialog" aria-label="Gauge settings" :style="{ top: `${panelAt.top}px`, left: `${panelAt.left}px` }">
        <div class="gpu-edit first">
          <span class="trips-head">GRAPHICS CARDS</span>
          <label class="toggle">
            <input v-model="gpu.on" type="checkbox" @change="saveGpu" />
            <span>Show graphics cards</span>
          </label>
          <p class="hint">{{ gpu.found ? `${gpu.found} found.` : 'None found: needs an NVIDIA card with its driver (nvidia-smi).' }}</p>
          <p v-if="gpuNote" class="note">{{ gpuNote }}</p>
        </div>
        <span class="trips-head section with-tip">CPU AND RAM<HelpTip block text="When CPU or RAM is held at its limit for about 15 seconds, RAM is tidied by itself and the busiest apps are shown. Nothing is closed without you." /></span>
        <label class="toggle">
          <input v-model="watch.on" type="checkbox" @change="save" />
          <span>Watch CPU and RAM</span>
        </label>
        <div class="limits" :class="{ off: !watch.on }">
          <label>CPU at <input v-model="watch.cpu" type="number" min="50" max="99" :disabled="!watch.on" @change="save" />%</label>
          <label>RAM at <input v-model="watch.memory" type="number" min="50" max="99" :disabled="!watch.on" @change="save" />%</label>
        </div>
        <button class="tidy" type="button" :disabled="tidying" @click="tidyNow">{{ tidying ? 'Tidying…' : 'Tidy RAM now' }}</button>
        <p v-if="tidied" class="hint">{{ tidied }}</p>
        <p v-if="gearNote" class="note">{{ gearNote }}</p>
        <div v-if="trips.length" class="trips">
          <span class="trips-head">LAST TRIPS</span>
          <div v-for="t in trips" :key="t.at + t.meter" class="trip">
            <span>{{ timeLabel(t.at) }}</span>
            <span>{{ t.meter === 'memory' ? 'RAM' : 'CPU' }} {{ t.value }}%</span>
            <span class="freed">{{ t.freedMb ? `−${sizeLabel(t.freedMb)}` : '' }}</span>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.gauges {
  position: relative;
  display: flex;
  flex-wrap: wrap;
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
  transition: stroke-dasharray 0.6s ease, stroke 0.4s ease;
}

.fill.memory {
  stroke: var(--accent-2, #8cc8ff);
}

.fill.gpu {
  stroke: var(--accent-3, #b9a2ff);
}

.fill.hot {
  stroke: var(--warn, #ffb37a);
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
  padding: 0 8px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}

.gear {
  position: absolute;
  top: 6px;
  right: 6px;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: none;
  color: var(--text-dim);
  opacity: 0.55;
  cursor: pointer;
}

.gauges:hover .gear,
.gear.on,
.gear:focus-visible {
  opacity: 1;
}

.gear:hover {
  color: var(--text);
}

.gear svg {
  width: 15px;
  height: 15px;
  transform: none;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
}

.gauge-panel {
  position: fixed;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 280px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
  color: var(--text);
  font-size: 13px;
}

.toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.limits {
  display: flex;
  gap: 14px;
}

.limits.off {
  opacity: 0.5;
}

.limits input {
  width: 48px;
  margin: 0 3px;
  padding: 3px 5px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
}

.hint,
.note {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.45;
  color: var(--text-dim);
}

.note {
  color: var(--warn, #ffd59a);
}

.tidy {
  align-self: flex-start;
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.tidy:disabled {
  opacity: 0.6;
  cursor: default;
}

.gpu-edit {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.section {
  padding-top: 8px;
  border-top: 1px solid var(--line);
}

/* a section head with its (?): the bubble spans the panel's width under the head */
.with-tip {
  position: relative;
  display: flex;
  align-items: center;
}

.trips {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding-top: 8px;
  border-top: 1px solid var(--line);
}

.trips-head {
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.trip {
  display: grid;
  grid-template-columns: 70px 1fr auto;
  gap: 8px;
  font-size: 12px;
}

.freed {
  color: var(--good, #9be7b0);
}
</style>
