<script>
// One reading shared by the four credit widgets: asked on mount, every five minutes while the page
// is visible, and whenever Castle says a figure changed (a page read, a typed figure, a budget).
import { ref } from 'vue'
import { creditsRead, onCreditsChanged } from '../../api/castle.js'
import { monthNow } from './credits.js'

const shared = ref([])
const sharedNow = ref(Date.now())
let users = 0
let timer
let ticker
let stopListening
let latest = 0

async function readShared() {
  sharedNow.value = Date.now()
  if (document.visibilityState !== 'visible') return
  const mine = ++latest
  try {
    const { month, start } = monthNow()
    const answer = await creditsRead(month, start)
    if (mine === latest) shared.value = answer
  } catch {
    // The last reading stays.
  }
}

async function join() {
  users += 1
  if (users > 1) return
  readShared()
  timer = setInterval(readShared, 5 * 60 * 1000)
  ticker = setInterval(() => (sharedNow.value = Date.now()), 30000)
  try {
    stopListening = await onCreditsChanged(readShared)
  } catch {
    // outside Castle's window
  }
}

function leave() {
  users -= 1
  if (users > 0) return
  clearInterval(timer)
  clearInterval(ticker)
  stopListening?.()
  stopListening = null
}
</script>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted } from 'vue'
import { creditsBudget, creditsReadNow, creditsSet, creditsSignIn } from '../../api/castle.js'
import { barOf, dimLine, nameOf, parseUsd, providerOf, valueLine, wantsSignIn } from './credits.js'

const props = defineProps({ widget: { type: String, required: true } })
// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const provider = computed(() => providerOf(props.widget))
const credit = computed(() => shared.value.find((c) => c.provider === provider.value) ?? { provider: provider.value, value: null })
const bar = computed(() => barOf(credit.value))
const readsPage = computed(() => provider.value === 'openai' || provider.value === 'seedance')

// The typed backup ("set") and the Seedance budget, typed in place.
const editing = ref('')
const typed = ref('')
const problem = ref('')
const field = ref(null)

async function edit(what) {
  problem.value = ''
  editing.value = what
  typed.value = what === 'budget' && credit.value.whole ? String(credit.value.whole) : ''
  await nextTick()
  field.value?.focus()
}

async function commit() {
  const what = editing.value
  if (!what) return
  const text = typed.value.trim()
  const amount = parseUsd(text)
  if (what === 'budget' && text === '') {
    editing.value = ''
    return run(() => creditsBudget(null))
  }
  if (amount === null) {
    problem.value = 'Type an amount in USD, like 12.50.'
    return
  }
  editing.value = ''
  if (what === 'budget') return run(() => creditsBudget(amount))
  const { month } = monthNow()
  return run(() => creditsSet(provider.value, amount, provider.value === 'seedance' ? month : null))
}

async function run(call) {
  try {
    await call()
    problem.value = ''
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

function signIn() {
  run(() => creditsSignIn(provider.value))
}

async function readNow() {
  if (!readsPage.value || !credit.value.state) return
  try {
    const started = await creditsReadNow(provider.value)
    problem.value = started ? '' : 'Read a minute ago - try again shortly.'
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

onMounted(join)
onBeforeUnmount(leave)
</script>

<template>
  <section class="widget credit" :class="provider">
    <h3>{{ nameOf(provider) }}</h3>
    <button v-if="readsPage && credit.state" type="button" class="value" :class="{ low: bar?.low }" title="Read it again now" @click="readNow">{{ valueLine(credit) }}</button>
    <p v-else class="value" :class="{ low: bar?.low }">{{ valueLine(credit) }}</p>
    <div class="track" :class="{ none: !bar }" aria-hidden="true">
      <span v-if="bar" class="fill" :class="{ low: bar.low }" :style="{ width: `${Math.round(bar.fill * 100)}%` }" />
    </div>
    <form v-if="editing" class="typed" @submit.prevent="commit">
      <input ref="field" v-model="typed" :placeholder="editing === 'budget' ? 'Monthly budget, USD' : provider === 'seedance' ? 'Spent this month, USD' : 'Balance, USD'" @keydown.esc="editing = ''" @blur="commit" />
    </form>
    <p v-else class="dim" :title="dimLine(credit, sharedNow)">{{ dimLine(credit, sharedNow) }}</p>
    <p v-if="problem" class="problem">{{ problem }}</p>
    <div v-if="readsPage" class="actions">
      <button v-if="wantsSignIn(credit)" type="button" @click="signIn">Sign in ↗</button>
      <button type="button" @click="edit('set')">set ✎</button>
      <button v-if="provider === 'seedance'" type="button" @click="edit('budget')">budget ✎</button>
    </div>
  </section>
</template>

<style scoped>
.credit {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 150px;
  padding: 14px 16px 12px;
}

h3 {
  margin: 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.value {
  margin: 0;
  padding: 0;
  border: 0;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 24px;
  font-weight: 500;
  line-height: 1.15;
  text-align: left;
  white-space: nowrap;
}

button.value {
  cursor: pointer;
}

.value.low {
  color: var(--warn, #ffb37a);
}

.track {
  height: 5px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--line) 70%, transparent);
  overflow: hidden;
}

.track.none {
  visibility: hidden;
}

.fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
  transition: width 0.6s ease;
}

.credit.suno .fill,
.credit.elevenlabs .fill {
  background: var(--accent-2, #8cc8ff);
}

.fill.low {
  background: var(--warn, #ffb37a);
}

.dim,
.problem {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.4;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.problem {
  color: var(--warn, #ffd59a);
  white-space: normal;
}

.typed input {
  width: 100%;
  box-sizing: border-box;
  padding: 3px 6px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
  font-size: 12px;
}

.actions {
  display: flex;
  gap: 10px;
}

.actions button {
  padding: 0;
  border: 0;
  background: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 11px;
  cursor: pointer;
}

.actions button:hover {
  color: var(--text);
}
</style>
