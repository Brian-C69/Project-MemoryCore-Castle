<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { calendarEventDelete, calendarEventGet, calendarLinkJobs, calendarEventMove, calendarEventSave, calendarEventSkip, calendarIcsDiscard, calendarIcsExport, calendarIcsImport, calendarIcsPreview, calendarProposalDecide, calendarProposals, calendarRange } from '../api/castle.js'
import { SHOWN_KINDS, addDays, agendaDays, cellItems, dayTitle, filterItems, groupByDay, gridRange, iso, itemLine, itemMark, monthGrid, monthTitle, ownerRoom, shiftMonth, weekdayNames } from '../calendar.js'
import { canDrag, colourVar, emptyForm, formFromQuick, movedStart, proposalLine, toEvent, toForm, weekOf, weekTitle } from '../calendarEvents.js'
import { calendarSeed, refreshCalendarCard } from '../composables/useCalendar.js'
import EventForm from '../components/calendar/EventForm.vue'
import IcsPanel from '../components/calendar/IcsPanel.vue'
import SubsPanel from '../components/calendar/SubsPanel.vue'
import QuickAdd from '../components/calendar/QuickAdd.vue'
import WeekGrid from '../components/calendar/WeekGrid.vue'
import '../components/automations/roomcard.css'

const emit = defineEmits(['open'])
const now = new Date()
const today = ref(iso(now))
const year = ref(now.getFullYear())
const month = ref(now.getMonth())
const selected = ref(today.value)
const weekStart = 1 // Monday; a Settings choice may come later
const items = ref([])
const error = ref('')
const loaded = ref(false)
const width = ref(1000)
const root = ref(null)
const busy = ref(false)
const form = ref(null)
const formOccurrence = ref('')
const formError = ref('')
const proposals = ref([])
// a calendar file: { mode: 'preview', preview } or { mode: 'export' }
const ics = ref(null)
// the holidays panel (calendars subscribed to)
const holidays = ref(false)
const icsError = ref('')
const notice = ref('')
const jobs = ref(null)
async function readJobs() { try { jobs.value = await calendarLinkJobs() } catch { jobs.value = [] } }

// remembered choices (per viewer only; the room works without them)
const KEY = 'castle-calendar'
function remembered() { try { return JSON.parse(localStorage.getItem(KEY) || '{}') || {} } catch { return {} } }
function remember() { try { localStorage.setItem(KEY, JSON.stringify({ view: view.value, filters: filters.value })) } catch { /* private window */ } }
const saved = remembered()
const filters = ref({ holiday: true, event: true, reminder: true, task: true, automation: true, diary: true, ...(saved.filters || {}) })
const view = ref(['month', 'week', 'agenda'].includes(saved.view) ? saved.view : '')
const narrow = computed(() => width.value < 700)
const shownView = computed(() => view.value || (narrow.value ? 'agenda' : 'month'))
watch([view, filters], remember, { deep: true })

const weeks = computed(() => monthGrid(year.value, month.value, weekStart))
const names = computed(() => weekdayNames(weekStart))
const week = computed(() => weekOf(selected.value, weekStart))
const shown = computed(() => filterItems(items.value, filters.value))
const byDay = computed(() => groupByDay(shown.value))
const agenda = computed(() => agendaDays(shown.value, today.value, 30))
const dayItems = computed(() => byDay.value[selected.value] || [])

function windowFor() {
  if (shownView.value === 'agenda') return { from: today.value, to: addDays(today.value, 29) }
  if (shownView.value === 'week') return { from: week.value.from, to: week.value.to }
  return gridRange(weeks.value)
}

// the start-up loader already read this month: show it at once, then refresh
const seed = calendarSeed.value
if (seed && shownView.value === 'month' && seed.day === today.value) { items.value = seed.items; loaded.value = true }

