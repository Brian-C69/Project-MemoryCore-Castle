<script setup>
// The Trade log (page 4) and the Lessons (page 5): one card per companion turn, and one row per
// lesson it kept from a closed trade, newest first, each with Delete.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import GlassSelect from '../GlassSelect.vue'
import { marketLessonDelete, marketLessons, marketTurns, onMarketChanged } from '../../api/castle.js'
import { actionsOf, chipKind, chipOf, percent, tagOf, turnCounts, unfinishedOf } from './turns.js'

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
// one turn opened in the ledger at a time
const openId = ref(null)
const counts = computed(() => turnCounts(turns.value))
const toggle = (id) => (openId.value = openId.value === id ? null : id)
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
      <div class="panel top">
        <GlassSelect v-model="runId" :options="[{ value: null, label: 'This account' }, ...accounts.map((a) => ({ value: a.id, label: `Account ${a.runNo}` }))]" aria-label="Account" />
        <span class="pages">
          <button :disabled="page === 0" aria-label="Newer turns" @click="page--">‹</button>
          <button :disabled="turns.length < 20" aria-label="Older turns" @click="page++">›</button>
        </span>
      </div>

      <div v-if="turns.length" class="counters">
        <div class="panel counter"><h4>{{ page > 0 ? 'TURNS ON THIS PAGE' : 'TURNS' }}</h4><b>{{ counts.turns }}</b></div>
        <div class="panel counter"><h4>WENT THROUGH / REFUSED</h4><b><span class="ok">{{ counts.done }}</span> <i>/</i> <span class="no">{{ counts.refused }}</span></b></div>
        <div class="panel counter"><h4>NO TRADE</h4><b>{{ counts.noTrade }}</b></div>
      </div>

      <div v-if="turns.length" class="panel ledger">
        <div class="ledger-row head" aria-hidden="true">
          <span>TIME</span><span>WHO STARTED IT</span><span>OUTCOME</span><span>RESULT</span><span>REPLY</span><span></span>
        </div>
        <div v-for="t in turns" :key="t.id" class="entry" :class="{ open: openId === t.id }">
          <button class="ledger-row" :aria-expanded="openId === t.id" @click="toggle(t.id)">
            <time>{{ when(t.createdAt) }}</time>
            <span><span class="tag" :class="{ auto: t.triggerKind === 'AUTO' }">{{ tagOf(t) }}</span></span>
            <span class="outcome">
              <template v-if="actionsOf(t).length">
                <span class="chip" :class="chipKind(actionsOf(t)[0])">{{ chipOf(actionsOf(t)[0]) }}</span>
                <span v-if="actionsOf(t).length > 1" class="more">+{{ actionsOf(t).length - 1 }}</span>
              </template>
              <span v-else-if="unfinishedOf(t)" class="chip none">{{ unfinishedOf(t) }}</span>
              <span v-else class="chip none">No trade</span>
            </span>
            <span class="result">
              <span v-if="actionsOf(t).length" class="pill" :class="actionsOf(t)[0].ok ? 'ok' : 'no'" :title="actionsOf(t)[0].result">{{ actionsOf(t)[0].result }}</span>
              <span v-else class="dim">—</span>
            </span>
            <span class="reply">{{ t.reply || (t.message ? `“${t.message}”` : '') }}</span>
            <svg class="chev" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
          </button>
          <div v-if="openId === t.id" class="detail">
            <div v-if="t.message" class="part">
              <h4>YOUR WORDS</h4>
              <p>“{{ t.message }}”</p>
            </div>
            <div class="part">
              <h4>{{ companion.toUpperCase() }}’S REPLY</h4>
              <p>{{ t.reply || unfinishedOf(t) || '—' }}</p>
              <ul v-if="actionsOf(t).length" class="acts">
                <li v-for="(a, i) in actionsOf(t)" :key="i"><span class="chip" :class="chipKind(a)">{{ chipOf(a) }}</span><span :class="a.ok ? 'ok' : 'no'">● {{ a.result }}</span></li>
              </ul>
            </div>
          </div>
        </div>
      </div>
      <p v-else class="dim empty">{{ companion }}'s turns appear here.</p>
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
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: var(--scrollbar-width);
}

/* ---------- the Trade log: toolbar, counters, ledger ---------- */
.top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
}

.pages {
  display: flex;
  gap: 6px;
}

.top button {
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-background);
  color: var(--text);
  font: inherit;
}

.counters {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.counter {
  padding: 14px 18px;
}

h4 {
  margin: 0 0 6px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}

.counter b {
  font-size: 22px;
  font-weight: 500;
}

.counter i {
  font-style: normal;
  color: var(--text-dim);
}

.ledger {
  display: flex;
  flex-direction: column;
  padding: 4px 0;
}

.ledger-row {
  display: grid;
  grid-template-columns: 90px 140px minmax(150px, 220px) minmax(90px, 170px) minmax(0, 1fr) 20px;
  align-items: center;
  gap: 14px;
  width: 100%;
  padding: 12px 20px;
  border: 0;
  border-top: 1px solid var(--line);
  background: none;
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.ledger-row.head {
  border-top: 0;
  font-size: 11px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
  cursor: default;
}

.ledger-row time {
  color: var(--text);
}

.ledger-row:not(.head):hover {
  background: color-mix(in srgb, var(--accent) 6%, transparent);
}

.ledger-row:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.entry.open {
  margin: 0 6px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent) 6%, transparent);
}

.entry.open .ledger-row {
  border-top-color: transparent;
}

.entry.open + .entry .ledger-row {
  border-top-color: transparent;
}

.outcome {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.outcome .chip {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.more {
  font-size: 12px;
  color: var(--text-dim);
}

.result {
  min-width: 0;
}

.pill {
  display: inline-block;
  max-width: 100%;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: middle;
}

.pill.ok {
  background: color-mix(in srgb, var(--good) 22%, transparent);
}

.pill.no {
  background: color-mix(in srgb, var(--bad) 22%, transparent);
}

.ledger-row .reply {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-dim);
}

.chev {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: var(--text-dim);
  stroke-width: 2;
  transition: transform 0.15s;
}

.entry.open .chev {
  transform: rotate(180deg);
}

.detail {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 18px;
  margin: 0 14px 14px;
  padding: 16px 20px;
  border: 1px solid var(--line);
  border-radius: 10px;
}

.detail .part + .part {
  border-left: 1px solid var(--line);
  padding-left: 18px;
}

.detail p {
  margin: 0;
  line-height: 1.5;
}

.acts {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 12px 0 0;
  padding: 0;
  list-style: none;
}

.acts li {
  display: flex;
  align-items: center;
  gap: 10px;
}

@media (max-width: 900px) {
  .counters {
    grid-template-columns: 1fr;
  }

  .ledger-row {
    grid-template-columns: 70px minmax(0, 1fr) minmax(0, 1fr) 20px;
  }

  .ledger-row > :nth-child(2),
  .ledger-row > :nth-child(5) {
    display: none;
  }

  .detail .part + .part {
    border-left: 0;
    padding-left: 0;
  }
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
