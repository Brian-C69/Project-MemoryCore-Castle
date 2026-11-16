<script setup>
// MARKET > Settings, design 3: a status strip (trading by itself on or off, today's rounds and
// result, the heart tick), then RHYTHM and LIMITS beside the view's TRADING RULES (the `rules`
// slot). Off until the member turns it on, and off again whenever Castle starts. The heart tick is
// MARKET's own switch for the emotion engine, apart from the chat room's.
import { onMounted, ref } from 'vue'
import { getSettings, marketAutoGet, marketAutoSave, marketStop, saveSettings } from '../../api/castle.js'
import { signedUsd, tone } from './market.js'

const props = defineProps({ companion: { type: String, default: 'Companion' } })

const on = ref(false)
const every = ref(15)
const cap = ref(20)
const loss = ref(500)
const target = ref('')
const targetOn = ref(false)
const balanceTarget = ref('')
const balanceOn = ref(false)
const heart = ref(true)
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
    targetOn.value = s.autoDailyTarget != null
    target.value = s.autoDailyTarget ?? ''
    balanceOn.value = s.autoBalanceTarget != null
    balanceTarget.value = s.autoBalanceTarget ?? ''
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
}

async function loadHeart() {
  try {
    heart.value = (await getSettings()).settings.emotionMarketVoice !== false
  } catch {
    // the tick keeps its default; saving it says any problem
  }
}

const num = (v) => (String(v ?? '').trim() === '' ? null : Number(v))

async function save() {
  problem.value = ''
  saved.value = ''
  try {
    await marketAutoSave({
      on: on.value,
      every: Number(every.value),
      cap: Number(cap.value),
      loss: Number(loss.value),
      target: targetOn.value ? num(target.value) : null,
      balanceTarget: balanceOn.value ? num(balanceTarget.value) : null,
    })
    saved.value = 'Saved.'
    await load()
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
    await load()
  }
}

// A target's tick: on keeps (or waits for) its figure, off clears it.
function toggleTarget(which, checked) {
  if (which === 'daily') targetOn.value = checked
  else balanceOn.value = checked
  if (!checked || num(which === 'daily' ? target.value : balanceTarget.value) != null) save()
}

