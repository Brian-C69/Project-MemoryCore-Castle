<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  projectDbChange,
  projectDbColumns,
  projectDbConnection,
  projectDbDatabases,
  projectDbDump,
  projectDbExport,
  projectDbFinish,
  projectDbForget,
  projectDbImport,
  projectDbLook,
  projectDbPickCa,
  projectDbSave,
  projectDbSaved,
  projectDbSavedSet,
  projectDbTableRows,
  projectDbTables,
  projectDbTest,
} from '../../api/castle.js'
import { isMac } from '../../platform.js'
import GlassSelect from '../../components/GlassSelect.vue'
import { chipOf, filterTables, formOf, formToSave, fromWords, kindOf, rowsLabel, savedWith, savedWithout, secondsLeft, shortHost, tableLook, updateFor } from './projectdb.js'

// A DEV project's own database, wide beside the chat. Each side is set up once in the panel itself
// (Local is pre-filled from the project's .env), then it shows. Local (teal) edits freely;
// Production (amber) opens in LOOK, read only, until the member unlocks Change - and even then a
// write waits for their Keep or Undo, and a change to a table's shape asks twice.
const props = defineProps({
  slot: { type: Number, required: true },
  shown: { type: Boolean, default: true },
})

const side = ref('local')
const production = computed(() => side.value === 'production')
const unlocked = ref(false) // Change mode on Production
const database = ref('')
const target = computed(() => ({ slot: props.slot, side: side.value, database: database.value }))

const connection = ref(null) // { project, local, production }
const view = computed(() => connection.value?.[side.value] ?? null)
const blocked = computed(() => view.value?.blocked ?? '')

// ---------- the connection form ----------

const editing = ref(false)
const form = ref(formOf(null, 'local'))
const formProblem = ref('')
const formNote = ref('')
const saving = ref(false)
// the form shows when this side has nothing it can open with, or the member asked for it
const showForm = computed(() => editing.value || (!!connection.value && !!blocked.value))

function openForm() {
  form.value = formOf(view.value, side.value)
  formProblem.value = ''
  formNote.value = ''
  editing.value = true
}

async function chooseCa() {
  try {
    const path = await projectDbPickCa()
    if (path) form.value = { ...form.value, caPath: path }
  } catch (reason) {
    formProblem.value = String(reason)
  }
}

async function saveForm() {
  const { form: sending, problem: why } = formToSave(form.value, side.value, !!view.value?.hasPassword)
  if (why) {
    formProblem.value = why
    return
  }
  saving.value = true
  formProblem.value = ''
  formNote.value = ''
  try {
    await projectDbSave(props.slot, side.value, sending)
    const version = await projectDbTest({ slot: props.slot, side: side.value, database: '' })
    formNote.value = `Connected${version ? ` - ${version}` : ''}.`
    editing.value = false
    database.value = ''
    await readPlace()
  } catch (reason) {
    // saved, but it did not open: the form stays, with the reason
    formProblem.value = String(reason)
    await readConnection()
    form.value = { ...form.value, password: '' }
    editing.value = true
  } finally {
    saving.value = false
  }
}

async function forgetSide() {
  try {
    await projectDbForget(props.slot, side.value)
    editing.value = false
    database.value = ''
    await readPlace()
    form.value = formOf(view.value, side.value)
  } catch (reason) {
    formProblem.value = String(reason)
  }
}

// ---------- the database ----------

const databases = ref([])
const tables = ref([])
const rows = ref({})
const tableFilter = ref('')
const shownTables = computed(() => filterTables(tables.value, tableFilter.value))
const table = ref('')
const columns = ref([])
const sql = ref('')
const result = ref(null)
const problem = ref('')
const working = ref(false)
const asking = ref(null) // { sql, kind, note } waiting for the member's yes
const held = ref(null) // { id, affected, since }
const now = ref(Date.now())
const saved = ref([])
const naming = ref(false)
const savedName = ref('')
const cell = ref(null) // { r, c, value }
const note = ref('')
let clock

