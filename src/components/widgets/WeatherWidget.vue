<script setup>
// HOME's WEATHER card: the weather where the member is, and the days ahead. The place comes from
// the computer's location (only after the member allows it) or from a city they search for. Nothing
// but a place name or a pair of coordinates ever leaves the computer, and only to Open-Meteo.
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { weatherGet, weatherHere, weatherRead, weatherSave, weatherSearch } from '../../api/castle.js'
import { dayName, hereMessage, mood, placeLine, tempText, weatherTitle, windText } from './weather.js'
import HelpTip from '../HelpTip.vue'

defineProps({ size: { type: String, default: 'm' } })
// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const REFRESH_MS = 15 * 60 * 1000
const settings = ref({ place: null, units: 'metric' })
const reading = ref(null)
const loaded = ref(false)
const loading = ref(false)
const problem = ref('')
let timer

// The gear: where, and the units.
const gearOpen = ref(false)
const query = ref('')
const found = ref([])
const searching = ref(false)
const searched = ref(false)
const gearNote = ref('')
const locating = ref(false)
const card = ref(null)
const panelAt = ref({ top: 0, left: 0 })
const input = ref(null)

const title = computed(() => weatherTitle(reading.value))
const feel = computed(() => mood(reading.value?.icon, reading.value?.isDay ?? true))
const place = computed(() => placeLine(settings.value.place))
const wind = computed(() => windText(reading.value?.wind, reading.value?.windDir, reading.value?.units))
const today = computed(() => reading.value?.days?.[0] ?? null)

async function read(force = false) {
  if (!settings.value.place) {
    reading.value = null
    return
  }
  if (document.visibilityState !== 'visible' && reading.value) return
  loading.value = true
  try {
    reading.value = await weatherRead(force)
    problem.value = ''
  } catch (reason) {
    // The last reading stays; the card says it could not be updated.
    problem.value = String(reason)
  } finally {
    loading.value = false
  }
}

async function start() {
  try {
    settings.value = await weatherGet()
  } catch {
    // no place yet
  }
  loaded.value = true
  await read()
}

function placeGear() {
  const box = card.value?.getBoundingClientRect()
  if (!box) return
  const width = 300
  const left = Math.min(Math.max(8, box.left), window.innerWidth - width - 8)
  const below = box.bottom + 6
  const top = below + 380 > window.innerHeight ? Math.max(8, box.top - 386) : below
  panelAt.value = { top, left }
}

async function toggleGear() {
  gearOpen.value = !gearOpen.value
  if (!gearOpen.value) return
  gearNote.value = ''
  placeGear()
  await nextTick()
  input.value?.focus()
}

async function choose(chosen) {
  gearNote.value = ''
  try {
    settings.value = await weatherSave(chosen, settings.value.units)
    found.value = []
    query.value = ''
    searched.value = false
    await read(true)
  } catch (reason) {
    gearNote.value = String(reason)
  }
}

async function search() {
  const text = query.value.trim()
  searched.value = false
  if (text.length < 2) {
    found.value = []
    return
  }
  searching.value = true
  gearNote.value = ''
  try {
    found.value = await weatherSearch(text)
    searched.value = true
  } catch (reason) {
    found.value = []
    gearNote.value = String(reason)
  } finally {
    searching.value = false
  }
}

// Asks Windows where this computer is. The system asks the member first; Castle only receives the
// answer if they allow it.
function useMyLocation() {
  gearNote.value = ''
  if (!navigator.geolocation) {
    gearNote.value = hereMessage(2)
    return
  }
  locating.value = true
  navigator.geolocation.getCurrentPosition(
    async (position) => {
      try {
        await choose(await weatherHere(position.coords.latitude, position.coords.longitude))
      } catch (reason) {
        gearNote.value = String(reason)
      } finally {
        locating.value = false
      }
    },
    (failure) => {
      gearNote.value = hereMessage(failure?.code)
      locating.value = false
    },
    { enableHighAccuracy: false, timeout: 15000, maximumAge: 10 * 60 * 1000 },
  )
}