async function saveHeart(checked) {
  problem.value = ''
  saved.value = ''
  try {
    // read fresh first: Settings saves the same file, and an older copy here would undo it
    const fresh = (await getSettings()).settings
    heart.value = (await saveSettings({ ...fresh, emotionMarketVoice: checked })).settings.emotionMarketVoice !== false
    saved.value = 'Saved.'
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
    await loadHeart()
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

onMounted(() => {
  load()
  loadHeart()
})
</script>

<template>
  <div class="auto">
    <section class="panel strip" aria-label="Trading by itself">
      <div class="by-itself">
        <h3>{{ companion.toUpperCase() }} TRADES BY ITSELF</h3>
        <label class="switch" :title="on ? 'On' : 'Off'">
          <input v-model="on" type="checkbox" role="switch" @change="save" />
          <span class="track"><span class="knob"></span></span>
        </label>
      </div>
      <p v-if="view" class="today">
        <span class="dot" :class="{ live: view.setting.autoOn }"></span>
        Today: {{ view.roundsToday }} round{{ view.roundsToday === 1 ? '' : 's' }}, <b :class="tone(view.resultToday)">{{ signedUsd(view.resultToday) }}</b>
        <button v-if="view.setting.autoOn || view.thinking" class="stop" @click="stopNow">Stop</button>
      </p>
      <label class="heart">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 21s-7.5-4.6-9.6-9.2C.9 8.4 3 4.5 6.8 4.5c2.1 0 3.6 1.2 4.2 2.4h2c.6-1.2 2.1-2.4 4.2-2.4 3.8 0 5.9 3.9 4.4 7.3C19.5 16.4 12 21 12 21z" /></svg>
        <input type="checkbox" :checked="heart" @change="saveHeart($event.target.checked)" />
        Let {{ companion }}'s emotion engine colour the desk
      </label>
    </section>

    <div class="cards">
      <article class="panel card">
        <h3>RHYTHM</h3>
        <label>A round every <span><input v-model="every" inputmode="numeric" @change="save" /> minutes</span></label>
        <label>Rounds a day <span><input v-model="cap" inputmode="numeric" @change="save" /></span></label>
        <p class="dim">Each round uses one turn of your own Claude Code or Codex. Turns itself off when Castle closes.</p>
      </article>

      <article class="panel card">
        <h3>LIMITS</h3>
        <label>
          <span class="name"><input type="checkbox" checked disabled title="Always on - the safety rail for trading by itself" /> Daily loss stop</span>
          <span>USD <input v-model="loss" inputmode="decimal" @change="save" /></span>
        </label>
        <label :class="{ off: !targetOn }">
          <span class="name"><input type="checkbox" :checked="targetOn" @change="toggleTarget('daily', $event.target.checked)" /> Daily profit target</span>
          <span>USD <input v-model="target" inputmode="decimal" :disabled="!targetOn" placeholder="—" @change="save" /></span>
        </label>
        <label :class="{ off: !balanceOn }">
          <span class="name"><input type="checkbox" :checked="balanceOn" @change="toggleTarget('balance', $event.target.checked)" /> Balance target</span>
          <span>USD <input v-model="balanceTarget" inputmode="decimal" :disabled="!balanceOn" placeholder="—" @change="save" /></span>
        </label>
        <p v-if="view?.stoppedWhy && !view.setting.autoOn" class="why">{{ view.stoppedWhy }}</p>
      </article>

      <slot name="rules" />
    </div>

    <p v-if="problem" class="problem">{{ problem }}</p>
    <p v-else-if="saved" class="saved">{{ saved }}</p>
  </div>
</template>

<style scoped>
.auto {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

h3 {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

/* ---------- the status strip ---------- */
.strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 14px 22px;
  padding: 14px 20px;
}

.by-itself {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-right: 22px;
  border-right: 1px solid var(--line);
}

.switch {
  position: relative;
  display: inline-flex;
  cursor: pointer;
}

.switch input {
  position: absolute;
  opacity: 0;
  inset: 0;
  margin: 0;
  cursor: pointer;
}

.track {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.12);
  border: 1px solid var(--line);
  transition: background 0.15s;
}

.knob {
  display: block;
  width: 18px;
  height: 18px;
  margin: 2px;
  border-radius: 50%;
  background: var(--text);
  transition: transform 0.15s;
}

.switch input:checked + .track {
  background: color-mix(in srgb, var(--accent) 70%, transparent);
}

.switch input:checked + .track .knob {
  transform: translateX(18px);
}

.switch input:focus-visible + .track {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.today {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 15px;
  color: var(--text);
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
  margin-left: 6px;
  padding: 4px 12px;
  border: 1px solid var(--bad);
  border-radius: 8px;
  background: none;
  color: var(--bad);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}

.heart {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-left: auto;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
}

.heart svg {
  width: 20px;
  height: 20px;
  fill: var(--accent);
  filter: drop-shadow(0 0 6px color-mix(in srgb, var(--accent) 60%, transparent));
}

/* ---------- the cards ---------- */
.cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
  align-items: stretch;
}

.card,
.cards :slotted(article) {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 20px;
}

label {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--text-dim);
}

.name {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

label.off input:not([type='checkbox']) {
  opacity: 0.45;
}

input[type='checkbox'] {
  accent-color: var(--accent);
  width: 16px;
  height: 16px;
  margin: 0;
}

input[type='checkbox']:disabled {
  opacity: 0.8;
  cursor: not-allowed;
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

.card .dim {
  margin-top: auto;
}

.problem {
  margin: 0;
  color: var(--warn);
}

@media (max-width: 1100px) {
  .cards {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .cards {
    grid-template-columns: 1fr;
  }

  .by-itself {
    padding-right: 0;
    border-right: none;
  }

  .heart {
    margin-left: 0;
  }
}
</style>