const canChange = computed(() => !production.value || unlocked.value)

async function readConnection() {
  connection.value = await projectDbConnection(props.slot)
}

// The tables, then their row estimates beside them (an estimate that cannot be read leaves no count).
async function readTables() {
  tables.value = await projectDbTables(target.value)
  rows.value = await projectDbTableRows(target.value).catch(() => ({}))
}
const rowsTitle = (name) => (rows.value[name] != null ? `${name} - about ${Number(rows.value[name]).toLocaleString('en-US')} rows (MySQL's estimate)` : name)

async function readPlace() {
  problem.value = ''
  tables.value = []
  rows.value = {}
  databases.value = []
  tableFilter.value = ''
  result.value = null
  try {
    await readConnection()
    if (blocked.value) {
      form.value = formOf(view.value, side.value)
      return
    }
    if (!database.value && view.value?.database) database.value = view.value.database
    databases.value = await projectDbDatabases(target.value).catch(() => [])
    if (database.value) await readTables()
  } catch (reason) {
    problem.value = String(reason)
  }
}

watch(side, () => {
  unlocked.value = false
  editing.value = false
  database.value = ''
  table.value = ''
  readPlace()
})
watch(database, async (db, before) => {
  if (!db || db === before || !connection.value || blocked.value) return
  try {
    await readTables()
  } catch (reason) {
    problem.value = String(reason)
  }
})

async function openTable(name) {
  table.value = name
  sql.value = tableLook(name)
  columns.value = await projectDbColumns(target.value, name).catch(() => [])
  run()
}

async function run() {
  const text = sql.value.trim()
  if (!text || working.value) return
  problem.value = ''
  note.value = ''
  const kind = kindOf(text)
  if (kind === 'read') return look(text)
  if (production.value && !unlocked.value) {
    problem.value = 'Production is in LOOK - read only. Unlock Change mode to run a change.'
    return
  }
  asking.value = {
    sql: text,
    kind,
    note:
      kind === 'shape'
        ? `This changes the shape of ${database.value || 'the database'} on ${production.value ? 'Production' : 'Local'}. MySQL cannot undo it. Run it?`
        : production.value
          ? `Run this on Production (${database.value})? You can keep or undo it for two minutes.`
          : `Run this on Local (${database.value})?`,
  }
}

async function look(text) {
  working.value = true
  try {
    result.value = await projectDbLook(target.value, text)
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    working.value = false
  }
}

async function confirmRun() {
  const ask = asking.value
  asking.value = null
  if (!ask) return
  working.value = true
  try {
    // the yes on this card is the second yes a shape change needs
    const done = await projectDbChange(target.value, ask.sql, ask.kind === 'shape')
    result.value = { columns: [], rows: [], affected: done.affected, ms: done.ms }
    if (done.id) held.value = { id: done.id, affected: done.affected, since: Date.now() }
    else if (table.value) look(tableLook(table.value))
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    working.value = false
  }
}

async function finish(keep) {
  const h = held.value
  held.value = null
  if (!h) return
  try {
    await projectDbFinish(h.id, keep)
    note.value = keep ? `Kept: ${h.affected} ${h.affected === 1 ? 'row' : 'rows'} changed.` : 'Undone - nothing changed.'
    if (table.value) look(tableLook(table.value))
  } catch (reason) {
    problem.value = String(reason)
  }
}

// a grid cell edited: its UPDATE is shown and asked like any change
function startEdit(r, c) {
  if (!canChange.value || !result.value?.columns?.length) return
  cell.value = { r, c, value: result.value.rows[r][c] ?? '' }
}
function commitEdit() {
  const e = cell.value
  cell.value = null
  if (!e) return
  const before = result.value.rows[e.r][e.c]
  if ((before ?? '') === e.value) return
  const update = updateFor(table.value, columns.value, result.value.columns, result.value.rows[e.r], e.c, e.value)
  if (!update) {
    problem.value = 'This grid has no primary key column, so a cell cannot be changed safely here - write the UPDATE yourself.'
    return
  }
  sql.value = update
  run()
}

