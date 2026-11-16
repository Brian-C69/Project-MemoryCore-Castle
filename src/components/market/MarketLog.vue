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

    <template v-else>
      <h3>WHAT {{ companion.toUpperCase() }} LEARNED FROM EACH CLOSED TRADE</h3>
      <div v-for="l in lessons" :key="l.id" class="panel row">
        <span class="dim">{{ day(l.createdAt) }}</span>
        <span class="sym">{{ l.symbol }}</span>
        <span :class="l.resultPct >= 0 ? 'ok' : 'no'">{{ percent(l.resultPct) }}</span>
        <span class="tag" :class="{ auto: l.tradedBy === 'COMPANION' }">{{ l.tradedBy === 'COMPANION' ? companion : 'You' }}</span>
        <span class="lesson">{{ l.lesson }}</span>
        <button class="trash" aria-label="Delete this lesson" title="Delete" @click="remove(l.id)">🗑</button>
      </div>
      <p v-if="!lessons.length" class="dim empty">A lesson appears after each closed trade.</p>
      <p v-else class="dim foot">The newest 12 ride into every decision.</p>
    </template>
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

.row {
  display: grid;
  grid-template-columns: 60px 110px 70px auto 1fr 32px;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
}

.sym {
  padding: 3px 10px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  text-align: center;
}

.lesson {
  line-height: 1.4;
}

.trash {
  border: 0;
  background: none;
  cursor: pointer;
  opacity: 0.7;
}

.empty,
.foot {
  text-align: center;
}

.problem {
  color: var(--warn);
}
</style>