async function setUnits(units) {
  if (settings.value.units === units) return
  try {
    settings.value = await weatherSave(settings.value.place, units)
    await read(true)
  } catch (reason) {
    gearNote.value = String(reason)
  }
}

async function forget() {
  try {
    settings.value = await weatherSave(null, settings.value.units)
    reading.value = null
    problem.value = ''
  } catch (reason) {
    gearNote.value = String(reason)
  }
}

function onKey(event) {
  if (event.key === 'Escape' && gearOpen.value) gearOpen.value = false
}

function onPointer(event) {
  if (!gearOpen.value) return
  if (event.target.closest?.('.weather-panel') || event.target.closest?.('.weather .gear')) return
  gearOpen.value = false
}

onMounted(() => {
  start()
  timer = setInterval(() => read(), REFRESH_MS)
  document.addEventListener('visibilitychange', onVisible)
  window.addEventListener('keydown', onKey)
  window.addEventListener('pointerdown', onPointer, true)
})
onBeforeUnmount(() => {
  clearInterval(timer)
  document.removeEventListener('visibilitychange', onVisible)
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('pointerdown', onPointer, true)
})

function onVisible() {
  if (document.visibilityState === 'visible') read()
}
</script>

<template>
  <div ref="card" class="widget weather" :class="[feel, { 'no-place': loaded && !settings.place }]" :title="title || undefined">
    <!-- no place yet: an invitation, not an error -->
    <div v-if="loaded && !settings.place" class="invite">
      <svg class="big-icon" viewBox="0 0 64 64" aria-hidden="true">
        <circle cx="24" cy="26" r="9" class="sun" />
        <path d="M22 46h26a9 9 0 0 0 0-18 12 12 0 0 0-23-3 10 10 0 0 0-3 21z" class="cloud" />
      </svg>
      <div class="invite-text">
        <span class="head">WEATHER</span>
        <span class="line">See the sky where you are.</span>
        <button class="primary" type="button" @click="toggleGear">Set location</button>
      </div>
    </div>

    <!-- the weather -->
    <template v-else-if="reading">
      <div class="now">
        <svg class="big-icon" viewBox="0 0 64 64" aria-hidden="true">
          <template v-if="reading.icon === 'clear'">
            <g v-if="reading.isDay" class="rays">
              <circle cx="32" cy="32" r="11" class="sun" />
              <path d="M32 8v7M32 49v7M8 32h7M49 32h7M15 15l5 5M44 44l5 5M49 15l-5 5M20 44l-5 5" />
            </g>
            <path v-else class="moon" d="M41 12a21 21 0 1 0 11 38 17 17 0 0 1-11-38z" />
          </template>
          <template v-else-if="reading.icon === 'partly'">
            <circle v-if="reading.isDay" cx="24" cy="24" r="9" class="sun" />
            <path v-else class="moon" d="M30 8a14 14 0 1 0 8 25 11 11 0 0 1-8-25z" />
            <path d="M22 50h26a9 9 0 0 0 0-18 12 12 0 0 0-23-3 10 10 0 0 0-3 21z" class="cloud" />
          </template>
          <template v-else>
            <path d="M20 40h28a9 9 0 0 0 0-18 12 12 0 0 0-23-3 10 10 0 0 0-5 21z" class="cloud" :class="{ dark: reading.icon === 'storm' || reading.icon === 'rain' }" />
            <g v-if="reading.icon === 'rain'" class="drops"><path d="M24 46l-3 8M34 46l-3 8M44 46l-3 8" /></g>
            <g v-else-if="reading.icon === 'snow'" class="flakes">
              <circle cx="24" cy="50" r="2" /><circle cx="34" cy="53" r="2" /><circle cx="44" cy="50" r="2" />
            </g>
            <path v-else-if="reading.icon === 'storm'" class="bolt" d="M35 42l-8 11h6l-3 9 11-13h-6z" />
            <g v-else-if="reading.icon === 'fog'" class="mist"><path d="M16 48h32M22 55h26" /></g>
          </template>
        </svg>
        <div class="figures">
          <span class="temp">{{ tempText(reading.temp) }}<small>{{ reading.units === 'imperial' ? 'F' : 'C' }}</small></span>
          <span class="label">{{ reading.label }}</span>
          <span v-if="today" class="range">↑ {{ tempText(today.high) }}  ↓ {{ tempText(today.low) }}</span>
        </div>
      </div>

      <div class="place" :title="place">{{ place }}</div>

      <dl class="facts">
        <div v-if="reading.feels !== null"><dt>Feels</dt><dd>{{ tempText(reading.feels) }}</dd></div>
        <div v-if="reading.humidity !== null"><dt>Humidity</dt><dd>{{ reading.humidity }}%</dd></div>
        <div v-if="wind"><dt>Wind</dt><dd>{{ wind }}</dd></div>
      </dl>

      <ul v-if="reading.days.length > 1" class="days">
        <li v-for="(day, index) in reading.days.slice(1)" :key="day.date" :title="`${day.label}${day.rain !== null ? ` · ${day.rain}% chance of rain` : ''}`">
          <span class="dn">{{ dayName(day.date, index + 1) }}</span>
          <svg class="mini" viewBox="0 0 64 64" aria-hidden="true">
            <template v-if="day.icon === 'clear'"><circle cx="32" cy="32" r="12" class="sun" /></template>
            <template v-else-if="day.icon === 'partly'">
              <circle cx="24" cy="24" r="9" class="sun" />
              <path d="M22 50h26a9 9 0 0 0 0-18 12 12 0 0 0-23-3 10 10 0 0 0-3 21z" class="cloud" />
            </template>
            <template v-else>
              <path d="M16 42h32a10 10 0 0 0 0-20 13 13 0 0 0-25-3 11 11 0 0 0-7 23z" class="cloud" :class="{ dark: day.icon === 'rain' || day.icon === 'storm' }" />
              <path v-if="day.icon === 'rain'" class="drops" d="M24 48l-3 8M34 48l-3 8M44 48l-3 8" />
              <path v-else-if="day.icon === 'storm'" class="bolt" d="M35 42l-8 11h6l-3 9 11-13h-6z" />
              <path v-else-if="day.icon === 'fog'" class="mist" d="M16 50h32M22 57h26" />
              <g v-else-if="day.icon === 'snow'" class="flakes"><circle cx="24" cy="52" r="2" /><circle cx="34" cy="56" r="2" /><circle cx="44" cy="52" r="2" /></g>
            </template>
          </svg>
          <span class="hi">{{ tempText(day.high) }}</span>
          <span class="lo">{{ tempText(day.low) }}</span>
        </li>
      </ul>

      <p v-if="problem" class="problem" :title="problem">Could not update just now.</p>
      <a class="credit" href="https://open-meteo.com/" target="_blank" rel="noopener noreferrer" title="Weather data by Open-Meteo.com">Open-Meteo</a>
    </template>

    <!-- a place is chosen but nothing has been read yet -->
    <div v-else-if="loaded && settings.place" class="waiting">
      <span class="head">WEATHER</span>
      <span class="line">{{ problem ? 'Could not reach the weather service.' : 'Reading the sky…' }}</span>
      <span class="place">{{ place }}</span>
      <button v-if="problem" class="primary" type="button" :disabled="loading" @click="read(true)">Try again</button>
    </div>

    <button class="gear" type="button" :class="{ on: gearOpen }" aria-label="Weather settings" title="Weather settings: location and units" :aria-expanded="gearOpen" @click="toggleGear">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
      </svg>
    </button>

    <Teleport to="body">
      <div v-if="gearOpen" class="weather-panel" role="dialog" aria-label="Weather settings" :style="{ top: `${panelAt.top}px`, left: `${panelAt.left}px` }">
        <span class="panel-head with-tip">LOCATION<HelpTip block text="Windows asks you first. Only a rounded spot (about 1 km) is kept, on this computer, and sent to Open-Meteo to read the weather." /></span>
        <button class="here" type="button" :disabled="locating" @click="useMyLocation">{{ locating ? 'Waiting for Windows…' : 'Use my location' }}</button>
        <form class="finder" @submit.prevent="search">
          <input ref="input" v-model="query" type="text" maxlength="80" placeholder="Search a city" aria-label="Search a city" />
          <button type="submit" :disabled="searching || query.trim().length < 2">{{ searching ? '…' : 'Search' }}</button>
        </form>
        <ul v-if="found.length" class="results">
          <li v-for="p in found" :key="`${p.lat},${p.lon},${p.name}`">
            <button type="button" @click="choose(p)">
              <span class="rn">{{ p.name }}</span><span class="rr">{{ p.region }}</span>
            </button>
          </li>
        </ul>
        <p v-else-if="searched" class="hint">No place found. Try another spelling.</p>
        <p v-if="gearNote" class="note">{{ gearNote }}</p>

        <span class="panel-head section">UNITS</span>
        <div class="units" role="group" aria-label="Units">
          <button type="button" :class="{ on: settings.units === 'metric' }" @click="setUnits('metric')">°C · km/h</button>
          <button type="button" :class="{ on: settings.units === 'imperial' }" @click="setUnits('imperial')">°F · mph</button>
        </div>

        <button v-if="settings.place" class="forget" type="button" @click="forget">Forget my place</button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.weather {
  --sky: var(--accent);
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 250px;
  padding: 14px 16px 10px;
  overflow: hidden;
}