let token = 0
async function load() {
  const mine = ++token
  const { from, to } = windowFor()
  try {
    const got = await calendarRange(from, to)
    if (mine !== token) return
    items.value = got
    error.value = ''
    proposals.value = await calendarProposals().catch(() => proposals.value)
  } catch (e) {
    if (mine !== token) return
    error.value = e?.message || String(e)
  } finally {
    if (mine === token) loaded.value = true
  }
}
watch([year, month, shownView], load)
watch(() => week.value.from, () => { if (shownView.value === 'week') load() })

function step(by) {
  if (shownView.value === 'week') { pick(addDays(selected.value, by * 7)); return }
  const m = shiftMonth(year.value, month.value, by)
  year.value = m.year
  month.value = m.month
}
function goToday() {
  today.value = iso(new Date())
  const d = new Date()
  year.value = d.getFullYear()
  month.value = d.getMonth()
  selected.value = today.value
  load()
}
function pick(date) {
  selected.value = date
  const y = Number(date.slice(0, 4))
  const m = Number(date.slice(5, 7)) - 1
  if (y !== year.value || m !== month.value) { year.value = y; month.value = m }
}
function toggle(kind) { filters.value = { ...filters.value, [kind]: filters.value[kind] === false } }

// ---- events ----
function newEvent(date = selected.value, time = '') {
  readJobs()
  form.value = emptyForm(date, time)
  formOccurrence.value = ''
  formError.value = ''
}
async function open(item) {
  if (item.kind !== 'event') {
    const room = ownerRoom(item)
    if (room) emit('open', room)
    return
  }
  busy.value = true
  readJobs()
  try {
    form.value = toForm(await calendarEventGet(item.id))
    formOccurrence.value = item.occurrence || ''
    formError.value = ''
    pick(item.date)
  } catch (e) { error.value = e?.message || String(e) } finally { busy.value = false }
}
async function work(job, onFail) {
  busy.value = true
  try { await job(); error.value = ''; await load(); return true } catch (e) { (onFail || ((m) => { error.value = m }))(e?.message || String(e)); return false } finally { busy.value = false }
}
async function save() {
  const ok = await work(async () => { await calendarEventSave(toEvent(form.value)); }, (m) => { formError.value = m })
  if (ok) form.value = null
}
async function remove() {
  const ok = await work(() => calendarEventDelete(form.value.id), (m) => { formError.value = m })
  if (ok) form.value = null
}
async function skip() {
  const ok = await work(() => calendarEventSkip(form.value.id, formOccurrence.value, false), (m) => { formError.value = m })
  if (ok) form.value = null
}
// calendar files (.ics)
async function icsStart() {
  form.value = null
  holidays.value = false
  notice.value = ''
  icsError.value = ''
  busy.value = true
  try {
    const preview = await calendarIcsPreview()
    if (preview) ics.value = { mode: 'preview', preview }
  } catch (e) { error.value = e?.message || String(e) } finally { busy.value = false }
}
function openHolidays() { form.value = null; ics.value = null; notice.value = ''; holidays.value = true }
function icsOpenExport() { form.value = null; holidays.value = false; notice.value = ''; icsError.value = ''; ics.value = { mode: 'export' } }
async function icsCancel() { ics.value = null; icsError.value = ''; await calendarIcsDiscard().catch(() => {}) }
async function icsConfirm() {
  busy.value = true
  icsError.value = ''
  try {
    const s = await calendarIcsImport(ics.value.preview.token)
    notice.value = `Imported ${s.imported} event${s.imported === 1 ? '' : 's'}${s.duplicates ? ` (${s.duplicates} already there)` : ''}${s.refused ? `, ${s.refused} could not be added` : ''}.`
    ics.value = null
    await load()
  } catch (e) { icsError.value = e?.message || String(e) } finally { busy.value = false }
}
async function icsExport(includeDated) {
  busy.value = true
  icsError.value = ''
  try {
    const r = await calendarIcsExport(includeDated)
    if (r) { notice.value = `Saved ${r.count} item${r.count === 1 ? '' : 's'} to ${r.path}.`; ics.value = null }
  } catch (e) { icsError.value = e?.message || String(e) } finally { busy.value = false }
}

