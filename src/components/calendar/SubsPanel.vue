<script setup>
// Holidays and other calendars the member subscribes to. Castle fetches each one once a day and keeps a
// copy; they are only shown, never turned into events or reminders. A failed update is a quiet line,
// not an error wall, and the last copy stays on screen.
import { computed, onMounted, ref } from 'vue'
import { calendarSubAdd, calendarSubRefresh, calendarSubRemove, calendarSubSet, calendarSubs } from '../../api/castle.js'
import { agoText } from '../../calendar.js'
import GlassSelect from '../GlassSelect.vue'
import HelpTip from '../HelpTip.vue'
import SettingConfirm from '../settings/SettingConfirm.vue'
import SettingSwitch from '../settings/SettingSwitch.vue'

const emit = defineEmits(['changed', 'close'])
const info = ref({ subs: [], presets: [], states: [] })
const busy = ref(false)
const error = ref('')
const more = ref(false)
const name = ref('')
const url = ref('')

async function work(job) {
  busy.value = true
  error.value = ''
  try {
    const got = await job()
    if (got) info.value = got
    emit('changed')
  } catch (e) { error.value = e?.message || String(e) } finally { busy.value = false }
}
onMounted(() => work(() => calendarSubs()))

const offered = computed(() => info.value.presets.filter((p) => !info.value.subs.some((s) => s.preset === p.id)))
const stateOptions = computed(() => [{ value: 'none', label: 'Nationwide days only' }, ...info.value.states.map((s) => ({ value: s, label: s }))])
const status = (s) => (s.lastOkAt ? `Updated ${agoText(s.lastOkAt)} · ${s.entries} entries` : s.lastError ? 'Not fetched yet' : 'Waiting for the first update')
const add = (preset) => work(() => calendarSubAdd({ preset }))
async function addOwn() {
  await work(() => calendarSubAdd({ name: name.value, url: url.value }))
  if (!error.value) { name.value = ''; url.value = ''; more.value = false }
}
</script>

<template>
  <div class="subs">
    <h3>Holidays and calendars</h3>
    <p class="hint">Castle fetches each calendar once a day and keeps a copy. They are only shown here.<HelpTip block text="Only the web address is contacted; nothing about you is sent, and a calendar can never create an event, a reminder or an instruction. If an update fails, the last copy stays and Castle tries again later." /></p>
    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <ul v-if="info.subs.length" class="list">
      <li v-for="s in info.subs" :key="s.id">
        <div class="top">
          <strong>{{ s.name }}</strong>
          <SettingSwitch :model-value="s.enabled" :label="`Show ${s.name}`" :disabled="busy" @update:model-value="(v) => work(() => calendarSubSet(s.id, v, s.region))" />
        </div>
        <p class="status">{{ status(s) }}</p>
        <p v-if="s.lastError" class="quiet">Could not update: {{ s.lastError }}<template v-if="s.lastOkAt"> The last copy is still shown.</template> Castle will try again.</p>
        <label v-if="s.preset === 'malaysia'" class="state">My state
          <GlassSelect :model-value="s.region || 'none'" :options="stateOptions" aria-label="My state" :disabled="busy" @update:model-value="(v) => work(() => calendarSubSet(s.id, s.enabled, v === 'none' ? '' : v))" />
        </label>
        <p v-if="s.preset === 'malaysia'" class="hint">Its own state's holidays show with the nationwide days.<HelpTip block text="From Google Calendar's public Holidays in Malaysia. It lists public holidays and other observances together, nationwide and by state, and some dates are marked as tentative. Religious dates can change: check the official announcement." /></p>
        <div class="buttons">
          <button type="button" :disabled="busy" @click="work(() => calendarSubRefresh(s.id))">Update now</button>
          <SettingConfirm label="Remove" :disabled="busy" @confirm="work(() => calendarSubRemove(s.id))" />
        </div>
      </li>
    </ul>

    <div v-for="p in offered" :key="p.id" class="offer">
      <button type="button" :disabled="busy" @click="add(p.id)">Add {{ p.name }}</button>
    </div>
    <button v-if="!more" type="button" class="plain" :disabled="busy || info.subs.length >= 5" @click="more = true">Add another calendar by its web address</button>
    <form v-else class="own" @submit.prevent="addOwn">
      <label>Name <input v-model="name" type="text" maxlength="80" /></label>
      <label>Web address <input v-model="url" type="url" placeholder="https://..." /></label>
      <div class="buttons">
        <button type="submit" :disabled="busy || !name.trim() || !url.trim()">Add and update</button>
        <button type="button" :disabled="busy" @click="more = false">Cancel</button>
      </div>
    </form>
    <div class="buttons"><button type="button" @click="emit('close')">Done</button></div>
  </div>
</template>

<style scoped>
.subs { display: grid; gap: 12px; }
h3 { margin: 0; font-size: 11px; font-weight: 500; letter-spacing: 0.16em; color: var(--accent); }
.hint { margin: 0; font-size: 12px; color: var(--text-dim); line-height: 1.4; }
.error { margin: 0; color: #ef9393; }
.quiet { margin: 0; font-size: 12px; color: var(--text-dim); line-height: 1.4; }
.status { margin: 0; font-size: 12px; color: var(--text-dim); }
ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
.list li { display: grid; gap: 8px; padding: 10px; border: 1px solid var(--line); border-radius: 10px; }
.top { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.top strong { overflow-wrap: anywhere; }
.state { display: grid; gap: 4px; font-size: 12px; color: var(--text-dim); }
.own { display: grid; gap: 10px; }
.own label { display: grid; gap: 4px; font-size: 12px; color: var(--text-dim); }
input { width: 100%; box-sizing: border-box; padding: 9px 10px; color: var(--text); background: var(--panel-background); border: 1px solid var(--line); border-radius: 8px; font: inherit; }
input:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.buttons { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
button:where(:not(.danger)) { padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px; background: none; color: var(--text); font: inherit; cursor: pointer; }
button.plain { align-self: flex-start; border-style: dashed; }
button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
button:disabled { opacity: 0.45; cursor: default; }
button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