async function exportRows() {
  if (!result.value?.columns?.length) return
  try {
    const path = await projectDbExport(result.value, table.value || 'rows')
    if (path) note.value = `Exported to ${path}`
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function dump(whole) {
  try {
    const done = await projectDbDump(target.value, whole ? null : table.value || null)
    if (done) note.value = `Dumped ${done[1]} rows to ${done[0]}`
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function importFile() {
  try {
    const done = await projectDbImport(target.value, table.value || null, unlocked.value)
    if (done) {
      note.value = `Imported ${done[0]}: ${done[1]} ${done[0].toLowerCase().endsWith('.csv') ? 'rows' : 'statements'}.`
      await readTables()
    }
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function readSaved() {
  try {
    saved.value = await projectDbSaved(props.slot)
  } catch {
    saved.value = []
  }
}
async function keepSaved() {
  saved.value = savedWith(saved.value, savedName.value, sql.value)
  naming.value = false
  savedName.value = ''
  await projectDbSavedSet(props.slot, saved.value).catch((reason) => (problem.value = String(reason)))
}
async function dropSaved(name) {
  saved.value = savedWithout(saved.value, name)
  await projectDbSavedSet(props.slot, saved.value).catch((reason) => (problem.value = String(reason)))
}

function keys(event) {
  if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
    event.preventDefault()
    run()
  }
}

onMounted(() => {
  readPlace()
  readSaved()
  clock = setInterval(() => {
    now.value = Date.now()
    // the core undoes it after two minutes; the panel lets go of it at the same moment
    if (held.value && secondsLeft(held.value.since, now.value) === 0) {
      held.value = null
      note.value = 'Two minutes passed - the change was undone.'
    }
  }, 1000)
})
onBeforeUnmount(() => {
  clearInterval(clock)
  if (held.value) projectDbFinish(held.value.id, false).catch(() => {})
})
</script>

<template>
  <aside class="database" :class="side" aria-label="Database">
    <nav class="tabs" role="tablist">
      <button role="tab" :class="{ on: side === 'local' }" :aria-selected="side === 'local'" @click="side = 'local'">Local</button>
      <button role="tab" :class="{ on: side === 'production' }" :aria-selected="side === 'production'" @click="side = 'production'">Production</button>
    </nav>

    <!-- set up once: Local pre-filled from the project's files, Production filled by the member -->
    <form v-if="showForm" class="setup" @submit.prevent="saveForm">
      <h4>{{ production ? 'Production database' : 'Local database' }}</h4>
      <p class="dim">
        <template v-if="production">The live database. Fill it in once; it stays on this computer, the password with {{ isMac ? 'the Keychain' : 'Windows Credential Manager' }}.</template>
        <template v-else-if="view && view.from !== 'none' && view.from !== 'saved'">Found in {{ fromWords(view.from) }} - change anything here to use your own.</template>
        <template v-else>The database on this computer. Nothing was found in the project's .env, so fill it in once.</template>
      </p>
      <div class="fields">
        <label class="wide">Host <input v-model="form.host" spellcheck="false" :placeholder="production ? 'db-mysql-sgp1-12345.db.ondigitalocean.com' : '127.0.0.1'" /></label>
        <label>Port <input v-model.number="form.port" type="number" min="1" max="65535" /></label>
        <label>Database <input v-model="form.database" spellcheck="false" placeholder="app" /></label>
        <label>User <input v-model="form.user" spellcheck="false" autocomplete="off" /></label>
        <label>
          Password
          <input v-model="form.password" type="password" autocomplete="new-password" :placeholder="view?.hasPassword ? 'kept - leave empty to keep it' : ''" />
        </label>
        <div v-if="production" class="wide ca">
          <span>CA certificate</span>
          <button type="button" class="small" @click="chooseCa">Choose file…</button>
          <span class="dim">{{ form.caPath ? form.caPath.split(/[\\/]/).pop() : view?.ca ? 'kept' : 'none - TLS still on; add the cluster\'s ca-certificate.crt to check the server' }}</span>
        </div>
      </div>
      <p v-if="formProblem" class="bad">{{ formProblem }}</p>
      <div class="buttons">
        <button v-if="view?.from === 'saved'" type="button" class="quiet" @click="forgetSide">Forget</button>
        <span class="spacer"></span>
        <button v-if="editing && !blocked" type="button" class="quiet" @click="editing = false">Cancel</button>
        <button type="submit" class="go" :disabled="saving">{{ saving ? 'Connecting…' : 'Save and connect' }}</button>
      </div>
    </form>

    <template v-else-if="connection">
      <div class="where">
        <span class="dot" aria-hidden="true"></span>
        <code class="name">{{ database || '(no database)' }}</code>
        <span class="sep" aria-hidden="true"></span>
        <code class="host" :title="view?.host || ''">{{ shortHost(view?.host) || '?' }}</code>
        <GlassSelect v-if="databases.length" v-model="database" class="db-pick" :options="databases" placeholder="choose a database" searchable aria-label="Database" />
        <span class="push"></span>
        <span v-if="!production" class="chip teal">edits allowed</span>
        <template v-else>
          <span class="chip look" :class="{ off: unlocked }">{{ unlocked ? 'CHANGE - unlocked' : 'LOOK - read only' }}</span>
          <button class="lock" type="button" :class="{ on: unlocked }" @click="unlocked = !unlocked">
            <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="11" width="14" height="9" rx="2" /><path :d="unlocked ? 'M8 11V8a4 4 0 0 1 7.5-2' : 'M8 11V8a4 4 0 0 1 8 0v3'" /></svg>
            {{ unlocked ? 'Lock again' : 'Change mode' }}
          </button>
        </template>
        <button class="lock" type="button" title="Change this side's login" @click="openForm">Edit connection</button>
      </div>
      <p class="where-more">
        <span>login from {{ fromWords(view?.from) }}<template v-if="view?.user"> · {{ view.user }}</template></span>
        <span v-if="production && !view?.ca">no CA certificate kept - the server is not checked</span>
        <span v-if="formNote">{{ formNote }}</span>
      </p>

      <div class="work">
        <div class="side-list">
          <label class="filter">
            <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6.5" /><path d="M16 16l4.5 4.5" /></svg>
            <input v-model="tableFilter" type="search" placeholder="Filter tables" aria-label="Filter tables" spellcheck="false" />
          </label>
          <ul class="tables" aria-label="Tables">
            <li v-if="!tables.length" class="none">{{ database ? 'No tables.' : 'Choose a database.' }}</li>
            <li v-else-if="!shownTables.length" class="none">No table matches.</li>
            <li v-for="t in shownTables" :key="t">
              <button :class="{ on: t === table }" :title="rowsTitle(t)" @click="openTable(t)">
                <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="5" width="17" height="14" rx="1.5" /><path d="M3.5 10h17M3.5 14.5h17M9.5 5v14" /></svg>
                <span class="tname">{{ t }}</span>
                <span v-if="rowsLabel(rows[t])" class="count">{{ rowsLabel(rows[t]) }}</span>
              </button>
            </li>
          </ul>
        </div>

        <div class="main">
          <div class="query">
            <textarea v-model="sql" spellcheck="false" placeholder="SELECT * FROM ... (Ctrl+Enter runs)" aria-label="Query" @keydown="keys"></textarea>
            <button class="run" type="button" :disabled="working || !sql.trim()" @click="run">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 5l12 7-12 7z" /></svg>{{ working ? 'Running…' : 'Run' }}
            </button>
          </div>

          <p v-if="asking" class="ask" :class="asking.kind">
            <span>{{ asking.note }}</span>
            <code>{{ asking.sql }}</code>
            <span class="buttons"><button class="quiet" @click="asking = null">Not yet</button><button class="go" @click="confirmRun">Run it</button></span>
          </p>
          <p v-if="held" class="held">
            <span>{{ held.affected }} {{ held.affected === 1 ? 'row' : 'rows' }} changed on Production - not final yet ({{ secondsLeft(held.since, now) }} s)</span>
            <span class="buttons"><button class="quiet" @click="finish(false)">Undo</button><button class="go" @click="finish(true)">Keep</button></span>
          </p>
          <p v-if="problem" class="bad">{{ problem }}</p>
          <p v-else-if="note" class="dim">{{ note }}</p>

          <div class="bar">
            <span v-if="result" class="chip">{{ chipOf(result) }}</span>
            <span class="spacer"></span>
            <button class="small" :disabled="!result?.columns?.length" @click="exportRows">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19h14" /></svg>Export CSV
            </button>
            <button class="small" :disabled="!table" @click="dump(false)">
              <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3.5" y="5" width="17" height="14" rx="1.5" /><path d="M3.5 10h17M9.5 5v14" /></svg>Dump table
            </button>
            <button class="small" :disabled="!database" @click="dump(true)">
              <svg viewBox="0 0 24 24" aria-hidden="true"><ellipse cx="12" cy="6" rx="7" ry="2.5" /><path d="M5 6v12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5V6M5 12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5" /></svg>Dump database
            </button>
            <button class="small" :disabled="!database || (production && !unlocked)" :title="production && !unlocked ? 'Unlock Change mode to import into Production' : ''" @click="importFile">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15V4M7.5 8.5L12 4l4.5 4.5M5 19h14" /></svg>Import…
            </button>
          </div>

          <div v-if="result?.columns?.length" class="grid-wrap">
            <table class="grid">
              <thead>
                <tr><th v-for="c in result.columns" :key="c">{{ c }}</th></tr>
              </thead>
              <tbody>
                <tr v-for="(row, r) in result.rows" :key="r">
                  <td v-for="(v, c) in row" :key="c" :class="{ null: v === null }" :title="canChange ? 'Double-click to change' : ''" @dblclick="startEdit(r, c)">
                    <input v-if="cell && cell.r === r && cell.c === c" v-model="cell.value" class="cell" @keydown.enter="commitEdit" @keydown.esc="cell = null" @blur="commitEdit" />
                    <template v-else>{{ v === null ? 'NULL' : v }}</template>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <section class="saved">
            <h5>
              Saved queries
              <button v-if="!naming" class="link" :disabled="!sql.trim()" @click="naming = true">+ save this</button>
            </h5>
            <p v-if="naming" class="naming">
              <input v-model="savedName" placeholder="Name, e.g. Today's orders" aria-label="Saved query name" @keydown.enter="keepSaved" />
              <button class="small" @click="keepSaved">Save</button>
              <button class="link" @click="naming = false">cancel</button>
            </p>
            <ul>
              <li v-for="q in saved" :key="q.name">
                <button class="saved-row" :title="q.sql" @click="sql = q.sql; run()">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 3h7l4 4v14H7z" /><path d="M14 3v4h4" /></svg>{{ q.name }}
                </button>
                <button class="link" :aria-label="`Remove ${q.name}`" @click="dropSaved(q.name)">×</button>
              </li>
            </ul>
          </section>
        </div>
      </div>
    </template>

    <p v-else-if="problem" class="bad">{{ problem }}</p>
    <p v-else class="dim">Reading the project…</p>
  </aside>
</template>

<style scoped>
.database {
  --tint: #5fe0c8;
  display: flex;
  flex: 0 0 60%;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  min-height: 0;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--tint) 35%, var(--line, rgba(198, 162, 255, 0.2)));
  border-radius: 12px;
  font-size: 13px;
  overflow: hidden;
}

/* production wears its amber all round, so it is never mistaken for local */
.database.production {
  --tint: #f0b545;
  border: 1.5px solid #f0b545;
  box-shadow: 0 0 0 1px rgba(240, 181, 69, 0.12), 0 0 22px rgba(240, 181, 69, 0.14);
}

.tabs {
  display: inline-flex;
  align-self: flex-start;
  gap: 2px;
  padding: 3px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.25));
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.25);
}