// the companion's suggestions: they do nothing until the member says yes here
function openProposal(p) {
  readJobs()
  form.value = toForm(p)
  formOccurrence.value = ''
  formError.value = ''
  pick(p.startsAt.split(' ')[0])
}
async function decide(id, approve, event = null) {
  const ok = await work(async () => { await calendarProposalDecide(id, approve, event); refreshCalendarCard() }, (m) => { formError.value = m })
  if (ok && form.value?.id === id) form.value = null
  return ok
}
const approveForm = () => decide(form.value.id, true, toEvent(form.value))
const dismissForm = () => decide(form.value.id, false)

async function quickAdd(q) {
  const f = formFromQuick(q, selected.value)
  await work(async () => { await calendarEventSave(toEvent(f)); if (q.date) pick(q.date) })
}
function more(q) {
  readJobs()
  form.value = q ? formFromQuick(q, selected.value) : emptyForm(selected.value, '')
  formOccurrence.value = ''
  formError.value = ''
}
async function moveTo(dragged, start) {
  await work(() => calendarEventMove(dragged.id, dragged.occurrence || null, start))
}

// dragging an event onto another day (month) keeps its time of day
function dragStart(e, item) {
  e.dataTransfer.setData('application/x-castle-event', JSON.stringify({ id: item.id, occurrence: item.occurrence, allDay: item.allDay, time: item.time }))
  e.dataTransfer.effectAllowed = 'move'
}
function dropOnDay(e, date) {
  const raw = e.dataTransfer.getData('application/x-castle-event')
  if (!raw) return
  try { const item = JSON.parse(raw); moveTo(item, movedStart(item, date)) } catch { /* not ours */ }
}

function keys(e) {
  if (e.target?.closest?.('input, textarea, select, [contenteditable], [role="listbox"]') || e.ctrlKey || e.metaKey || e.altKey) return
  const move = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 }[e.key]
  if (move) { pick(addDays(selected.value, move)); e.preventDefault() }
  else if (e.key === 't') goToday()
  else if (e.key === 'm') view.value = 'month'
  else if (e.key === 'w') view.value = 'week'
  else if (e.key === 'a') view.value = 'agenda'
  else if (e.key === 'n') { newEvent(); e.preventDefault() }
  else if (e.key === 'Escape' && form.value) form.value = null
}

let timer, watcher
onMounted(() => {
  load()
  timer = setInterval(() => { if (!form.value) load() }, 60000)
  window.addEventListener('keydown', keys)
  if (root.value && typeof ResizeObserver !== 'undefined') {
    watcher = new ResizeObserver(([entry]) => { width.value = entry.contentRect.width })
    watcher.observe(root.value)
  }
})
onBeforeUnmount(() => { clearInterval(timer); window.removeEventListener('keydown', keys); watcher?.disconnect() })

const kindLabel = (kind) => SHOWN_KINDS.find((k) => k.id === kind)?.label.replace(/s$/, '') || kind
const title = computed(() => (shownView.value === 'week' ? weekTitle(week.value.days) : monthTitle(year.value, month.value)))
</script>