/* a soft glow in the sky's colour, kept low so any theme's text stays readable */
.weather::before {
  content: '';
  position: absolute;
  inset: 0;
  background: radial-gradient(120% 90% at 0% 0%, color-mix(in srgb, var(--sky) 22%, transparent), transparent 62%);
  pointer-events: none;
}

.weather.clear-day { --sky: #ffc857; }
.weather.partly-day { --sky: #ffd98a; }
.weather.clear-night,
.weather.partly-night { --sky: #8fa8ff; }
.weather.rain { --sky: #6fb4ff; }
.weather.storm { --sky: #b69cff; }
.weather.snow { --sky: #cfeaff; }
.weather.fog,
.weather.cloud { --sky: #aebdd0; }

.weather > * {
  position: relative;
}

.now {
  display: flex;
  align-items: center;
  gap: 12px;
}

.big-icon {
  flex: none;
  width: 68px;
  height: 68px;
  overflow: visible;
}

.mini {
  width: 30px;
  height: 30px;
  overflow: visible;
}

.sun { fill: #ffc857; stroke: none; }
.moon { fill: #dfe6ff; stroke: none; }
.cloud { fill: color-mix(in srgb, var(--text) 78%, transparent); stroke: none; }
.cloud.dark { fill: color-mix(in srgb, var(--text) 55%, var(--sky)); }
.rays path, .drops, .drops path, .mist, .mist path {
  fill: none;
  stroke: var(--sky);
  stroke-width: 3;
  stroke-linecap: round;
}
.rays path { stroke: #ffc857; }
.flakes circle { fill: var(--sky); }
.bolt { fill: #ffd54a; stroke: none; }

.figures {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.temp {
  font-size: 40px;
  line-height: 1;
  font-weight: 300;
  letter-spacing: -0.02em;
}

.temp small {
  margin-left: 2px;
  font-size: 14px;
  font-weight: 400;
  color: var(--text-dim);
  vertical-align: top;
}

.label {
  margin-top: 3px;
  font-size: 13px;
}

.range {
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-dim);
  white-space: pre;
}

.place {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 11px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--text-dim);
}

.facts {
  display: flex;
  gap: 14px;
  margin: 0;
  padding: 8px 0 2px;
  border-top: 1px solid var(--line);
}

.facts div {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.facts dt {
  font-size: 9px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-dim);
}

.facts dd {
  margin: 0;
  font-size: 13px;
}

.days {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  margin: 0;
  padding: 8px 0 0;
  list-style: none;
  border-top: 1px solid var(--line);
}

.days li {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  font-size: 12px;
}

.dn {
  font-size: 10px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-dim);
}

.lo {
  font-size: 11px;
  color: var(--text-dim);
}

.problem {
  margin: 0;
  font-size: 10px;
  color: var(--warn, #ffb37a);
}

.credit {
  align-self: flex-end;
  font-size: 9px;
  letter-spacing: 0.08em;
  color: var(--text-dim);
  opacity: 0.7;
  text-decoration: none;
}

.credit:hover {
  opacity: 1;
  text-decoration: underline;
}

.invite,
.waiting {
  display: flex;
  align-items: center;
  gap: 12px;
}

.waiting {
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
}

.invite-text {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 5px;
}

.head {
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.line {
  font-size: 13px;
}

.primary,
.here,
.finder button,
.units button,
.forget {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.primary {
  padding: 5px 12px;
}

.primary:hover:not(:disabled),
.here:hover:not(:disabled),
.finder button:hover:not(:disabled),
.units button:hover,
.forget:hover {
  background: color-mix(in srgb, var(--accent) 30%, transparent);
}

button:disabled {
  opacity: 0.55;
  cursor: default;
}

.gear {
  position: absolute;
  top: 8px;
  right: 8px;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.weather:hover .gear,
.weather:focus-within .gear,
.gear.on {
  opacity: 1;
}

.gear svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.gear:hover,
.gear.on {
  color: var(--text);
  background: color-mix(in srgb, var(--accent) 18%, transparent);
}

.no-place .gear {
  display: none;
}
</style>

<!-- the panel is teleported to the page, so it is styled outside this card's scope -->
<style>
.weather-panel {
  position: fixed;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 300px;
  max-height: calc(100vh - 16px);
  overflow: auto;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--background, #0c1424);
  color: var(--text);
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
  font-size: 12px;
}

.weather-panel .panel-head {
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.weather-panel .panel-head.section {
  margin-top: 4px;
}

/* a head with its (?): the bubble spans the panel's width under the head */
.weather-panel .panel-head.with-tip {
  position: relative;
  display: flex;
  align-items: center;
}

/* the panel's own button look must not reach the (?) */
.weather-panel .help-tip .mark,
.weather-panel .help-tip .mark:hover {
  background: transparent;
}

.weather-panel .hint,
.weather-panel .note {
  margin: 0;
  font-size: 11px;
  line-height: 1.45;
  color: var(--text-dim);
}

.weather-panel .note {
  color: var(--warn, #ffb37a);
}

.weather-panel button {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  color: var(--text);
  font: inherit;
  cursor: pointer;
}

.weather-panel button:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 30%, transparent);
}

.weather-panel button:disabled {
  opacity: 0.55;
  cursor: default;
}

.weather-panel .here {
  padding: 8px 10px;
}

.weather-panel .finder {
  display: flex;
  gap: 6px;
}

.weather-panel .finder input {
  flex: 1;
  min-width: 0;
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: color-mix(in srgb, var(--text) 6%, transparent);
  color: var(--text);
  font: inherit;
}

.weather-panel .finder button {
  padding: 6px 10px;
}

.weather-panel .results {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.weather-panel .results button {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  width: 100%;
  padding: 6px 8px;
  text-align: left;
}

.weather-panel .rr {
  font-size: 10px;
  color: var(--text-dim);
}

.weather-panel .units {
  display: flex;
  gap: 6px;
}

.weather-panel .units button {
  flex: 1;
  padding: 6px 8px;
}

.weather-panel .units button.on {
  background: color-mix(in srgb, var(--accent) 38%, transparent);
  border-color: var(--accent);
}

.weather-panel .forget {
  margin-top: 4px;
  padding: 6px 8px;
  color: var(--text-dim);
}
</style>
