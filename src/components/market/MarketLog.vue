<script setup>
// The Trade log (page 4) and the Lessons (page 5): one card per companion turn, and one row per
// lesson it kept from a closed trade, newest first, each with Delete.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { marketLessonDelete, marketLessons, marketTurns, onMarketChanged } from '../../api/castle.js'
import { actionsOf, chipKind, chipOf, percent, tagOf } from './turns.js'

const props = defineProps({
  view: { type: String, required: true },
  companion: { type: String, default: 'Companion' },
  runs: { type: Function, required: true },
})

const turns = ref([])
const lessons = ref([])
const accounts = ref([])
const runId = ref(null)
const page = ref(0)
const problem = ref('')
let stop

async function load() {
  problem.value = ''
  try {
    if (props.view === 'lessons') lessons.value = await marketLessons()
    else {
      accounts.value = await props.runs()
      turns.value = await marketTurns(runId.value, page.value)
    }
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

async function remove(id) {
  try {
    await marketLessonDelete(id)
    lessons.value = lessons.value.filter((l) => l.id !== id)
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

watch(() => [props.view, runId.value, page.value], load)
onMounted(async () => {
  load()
  try {
    stop = await onMarketChanged(load)
  } catch {
    // outside Castle's window
  }
})
onBeforeUnmount(() => stop?.())

const when = (ms) => new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
// the newest lessons that ride into every desk turn (DESK_LESSONS on the Rust side)
const RIDE = 12
const day = (ms) => new Date(ms).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
</script>

<template>
  <section class="log">
    <p v-if="problem" class="problem">{{ problem }}</p>

    <template v-if="view === 'log'">
      <div class="top">
        <select v-model="runId" aria-label="Account">
          <option :value="null">This account</option>
          <option v-for="a in accounts" :key="a.id" :value="a.id">Account {{ a.runNo }}</option>
        </select>
        <button :disabled="page === 0" @click="page--">‹</button>
        <button :disabled="turns.length < 20" @click="page++">›</button>
      </div>
      <article v-for="t in turns" :key="t.id" class="panel card">
        <header><time>{{ when(t.createdAt) }}</time><span class="tag" :class="{ auto: t.triggerKind === 'AUTO' }">{{ tagOf(t) }}</span><span v-if="t.status !== 'DONE'" class="dim">{{ t.status === 'RUNNING' ? 'Thinking…' : t.status === 'STOPPED' ? 'Stopped' : 'Did not finish' }}</span></header>
        <p v-if="t.message" class="said">“{{ t.message }}”</p>
        <p v-if="t.reply" class="reply">{{ t.reply }}</p>
        <div class="actions">
          <template v-if="actionsOf(t).length">
            <span v-for="(a, i) in actionsOf(t)" :key="i" class="act"><span class="chip" :class="chipKind(a)">{{ chipOf(a) }}</span><span :class="a.ok ? 'ok' : 'no'">● {{ a.result }}</span></span>
          </template>
          <span v-else-if="t.status === 'DONE'" class="chip none">No trade</span>
        </div>
      </article>
      <p v-if="!turns.length" class="dim empty">{{ companion }}'s turns appear here.</p>
    </template>

    <section v-else class="panel lessons" aria-label="Lessons">
      <header class="lessons-head">
        <h3>WHAT {{ companion.toUpperCase() }} LEARNED FROM EACH CLOSED TRADE</h3>
        <p v-if="lessons.length" class="rides-note"><span class="bracket"></span>The newest {{ RIDE }} ride into every decision.</p>
      </header>
      <ol v-if="lessons.length" class="timeline">
        <li v-for="(l, i) in lessons" :key="l.id" :class="{ rides: i < RIDE }">
          <span class="dot" :class="l.resultPct >= 0 ? 'up' : 'down'" aria-hidden="true"></span>
          <article class="bubble">
            <div class="facts">
              <time>{{ day(l.createdAt) }}</time>
              <span class="sym">{{ l.symbol }}</span>
              <span class="side">{{ l.side }}</span>
              <span :class="l.resultPct >= 0 ? 'ok' : 'no'">{{ percent(l.resultPct) }}</span>
            </div>
            <p class="lesson">{{ l.lesson }}</p>
            <span class="tag" :class="{ auto: l.tradedBy === 'COMPANION' }">{{ l.tradedBy === 'COMPANION' ? companion : 'You' }}</span>
            <button class="trash" aria-label="Delete this lesson" title="Delete" @click="remove(l.id)">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 3h6l1 2h4v2H4V5h4l1-2zm-3 6h12l-1 12H7L6 9zm4 2v8h2v-8h-2zm4 0v8h2v-8h-2z" /></svg>
            </button>
          </article>
        </li>
      </ol>
      <p v-else class="dim empty">A lesson appears after each closed trade.</p>
    </section>
  </section>
</template>

<style scoped>
.log {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
  scrollbar-color: var(--line) transparent;
  scrollbar-width: thin;
}

.top {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}

.top select,
.top button {
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-background);
  color: var(--text);
  font: inherit;
}

.card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 18px;
}

.card header {
  display: flex;
  align-items: center;
  gap: 10px;
}

time,
.dim {
  color: var(--text-dim);
}

.tag {
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  background: color-mix(in srgb, var(--accent) 30%, transparent);
}

.tag.auto {
  background: color-mix(in srgb, var(--accent-2, var(--accent)) 18%, transparent);
  border: 1px solid var(--line);
}

.said {
  margin: 0;
  font-size: 15px;
}

.reply {
  margin: 0;
  color: var(--text-dim);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 10px;
}

.act {
  display: flex;
  align-items: center;
  gap: 10px;
}

.chip {
  padding: 3px 10px;
  border-radius: 8px;
  font-size: 13px;
}

.chip.buy {
  background: color-mix(in srgb, var(--good) 30%, transparent);
}

.chip.sell {
  background: color-mix(in srgb, var(--bad) 30%, transparent);
}

.chip.close,
.chip.none {
  background: color-mix(in srgb, var(--accent) 18%, transparent);
}

.ok {
  color: var(--good);
}

.no {
  color: var(--bad);
}

h3 {
  margin: 0;
  font-size: 12px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.lessons {
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 20px 24px;
}

.lessons-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 20px;
}

.rides-note {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 0;
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  font-size: 12.5px;
  color: var(--text-dim);
}

.bracket {
  width: 4px;
  height: 18px;
  border-radius: 2px;
  background: var(--accent);
  box-shadow: 0 0 8px var(--accent);
}

.timeline {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.timeline li {
  position: relative;
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr);
  align-items: start;
}

/* the line: lavender and glowing where the lessons ride into decisions, quiet after */
.timeline li::before {
  content: '';
  position: absolute;
  left: 16px;
  top: 0;
  bottom: -14px;
  width: 2px;
  background: var(--line);
}

.timeline li:last-child::before {
  bottom: 50%;
}

.timeline li.rides::before {
  background: color-mix(in srgb, var(--accent) 75%, transparent);
  box-shadow: 0 0 6px color-mix(in srgb, var(--accent) 60%, transparent);
}

.dot {
  position: relative;
  z-index: 1;
  width: 14px;
  height: 14px;
  margin: 22px 0 0 10px;
  border-radius: 50%;
}

.dot.up {
  background: var(--good);
  box-shadow: 0 0 8px var(--good);
}

.dot.down {
  background: var(--bad);
  box-shadow: 0 0 8px var(--bad);
}

.bubble {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  grid-template-areas:
    'facts tag trash'
    'lesson lesson lesson';
  align-items: center;
  gap: 8px 12px;
  padding: 14px 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: color-mix(in srgb, var(--panel-background) 70%, transparent);
}

.facts {
  grid-area: facts;
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px 20px;
  font-size: 14px;
  letter-spacing: 0.04em;
}

.facts time {
  text-transform: uppercase;
}

.sym,
.side {
  color: var(--text);
}

.bubble .tag {
  grid-area: tag;
}

.lesson {
  grid-area: lesson;
  margin: 0;
  line-height: 1.45;
}

.trash {
  grid-area: trash;
  display: inline-flex;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text-dim);
  cursor: pointer;
}

.trash svg {
  width: 16px;
  height: 16px;
  fill: currentColor;
}

.trash:hover {
  color: var(--bad);
  border-color: var(--bad);
}

.empty {
  text-align: center;
}

.problem {
  color: var(--warn);
}
</style>