.tabs button {
  padding: 5px 18px;
  border: 0;
  border-radius: 7px;
  background: none;
  color: var(--text-dim, #c9bfe8);
  font: inherit;
  font-size: 13.5px;
  cursor: pointer;
}

.tabs button.on {
  background: color-mix(in srgb, var(--tint) 85%, #2a1f5c);
  color: #10142a;
  font-weight: 600;
}

/* the setup form */
.setup {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px dashed color-mix(in srgb, var(--tint) 55%, transparent);
  border-radius: 10px;
  overflow-y: auto;
}

.setup h4 {
  margin: 0;
  font-size: 14px;
  color: var(--tint);
}

.setup p {
  margin: 0;
}

.fields {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px 12px;
}

.fields label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 11.5px;
  color: var(--text-dim, #c9bfe8);
}

.fields .wide {
  grid-column: 1 / -1;
}

.fields input {
  padding: 6px 8px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.3));
  border-radius: 7px;
  background: rgba(0, 0, 0, 0.3);
  color: var(--text, #fff);
  font: inherit;
  font-size: 13px;
}

.fields input:focus-visible {
  outline: 2px solid var(--tint);
  outline-offset: 1px;
}

.ca {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  font-size: 11.5px;
  color: var(--text-dim, #c9bfe8);
}

.where {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 10px;
  padding: 2px 0;
}

.where .sep {
  width: 1px;
  height: 14px;
  background: var(--line, rgba(198, 162, 255, 0.3));
}

.where .host {
  color: var(--text-dim, #c9bfe8);
}

.where .push {
  flex: 1;
}

.where-more {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 16px;
  margin: -4px 0 0 18px;
  color: var(--text-dim, #c9bfe8);
  font-size: 11.5px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--tint);
}

code {
  font-family: 'Cascadia Mono', Consolas, monospace;
  font-size: 12.5px;
}

.where .db-pick {
  min-width: 180px;
  font-size: 12px;
}

.chip {
  padding: 2px 10px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.3));
  border-radius: 999px;
  font-size: 11.5px;
}

.chip.teal {
  border-color: rgba(95, 224, 200, 0.55);
  color: #5fe0c8;
}

.chip.look {
  border-color: rgba(120, 220, 150, 0.55);
  background: rgba(60, 180, 110, 0.14);
  color: #8fe6a8;
}

.chip.look.off {
  border-color: #f0b545;
  background: rgba(240, 181, 69, 0.18);
  color: #ffd27a;
}

.lock {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.35));
  border-radius: 8px;
  background: none;
  color: inherit;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.lock.on {
  border-color: #f0b545;
  color: #ffd27a;
}

