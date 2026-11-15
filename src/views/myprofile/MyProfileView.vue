<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { onCastleHidden, onProfileProgress, profileMonth, profileRead, profileScan } from '../../api/castle.js'
import { barWidth, CARDS, deltaText, glowing, grouped, monthLong, monthShort, progressLine, tileDeltas, yearMark } from './myprofile.js'

// MY PROFILE: the member's own personality, read by their own CLI from their own diary, one
// reading per finished month. It is personal, so the page starts veiled and veils again whenever
// Castle goes out of sight; nothing of it ever reaches the chat room.
defineProps({
  companion: { type: String, default: 'Your companion' },
})
const emit = defineEmits(['settings'])

const data = ref({ ready: false, timeline: [], current: null, waiting: [] })
const shown = ref(null) // the month on screen, in full
const veiled = ref(true)
const loading = ref(true)
const busy = ref(false)
const progress = ref(null)
const problem = ref('')
const noDatabase = ref(false)
const track = ref(null)

const timeline = computed(() => data.value.timeline)
const glow = computed(() => glowing(timeline.value))
const deltas = computed(() => (shown.value ? tileDeltas(timeline.value, shown.value.month) : {}))
const groups = computed(() => grouped(shown.value))
const waitingLine = computed(() => {
  const n = data.value.waiting.length
  return n ? `${n} finished month${n === 1 ? '' : 's'} not read yet` : 'Every finished month is read'
})

function failed(reason) {
  const why = String(reason?.message ?? reason)
  noDatabase.value = /database/i.test(why)
  problem.value = why
}

async function load() {
  try {
    data.value = await profileRead()
    shown.value = data.value.current
    noDatabase.value = false
    problem.value = ''
    await nextTick()
    if (track.value) track.value.scrollLeft = track.value.scrollWidth
  } catch (reason) {
    failed(reason)
  } finally {
    loading.value = false
  }
}

async function open(month) {
  if (shown.value?.month === month) return
  try {
    shown.value = (await profileMonth(month)) ?? shown.value
  } catch (reason) {
    failed(reason)
  }
}

async function scan() {
  if (busy.value) return
  busy.value = true
  problem.value = ''
  progress.value = null
  try {
    const result = await profileScan()
    await load()
    if (result.skipped.length) problem.value = `${result.skipped.map(monthLong).join(', ')} could not be read this time. Scan again to try once more.`
  } catch (reason) {
    failed(reason)
  } finally {
    busy.value = false
    progress.value = null
  }
}

const veil = () => (veiled.value = true)
const onVisibility = () => document.visibilityState === 'hidden' && veil()
let stopProgress = () => {}
let stopHidden = () => {}
onMounted(async () => {
  document.addEventListener('visibilitychange', onVisibility)
  try {
    stopProgress = await onProfileProgress((p) => (progress.value = p))
    stopHidden = await onCastleHidden(veil)
  } catch {
    // outside Castle's window: no events
  }
  load()
})
onBeforeUnmount(() => {
  document.removeEventListener('visibilitychange', onVisibility)
  stopProgress()
  stopHidden()
})
</script>