<template>
<section ref="root" class="calendar-room">
 <header class="bar">
  <h2>CALENDAR</h2><span class="spacer"></span>
  <button @click="newEvent()">New event</button>
  <button @click="openHolidays">Holidays</button>
  <button :disabled="busy" @click="icsStart">Import .ics</button>
  <button @click="icsOpenExport">Export .ics</button>
  <button :aria-pressed="shownView === 'month'" :class="{ on: shownView === 'month' }" @click="view = 'month'">Month</button>
  <button :aria-pressed="shownView === 'week'" :class="{ on: shownView === 'week' }" @click="view = 'week'">Week</button>
  <button :aria-pressed="shownView === 'agenda'" :class="{ on: shownView === 'agenda' }" @click="view = 'agenda'">Agenda</button>
 </header>
 <p class="lede">Everything in Castle that has a date, on one page. Your own events are made and changed here; reminders, project tasks, diary pages and automation runs are shown here and changed where they live. Arrow keys move the day, <kbd>n</kbd> is a new event, <kbd>t</kbd> today, <kbd>m</kbd> <kbd>w</kbd> <kbd>a</kbd> the view.</p>
 <div class="filters" role="group" aria-label="Show">
  <button v-for="k in SHOWN_KINDS" :key="k.id" :class="['chip', k.id, { on: filters[k.id] !== false }]" :aria-pressed="filters[k.id] !== false" @click="toggle(k.id)">{{ k.label }}</button>
 </div>
 <p v-if="error" class="error" role="alert">{{ error }}</p>
 <p v-if="notice" class="notice" role="status">{{ notice }}</p>
 <section v-if="proposals.length" class="castle-room-card proposals" aria-label="Suggested by your companion">
  <h3>Suggested by your companion</h3>
  <p class="hint">These do nothing, and no reminder rings, until you say yes.</p>
  <ul>
   <li v-for="p in proposals" :key="p.id">
    <span class="what"><span class="title">{{ proposalLine(p) }}</span><span v-if="p.note" class="sub">{{ p.note }}</span></span>
    <span class="acts">
     <button :disabled="busy" @click="decide(p.id, true)">Approve</button>
     <button :disabled="busy" @click="openProposal(p)">Edit</button>
     <button :disabled="busy" @click="decide(p.id, false)">Dismiss</button>
    </span>
   </li>
  </ul>
 </section>

 <div v-if="shownView !== 'agenda'" class="body">
  <div class="castle-room-card main">
   <div class="nav">
    <button aria-label="Previous" @click="step(-1)">&lt;</button>
    <h3>{{ title }}</h3>
    <button aria-label="Next" @click="step(1)">&gt;</button>
    <span class="spacer"></span>
    <button @click="goToday">Today</button>
   </div>
   <template v-if="shownView === 'month'">
    <div class="grid head" aria-hidden="true"><span v-for="n in names" :key="n">{{ n }}</span></div>
    <div v-for="(wk, w) in weeks" :key="w" class="grid">
     <div v-for="cell in wk" :key="cell.date" :class="['cell', { out: !cell.inMonth, today: cell.date === today, sel: cell.date === selected }]" tabindex="0" role="button" :aria-label="dayTitle(cell.date)" @click="pick(cell.date)" @keydown.enter="pick(cell.date)" @dragover.prevent @drop.prevent="dropOnDay($event, cell.date)">
      <span class="num">{{ cell.day }}</span>
      <template v-for="part in [cellItems(byDay[cell.date])]" :key="cell.date">
       <span v-for="(item, n) in part.shown" :key="n" :class="['entry', item.kind, item.state]" :style="{ '--tone': colourVar(item.colour) }" :title="itemLine(item)" :draggable="canDrag(item)" @dragstart="dragStart($event, item)" @click.stop="item.kind === 'event' ? open(item) : pick(cell.date)"><b aria-hidden="true">{{ itemMark(item) }}</b> {{ itemLine(item) }}</span>
       <span v-if="part.more" class="more">+{{ part.more }} more</span>
      </template>
     </div>
    </div>
   </template>
   <WeekGrid v-else :days="week.days" :items="shown" :today="today" :selected="selected" @select="pick" @open="open" @move="moveTo" @add="(d, t) => newEvent(d, t)" />
  </div>
  <aside class="castle-room-card day" aria-live="polite">
   <EventForm v-if="form" :form="form" :jobs="jobs" :occurrence="formOccurrence" :busy="busy" :error="formError" @save="save" @cancel="form = null" @delete="remove" @skip="skip" @approve="approveForm" @dismiss="dismissForm" />
   <IcsPanel v-else-if="ics" :mode="ics.mode" :preview="ics.preview || null" :busy="busy" :error="icsError" @confirm="icsConfirm" @cancel="icsCancel" @export="icsExport" />
   <SubsPanel v-else-if="holidays" @changed="load" @close="holidays = false" />
   <template v-else>
    <h3>{{ dayTitle(selected) }}</h3>
    <QuickAdd :day="selected" :busy="busy" @add="quickAdd" @more="more" />
    <p v-if="loaded && !dayItems.length" class="none">Nothing is dated this day.</p>
    <ul v-else>
     <li v-for="(item, n) in dayItems" :key="n">
      <button :class="['row', item.kind, item.state]" :style="{ '--tone': colourVar(item.colour) }" :disabled="item.kind !== 'event' && !ownerRoom(item)" @click="open(item)">
       <b aria-hidden="true">{{ itemMark(item) }}</b>
       <span class="what"><span class="title">{{ itemLine(item) }}</span><span class="sub">{{ kindLabel(item.kind) }}<template v-if="item.detail"> · {{ item.detail }}</template><template v-if="item.repeating"> · repeats</template></span></span>
      </button>
     </li>
    </ul>
   </template>
  </aside>
 </div>

 <div v-else class="body">
  <div class="castle-room-card agenda">
   <h3>The next 30 days</h3>
   <p v-if="loaded && !agenda.length" class="none">Nothing is dated in the next 30 days.</p>
   <section v-for="day in agenda" :key="day.date" class="agenda-day">
    <h4>{{ dayTitle(day.date) }}<span v-if="day.date === today"> · today</span></h4>
    <ul>
     <li v-for="(item, n) in day.items" :key="n">
      <button :class="['row', item.kind, item.state]" :style="{ '--tone': colourVar(item.colour) }" :disabled="item.kind !== 'event' && !ownerRoom(item)" @click="open(item)">
       <b aria-hidden="true">{{ itemMark(item) }}</b>
       <span class="what"><span class="title">{{ itemLine(item) }}</span><span class="sub">{{ kindLabel(item.kind) }}<template v-if="item.detail"> · {{ item.detail }}</template><template v-if="item.repeating"> · repeats</template></span></span>
      </button>
     </li>
    </ul>
   </section>
  </div>
  <aside v-if="form || ics || holidays" class="castle-room-card day">
   <SubsPanel v-if="holidays && !form && !ics" @changed="load" @close="holidays = false" />
   <IcsPanel v-if="ics && !form" :mode="ics.mode" :preview="ics.preview || null" :busy="busy" :error="icsError" @confirm="icsConfirm" @cancel="icsCancel" @export="icsExport" />
   <EventForm v-if="form" :form="form" :jobs="jobs" :occurrence="formOccurrence" :busy="busy" :error="formError" @save="save" @cancel="form = null" @delete="remove" @skip="skip" @approve="approveForm" @dismiss="dismissForm" /></aside>
 </div>