.lock svg,
.tables svg,
.run svg,
.small svg,
.filter svg,
.saved-row svg {
  width: 14px;
  height: 14px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.7;
}

.run svg {
  fill: currentColor;
  stroke: none;
}

.dim {
  color: var(--text-dim, #c9bfe8);
  font-size: 11.5px;
}

.work {
  display: flex;
  flex: 1;
  gap: 10px;
  min-height: 0;
}

.side-list {
  display: flex;
  flex: 0 0 210px;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
}

.filter {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.3));
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.3);
  color: var(--text-dim, #c9bfe8);
}

.filter input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: none;
  background: none;
  color: var(--text, #fff);
  font: inherit;
  font-size: 12.5px;
}

.tables {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.tables svg {
  flex: none;
}

.tables .tname {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tables .count {
  flex: none;
  margin-left: 4px;
  color: var(--text-dim, #c9bfe8);
  font-family: 'Cascadia Mono', Consolas, monospace;
  font-size: 11px;
  opacity: 0.8;
}

.tables button {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  padding: 6px 8px;
  border: 0;
  border-radius: 7px;
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.tables button.on {
  background: color-mix(in srgb, var(--tint) 18%, transparent);
}

.none {
  padding: 6px 8px;
  color: var(--text-dim, #c9bfe8);
}

.main {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
}

/* Run sits inside the editor's bottom-right corner */
.query {
  position: relative;
  display: flex;
}

textarea {
  flex: 1;
  min-height: 74px;
  padding: 8px 10px 44px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.3));
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.3);
  color: inherit;
  font-family: 'Cascadia Mono', Consolas, monospace;
  font-size: 12.5px;
  resize: vertical;
}

.run {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: 0;
  border-radius: 8px;
  background: var(--tint);
  color: #10142a;
  font: inherit;
  font-weight: 600;
  cursor: pointer;
}

.run:disabled {
  opacity: 0.45;
}

.ask,
.held {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 10px 12px;
  border: 1px solid #f0b545;
  border-radius: 10px;
  background: rgba(240, 181, 69, 0.1);
}

.ask.shape {
  border-color: #e05a6e;
  background: rgba(224, 90, 110, 0.12);
}

.ask code {
  overflow-wrap: anywhere;
}

.buttons {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.go,
.quiet,
.small {
  padding: 5px 12px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.35));
  border-radius: 7px;
  background: none;
  color: inherit;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.small {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.go {
  border-color: var(--tint);
  background: color-mix(in srgb, var(--tint) 22%, transparent);
}

.go:disabled,
.small:disabled {
  opacity: 0.5;
  cursor: default;
}

.bad {
  margin: 0;
  color: #ffb3bd;
}

.grid-wrap {
  max-height: 46vh;
  overflow: auto;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.2));
  border-radius: 8px;
}

.grid {
  width: 100%;
  border-collapse: collapse;
  font-family: 'Cascadia Mono', Consolas, monospace;
  font-size: 12px;
}

.grid th {
  position: sticky;
  top: 0;
  padding: 6px 10px;
  background: rgba(30, 20, 70, 0.98);
  color: var(--tint);
  font-weight: 500;
  text-align: left;
  white-space: nowrap;
}

.grid td {
  max-width: 280px;
  padding: 5px 10px;
  overflow: hidden;
  border-top: 1px solid rgba(198, 162, 255, 0.12);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.grid tbody tr:nth-child(even) td {
  background: rgba(255, 255, 255, 0.025);
}

.grid td.null {
  color: var(--text-dim, #8f86b8);
  font-style: italic;
}

.cell {
  width: 100%;
  border: 1px solid var(--tint);
  background: rgba(14, 9, 38, 0.95);
  color: inherit;
  font: inherit;
}

.bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.spacer {
  flex: 1;
}

.saved h5 {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 4px 0;
  font-size: 12px;
  font-weight: 600;
}

.saved ul {
  margin: 0;
  padding: 0;
  list-style: none;
}

.saved li {
  display: flex;
  align-items: center;
}

.saved-row {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 7px;
  padding: 6px 8px;
  border: 1px solid rgba(198, 162, 255, 0.2);
  border-radius: 8px;
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.naming {
  display: flex;
  gap: 6px;
  margin: 0 0 6px;
}

.naming input {
  flex: 1;
  padding: 5px 8px;
  border: 1px solid var(--line, rgba(198, 162, 255, 0.3));
  border-radius: 7px;
  background: rgba(20, 13, 51, 0.9);
  color: inherit;
  font: inherit;
}

.link {
  border: 0;
  background: none;
  color: #c9b3ff;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

button:focus-visible {
  outline: 2px solid var(--tint);
  outline-offset: 2px;
}
</style>