<template>
  <section class="myprofile" aria-label="My profile">
    <div v-if="noDatabase" class="invite">
      <p>Your profile lives in the memory database. Move your memory in first, then come back here.</p>
      <button class="primary" @click="emit('settings')">Open Settings &gt; Database</button>
    </div>

    <div v-else-if="!loading && !data.ready" class="empty">
      <h2>MY PROFILE</h2>
      <p>{{ companion }} reads your diary one finished month at a time and keeps how you seemed that month: your type, what you value, how you think, and your shadow.</p>
      <p class="dim">The month you are in is read once it is over. Only you see this page.</p>
      <button class="primary" :disabled="busy" @click="scan">{{ busy ? 'Reading…' : 'Scan my diary' }}</button>
      <p v-if="progress" class="hint">{{ progressLine(progress) }}</p>
      <p v-if="problem" class="problem" role="alert">{{ problem }}</p>
    </div>

    <template v-else-if="shown">
      <div class="content" :class="{ veiled }" :aria-hidden="veiled">
        <header class="hero card">
          <div class="emblem" aria-hidden="true">
            <svg viewBox="0 0 64 64">
              <circle cx="32" cy="32" r="27" />
              <path d="M32 10 L36 28 L54 32 L36 36 L32 54 L28 36 L10 32 L28 28 Z" />
            </svg>
          </div>
          <div class="who">
            <h2 class="type">{{ shown.mbti ?? '····' }}</h2>
            <p v-if="shown.enneagram || shown.tritype" class="chips">
              <span v-if="shown.enneagram">Enneagram {{ shown.enneagram }}</span>
              <span v-if="shown.tritype">Tritype {{ shown.tritype }}</span>
            </p>
            <p v-if="shown.headline" class="headline">{{ shown.headline }}</p>
            <p class="dim small">{{ monthLong(shown.month) }}<template v-if="shown.entriesRead"> · {{ shown.entriesRead }} diary entries</template></p>
          </div>
          <div class="tiles">
            <div v-for="tile in groups.scores.headline" :key="tile.label" class="tile">
              <span class="label">{{ tile.label }}</span>
              <div class="row">
                <div class="track" role="meter" :aria-label="tile.label" aria-valuemin="0" aria-valuemax="100" :aria-valuenow="tile.score">
                  <div class="fill gold" :style="{ width: barWidth(tile.score) }" />
                </div>
                <span class="num">{{ tile.score }}%</span>
              </div>
              <span v-if="deltas[tile.label] != null" class="delta" :class="{ up: deltas[tile.label] > 0, down: deltas[tile.label] < 0 }">{{ deltaText(deltas[tile.label]) }} since {{ monthShort(timeline[timeline.findIndex((p) => p.month === shown.month) - 1]?.month) }}</span>
            </div>
          </div>
        </header>

        <section class="changes card" aria-label="Changes">
          <h3>Changes</h3>
          <div ref="track" class="timeline">
            <div class="line" />
            <button
              v-for="(point, i) in timeline"
              :key="point.month"
              class="dot"
              :class="{ glow: glow.has(point.month), on: shown.month === point.month }"
              :title="`${monthLong(point.month)}${point.mbti ? ' · ' + point.mbti : ''}`"
              :aria-pressed="shown.month === point.month"
              @click="open(point.month)"
            >
              <span class="mark" />
              <span class="name">{{ monthShort(point.month) }}</span>
              <span class="year">{{ yearMark(timeline, i) }}</span>
            </button>
          </div>
        </section>

        <div class="cards">
          <section v-for="card in CARDS" :key="card.grp" class="card group" :aria-label="card.title">
            <h3>{{ card.title }}</h3>
            <div v-for="bar in groups.scores[card.grp]" :key="bar.label" class="bar">
              <span class="label">{{ bar.label }}</span>
              <div class="track"><div class="fill" :class="card.tone" :style="{ width: barWidth(bar.score) }" /></div>
              <span class="num">{{ bar.score }}%</span>
            </div>
            <p v-if="!groups.scores[card.grp].length" class="dim small">That month said little here.</p>
            <ul v-if="groups.notes[card.grp].length" class="notes">
              <li v-for="line in groups.notes[card.grp]" :key="line">{{ line }}</li>
            </ul>
          </section>
        </div>

        <section v-if="groups.notes.patterns.length || groups.notes.changes.length" class="card more">
          <div v-if="groups.notes.patterns.length">
            <h3>What kept coming back</h3>
            <ul class="notes"><li v-for="line in groups.notes.patterns" :key="line">{{ line }}</li></ul>
          </div>
          <div v-if="groups.notes.changes.length">
            <h3>What changed</h3>
            <ul class="notes"><li v-for="line in groups.notes.changes" :key="line">{{ line }}</li></ul>
          </div>
        </section>

        <footer class="scanbar">
          <span class="dim small">{{ waitingLine }}</span>
          <button :disabled="busy || !data.waiting.length" @click="scan">{{ busy ? 'Reading…' : 'Scan my diary' }}</button>
          <span v-if="progress" class="hint">{{ progressLine(progress) }}</span>
          <span v-if="problem" class="problem" role="alert">{{ problem }}</span>
        </footer>
      </div>

      <button class="eye" :aria-label="veiled ? 'Show my profile' : 'Hide my profile'" :title="veiled ? 'Show my profile' : 'Hide my profile'" @click="veiled = !veiled">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M2 12 C5 6, 19 6, 22 12 C19 18, 5 18, 2 12 Z" />
          <circle cx="12" cy="12" r="3" />
          <line v-if="veiled" x1="4" y1="20" x2="20" y2="4" />
        </svg>
      </button>
      <button v-if="veiled" class="unveil" @click="veiled = false">Show my profile</button>
    </template>

    <p v-else-if="problem" class="problem center" role="alert">{{ problem }}</p>
  </section>
</template>

<style scoped>
.myprofile {
  --gold: #e6c27a;
  --violet: #b48cff;
  position: absolute;
  inset: 16px;
  overflow-y: auto;
  scrollbar-color: var(--line) transparent;
  scrollbar-width: thin;
  padding: 20px 24px;
  /* no card around the page: its own cards (hero, timeline, ...) stand straight on the theme */
  color: var(--text);
}

.content {
  display: flex;
  flex-direction: column;
  gap: 14px;
  transition: filter 0.25s ease;
}

.content.veiled {
  filter: blur(18px);
  pointer-events: none;
  user-select: none;
}

.card {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
  padding: 16px 18px;
}

h2,
h3 {
  margin: 0;
  font-weight: 500;
}

h3 {
  font-size: 1.15rem;
  margin-bottom: 12px;
}

.dim {
  color: var(--text-dim);
}

.small {
  font-size: 0.82rem;
}