</section>
</template>

<style scoped>
.calendar-room {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: auto;
  color: var(--text);
  font-size: 13px;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
.bar, .nav, .filters { display: flex; align-items: center; gap: 12px; }
.bar, .filters { flex-wrap: wrap; }
.bar h2 { margin: 0; font-size: 13px; font-weight: 500; letter-spacing: 0.2em; color: var(--accent); }
.spacer { flex: 1; }
.lede { margin: 0; max-width: 60em; color: var(--text-dim); line-height: 1.5; }
kbd { padding: 0 5px; border: 1px solid var(--line); border-radius: 5px; font: inherit; font-size: 12px; }
.error { margin: 0; color: #ef9393; }
.notice { margin: 0; color: var(--text); overflow-wrap: anywhere; }
h3 { margin: 0; font-size: 11px; font-weight: 500; letter-spacing: 0.16em; color: var(--accent); }
h4 { margin: 0 0 6px; font-size: 13px; font-weight: 600; }
.body { display: flex; gap: 16px; align-items: flex-start; flex-wrap: wrap; }
.main { flex: 3 1 560px; min-width: 0; display: flex; flex-direction: column; gap: 6px; }
.day { flex: 1 1 280px; min-width: 260px; display: flex; flex-direction: column; gap: 12px; }
.nav { margin-bottom: 6px; }
.grid { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 4px; }
.grid.head span { padding: 2px 6px; font-size: 11px; letter-spacing: 0.1em; color: var(--text-dim); }
.cell {
  display: flex; flex-direction: column; gap: 2px; min-height: 92px; min-width: 0; padding: 6px;
  border: 1px solid var(--line); border-radius: 10px; cursor: pointer; overflow: hidden;
}
.cell:hover { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
.cell.out { opacity: 0.5; }
.cell.today .num { display: inline-flex; align-items: center; justify-content: center; min-width: 22px; height: 22px; border: 1.5px solid var(--accent); border-radius: 50%; color: var(--accent); }
.cell.sel { background: color-mix(in srgb, var(--accent) 12%, transparent); border-color: var(--accent); }
.num { align-self: flex-start; padding: 0 3px; font-size: 12px; color: var(--text-dim); }
.entry { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11.5px; line-height: 1.35; }
.entry.event { border-left: 3px solid var(--tone, var(--accent)); padding-left: 3px; border-radius: 3px; }
.entry.event[draggable='true'] { cursor: grab; }
.entry.holiday, .row.holiday { color: color-mix(in srgb, var(--accent) 70%, var(--text)); }
.entry.holiday { border-left: 3px double var(--accent); padding-left: 3px; border-radius: 3px; }
.row.holiday { border-style: dotted; }
.entry.proposed, .row.proposed { border: 1px dashed var(--accent); background: none; }
.entry.proposed { padding-left: 3px; border-radius: 3px; }
.proposals { display: flex; flex-direction: column; gap: 8px; }
.proposals .hint { margin: 0; font-size: 12px; color: var(--text-dim); }
.proposals li { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; padding: 8px 10px; border: 1px dashed var(--accent); border-radius: 10px; }
.acts { display: flex; gap: 8px; flex-wrap: wrap; }
.proposals button { padding: 6px 12px; border: 1px solid var(--line); border-radius: 10px; background: none; color: var(--text); font: inherit; cursor: pointer; }
.proposals button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
.proposals button:disabled { opacity: 0.45; cursor: default; }
.entry b, .row b { display: inline-block; width: 1em; text-align: center; color: var(--tone, var(--accent)); }
.entry.done, .row.done { opacity: 0.6; text-decoration: line-through; text-decoration-color: color-mix(in srgb, var(--text) 40%, transparent); }
.entry.diary, .row.diary { text-decoration: none; opacity: 0.85; }
.entry.failed, .row.failed, .entry.failed b, .row.failed b { color: #ef9393; }
.more { font-size: 11px; color: var(--text-dim); }
.none { margin: 0; color: var(--text-dim); }
ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
.row {
  display: flex; align-items: flex-start; gap: 8px; width: 100%; text-align: left; padding: 8px 10px;
  border: 1px solid var(--line); border-radius: 10px; background: none; color: var(--text); font: inherit; cursor: pointer;
}
.row.event { border-left: 3px solid var(--tone, var(--accent)); }
.row:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
.row:disabled { cursor: default; }
.what { display: grid; gap: 2px; min-width: 0; }
.title { overflow-wrap: anywhere; }
.sub { font-size: 11.5px; color: var(--text-dim); }
.agenda { flex: 2 1 420px; display: flex; flex-direction: column; gap: 14px; max-width: 760px; }
.agenda-day { display: block; }
.chip, .bar button, .nav button {
  padding: 8px 14px; border: 1px solid var(--line); border-radius: 10px;
  background: none; color: var(--text); font: inherit; cursor: pointer;
}
.chip { padding: 5px 12px; border-radius: 20px; font-size: 12px; opacity: 0.6; }
.chip.on, .bar button.on { opacity: 1; border-color: var(--accent); color: var(--accent); }
.chip:hover, .bar button:hover, .nav button:hover { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
button:focus-visible, .cell:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
@media (max-width: 700px) { .cell { min-height: 64px; } .entry { font-size: 10.5px; } }
</style>
