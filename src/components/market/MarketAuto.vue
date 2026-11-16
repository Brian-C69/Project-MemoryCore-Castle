<script setup>
// Settings > COMPANION TRADES BY ITSELF (page 6): the switch, how often, how many rounds a day, the
// daily loss limit and the targets, today's rounds and result, and why it stopped. Off until the
// member turns it on, and off again whenever Castle starts.
import { onMounted, ref } from 'vue'
import { marketAutoGet, marketAutoSave, marketStop } from '../../api/castle.js'
import { signedUsd, tone } from './market.js'

const props = defineProps({ companion: { type: String, default: 'Companion' } })

const on = ref(false)
const every = ref(15)
const cap = ref(20)
const loss = ref(500)
const target = ref('')
const balanceTarget = ref('')
const view = ref(null)
const problem = ref('')
const saved = ref('')

async function load() {
  try {
    view.value = await marketAutoGet()
    const s = view.value.setting
    on.value = s.autoOn
    every.value = s.autoEveryMinutes
    cap.value = s.autoRoundCap
    loss.value = s.autoDailyLoss
    target.value = s.autoDailyTarget ?? ''
    balanceTarget.value = s.autoBalanceTarget ?? ''
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

const num = (v) => (String(v ?? '').trim() === '' ? null : Number(v))

async function save() {
  problem.value = ''
  saved.value = ''
  try {
    await marketAutoSave({ on: on.value, every: Number(every.value), cap: Number(cap.value), loss: Number(loss.value), target: num(target.value), balanceTarget: num(balanceTarget.value) })
    saved.value = 'Saved.'
    await load()
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
    await load()
  }
}

async function stopNow() {
  try {
    await marketStop()
    await load()
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

onMounted(load)
</script>

<template>
  <article class="panel auto">
    <header>
      <h3>{{ companion.toUpperCase() }} TRADES BY ITSELF</h3>
      <label class="switch"><input v-model="on" type="checkbox" @change="save" /> {{ on ? 'On' : 'Off' }}</label>
    </header>
    <label>A round every <span><input v-model="every" inputmode="numeric" @change="save" /> minutes</span></label>
    <label>Rounds a day <span><input v-model="cap" inputmode="numeric" @change="save" /></span></label>
    <label>Stop at a daily loss of <span>USD <input v-model="loss" inputmode="decimal" @change="save" /></span></label>
    <label>Daily profit target <span>USD <input v-model="target" inputmode="decimal" placeholder="—" @change="save" /></span></label>
    <label>Balance target <span>USD <input v-model="balanceTarget" inputmode="decimal" placeholder="—" @change="save" /></span></label>
    <p v-if="view" class="today">
      <span class="dot" :class="{ live: view.setting.autoOn }"></span>
      Today: {{ view.roundsToday }} round{{ view.roundsToday === 1 ? '' : 's' }}, <b :class="tone(view.resultToday)">{{ signedUsd(view.resultToday) }}</b>
      <button v-if="view.setting.autoOn || view.thinking" class="stop" @click="stopNow">Stop</button>
    </p>
    <p v-if="view?.stoppedWhy && !view.setting.autoOn" class="why">{{ view.stoppedWhy }}</p>
    <p class="dim">Turns itself off when Castle closes. Each round uses one turn of your own Claude Code or Codex.</p>
    <p v-if="problem" class="problem">{{ problem }}</p>
    <p v-else-if="saved" class="saved">{{ saved }}</p>
  </article>
</template>

<style scoped>
.auto {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 20px;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

h3 {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

label {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  color: var(--text-dim);
}

label input:not([type='checkbox']) {
  width: 90px;
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
  text-align: right;
}

.switch {
  gap: 8px;
  color: var(--text);
}

.today {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
}

.dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--text-dim);
}

.dot.live {
  background: var(--good);
  box-shadow: 0 0 8px var(--good);
}

.stop {
  margin-left: auto;
  padding: 4px 12px;
  border: 1px solid var(--bad);
  border-radius: 8px;
  background: none;
  color: var(--bad);
  font: inherit;
  cursor: pointer;
}

.gain {
  color: var(--good);
}

.loss {
  color: var(--bad);
}

.why {
  margin: 0;
  color: var(--warn);
  font-size: 12.5px;
}

.dim,
.saved {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}

.problem {
  margin: 0;
  color: var(--warn);
}
</style>