/* the hero */
.hero {
  display: grid;
  grid-template-columns: auto minmax(200px, 1fr) minmax(0, 1.6fr);
  gap: 20px;
  align-items: center;
}

.emblem svg {
  width: 84px;
  height: 84px;
  fill: none;
  stroke: var(--gold);
  stroke-width: 1.4;
  filter: drop-shadow(0 0 10px rgba(230, 194, 122, 0.45));
}

.emblem path {
  fill: var(--gold);
}

.type {
  font-family: Georgia, 'Times New Roman', serif;
  font-size: 3rem;
  letter-spacing: 0.04em;
  color: var(--gold);
}

.chips {
  display: flex;
  gap: 6px;
  margin: 4px 0;
}

.chips span {
  font-size: 0.75rem;
  padding: 2px 8px;
  border: 1px solid rgba(230, 194, 122, 0.5);
  border-radius: 999px;
  color: var(--gold);
}

.headline {
  margin: 6px 0;
  line-height: 1.45;
}

.tiles {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.tile {
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.label {
  font-size: 0.75rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--text-dim);
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.track {
  flex: 1;
  height: 6px;
  border-radius: 999px;
  background: var(--hover);
  overflow: hidden;
}

.fill {
  height: 100%;
  border-radius: 999px;
}

.fill.gold {
  background: var(--gold);
}

.fill.violet {
  background: var(--violet);
}

.num {
  font-size: 0.8rem;
  min-width: 36px;
  text-align: right;
  color: var(--text-dim);
}

.delta {
  font-size: 0.75rem;
  color: var(--text-dim);
}

.delta.up {
  color: var(--good, #9fe0b0);
}

.delta.down {
  color: var(--warn, #ffd59a);
}

/* the timeline */
.timeline {
  position: relative;
  display: flex;
  gap: 0;
  overflow-x: auto;
  scrollbar-color: var(--line) transparent;
  scrollbar-width: thin;
  padding: 6px 4px 4px;
}

.line {
  position: absolute;
  left: 0;
  right: 0;
  top: 18px;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(230, 194, 122, 0.5), transparent);
}

.dot {
  position: relative;
  flex: 0 0 78px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 4px 0;
}

.mark {
  width: 9px;
  height: 9px;
  margin-top: 4px;
  border-radius: 50%;
  background: var(--gold);
  opacity: 0.75;
}

.dot.glow .mark {
  width: 15px;
  height: 15px;
  margin-top: 1px;
  opacity: 1;
  box-shadow: 0 0 12px 3px rgba(230, 194, 122, 0.6);
}

.dot.on .mark {
  outline: 2px solid var(--text);
  outline-offset: 3px;
}

.dot.on .name {
  color: var(--text);
}

.name {
  font-size: 0.75rem;
  letter-spacing: 0.12em;
}

.year {
  font-size: 0.68rem;
  min-height: 0.8rem;
}

/* the three cards */
.cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
}

.bar {
  display: grid;
  grid-template-columns: 120px 1fr 40px;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.notes {
  margin: 12px 0 0;
  padding: 12px 0 0 18px;
  border-top: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  gap: 6px;
  line-height: 1.4;
  font-size: 0.9rem;
}

.more {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 18px;
}

.more .notes {
  border-top: none;
  padding-top: 0;
  margin-top: 0;
}

.scanbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}

/* the eye */
.eye {
  position: absolute;
  top: 26px;
  right: 30px;
  width: 38px;
  height: 30px;
  display: grid;
  place-items: center;
  padding: 0; /* the shared button padding pushed the icon off centre */
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  cursor: pointer;
  z-index: 2;
}

.eye svg {
  display: block;
  width: 20px;
  height: 20px;
  fill: none;
  stroke: var(--violet);
  stroke-width: 1.6;
}

.unveil {
  position: absolute;
  left: 50%;
  top: 40%;
  transform: translate(-50%, -50%);
  z-index: 2;
  padding: 10px 20px;
}

/* the empty room and the invite */
/* these two have nothing else around them, so they keep a panel to stay readable */
.empty,
.invite {
  max-width: 460px;
  margin: 12% auto 0;
  padding: 24px 28px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  line-height: 1.5;
}

.empty h2 {
  letter-spacing: 0.25em;
  color: var(--gold);
}

.hint {
  color: var(--text-dim);
  font-size: 0.85rem;
}

.problem {
  color: var(--warn);
  font-size: 0.85rem;
}

.center {
  text-align: center;
  margin-top: 20%;
}

button {
  font: inherit;
  color: var(--text);
  background: var(--hover);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 6px 14px;
  cursor: pointer;
}

button.primary,
.unveil {
  background: var(--accent);
  color: #07142e;
  border-color: transparent;
}

button:disabled {
  opacity: 0.45;
  cursor: default;
}

@media (max-width: 1100px) {
  .hero {
    grid-template-columns: auto 1fr;
  }

  .tiles {
    grid-column: 1 / -1;
  }

  .cards {
    grid-template-columns: 1fr;
  }
}
</style>
