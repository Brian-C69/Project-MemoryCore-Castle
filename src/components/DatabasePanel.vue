<script setup>
// The memory database, step by step: install (or use the one already on this PC), start,
// create the companion's own database, move the memory in. Used by Settings > Database, the
// HOME card's sheet and setup's last step. Optional: a memory kept in files works as before.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  chooseFolder, dbCreate, dbExport, dbInstall, dbMemory, dbMoveIn, dbMovePreview, dbRestore, dbSkills, dbSkillStub, dbStart, dbStatus, dbStop, dbUseExisting,
  getSettings, onDbInstall, saveSettings,
} from '../api/castle.js'
import { megabytes, moveLine, nextStep, progressLine, steps as stepsOf } from './database.js'

const props = defineProps({
  companion: { type: String, default: 'your companion' },
})
const emit = defineEmits(['changed'])

const view = ref(null) // { engine, version, downloadBytes, keepRunning }
const memory = ref(null) // { name, reachable, movedIn, canRestore }
const preview = ref(null) // Counts
const skills = ref([]) // [{ plugin, name, description, stubbed }]
const busy = ref('')
const problem = ref('')
const done = ref('')
const adminUser = ref('root')
const adminPassword = ref('')
const keepRunning = ref(false)
const bootAtWake = ref(true)
const sure = ref(false)
let settings = null
let stopListening = null

const engine = computed(() => view.value?.engine ?? null)
const steps = computed(() => stepsOf(engine.value, memory.value))
const next = computed(() => nextStep(engine.value, memory.value))
const own = computed(() => engine.value && engine.value.state !== 'external')

async function attempt(label, work) {
  problem.value = ''
  done.value = ''
  busy.value = label
  try {
    return await work()
  } catch (reason) {
    problem.value = String(reason)
    return null
  } finally {
    busy.value = ''
  }
}

async function check() {
  try {
    view.value = await dbStatus()
    memory.value = await dbMemory()
    keepRunning.value = view.value.keepRunning
    preview.value = next.value === 'move' ? await dbMovePreview() : null
    skills.value = next.value === null ? await dbSkills() : []
  } catch {
    // outside Castle's window
  }
  emit('changed')
}

async function install() {
  const result = await attempt('Downloading…', dbInstall)
  if (result) done.value = 'The database is installed and running.'
  await check()
}

async function useExisting() {
  const result = await attempt('Checking…', dbUseExisting)
  if (result) done.value = 'Castle will use the database already on this PC.'
  await check()
}

async function start() {
  const result = await attempt('Starting…', dbStart)
  if (result) done.value = 'Running.'
  await check()
}

async function stop() {
  const result = await attempt('Stopping…', dbStop)
  if (result) done.value = 'Stopped.'
  await check()
}

async function create() {
  const name = await attempt('Creating the database…', () => (own.value ? dbCreate(null, null) : dbCreate(adminUser.value, adminPassword.value)))
  adminPassword.value = ''
  if (name) done.value = `${name} is ready.`
  await check()
}

async function moveIn() {
  const counts = await attempt('Moving your memory in…', dbMoveIn)
  sure.value = false
  if (counts) done.value = `Moved in: ${counts.files} ${counts.files === 1 ? 'file' : 'files'} are in the database now.`
  await check()
}

async function restore() {
  const line = await attempt('Restoring from your backup…', dbRestore)
  if (line) done.value = 'Your memory is back from the backup.'
  await check()
}

async function exportOut() {
  const folder = await chooseFolder('Choose a folder for the markdown copy')
  if (!folder) return
  const line = await attempt('Writing the markdown…', () => dbExport(folder))
  if (line) done.value = `Written to ${folder}.`
}

// One skill between a whole SKILL.md on disk and a stub served from the database.
async function toggleStub(skill) {
  const rows = await attempt('Switching the skill…', () => dbSkillStub(skill.plugin, skill.name, !skill.stubbed))
  if (rows) {
    skills.value = rows
    done.value = skill.stubbed
      ? `${skill.name} is a whole file again.`
      : `${skill.name} is served from the database now. Start a new session with ${props.companion} to pick it up.`
  }
}

async function toggleBoot() {
  if (!settings) return
  settings = { ...settings, dbBootAtWake: bootAtWake.value }
  const saved = await attempt('Saving…', () => saveSettings(settings))
  if (saved) {
    settings = saved.settings
    done.value = bootAtWake.value ? `Castle loads the memory when ${props.companion} wakes.` : `${props.companion} loads the memory herself after waking.`
  }
}

async function toggleKeep() {
  if (!settings) return
  settings = { ...settings, dbKeepRunning: keepRunning.value }
  const saved = await attempt('Saving…', () => saveSettings(settings))
  if (saved) settings = saved.settings
}

onMounted(async () => {
  try {
    settings = (await getSettings()).settings
    bootAtWake.value = settings.dbBootAtWake
    stopListening = await onDbInstall(({ done: got, total }) => {
      if (busy.value) busy.value = progressLine(got, total)
    })
  } catch {
    // outside Castle's window
  }
  check()
})
onBeforeUnmount(() => stopListening?.())
</script>

<template>
  <div class="database">
    <p class="hint">
      Keep {{ companion }}'s memory in a database on this computer instead of in loose files: faster to search, every change kept, nothing to tidy.
      It is optional, and nothing leaves this PC.
    </p>

    <ol class="steps">
      <li v-for="step in steps" :key="step.id" :class="{ ok: step.ok }">
        <span class="mark">{{ step.ok ? '✓' : '○' }}</span> {{ step.label }}
      </li>
    </ol>

    <template v-if="next === 'install'">
      <div class="row">
        <button class="primary" :disabled="!!busy" @click="install">Install the database</button>
        <span class="hint">About {{ megabytes(view?.downloadBytes) }} · MariaDB {{ view?.version }} · no admin needed</span>
      </div>
      <div v-if="engine?.external" class="row">
        <button :disabled="!!busy" @click="useExisting">Use the database already on this PC</button>
        <span class="hint">A MySQL / MariaDB is answering on port {{ engine.external }} (XAMPP, for example).</span>
      </div>
    </template>

    <template v-else-if="next === 'start'">
      <div v-if="own" class="row">
        <button class="primary" :disabled="!!busy" @click="start">Start</button>
        <span class="hint">It also starts by itself whenever Castle opens.</span>
      </div>
      <p v-else class="next">The database on this PC (port {{ engine.port }}) is not answering. Start it in its own program, then press Check again.</p>
    </template>

    <template v-else-if="next === 'create'">
      <template v-if="!own">
        <p class="next">Castle needs that database's admin login once, to make {{ companion }}'s own database. It is used now and never kept.</p>
        <div class="row">
          <input v-model="adminUser" aria-label="Admin user" placeholder="root" />
          <input v-model="adminPassword" type="password" aria-label="Admin password" placeholder="password (often empty on XAMPP)" />
        </div>
      </template>
      <div class="row">
        <button class="primary" :disabled="!!busy" @click="create">Create {{ companion }}'s database</button>
      </div>
    </template>

    <template v-else-if="next === 'move'">
      <p class="next">{{ moveLine(preview) }}</p>
      <p v-if="preview?.skipped?.length" class="next warn">
        Left where they are (not plain text): {{ preview.skipped.join(', ') }}
      </p>
      <template v-if="preview?.files">
        <p class="hint">
          Castle copies every file into the database, reads each one back to check it, and only then deletes the files.
          Your skills stay as files too. <strong>Export to markdown</strong> gives everything back whenever you like.
        </p>
        <label class="toggle"><input v-model="sure" type="checkbox" /> I understand the files are deleted after they are safely in the database</label>
        <div class="row">
          <button class="primary" :disabled="!!busy || !sure" @click="moveIn">Move my memory in</button>
        </div>
      </template>
      <div v-if="memory?.canRestore" class="row">
        <button :disabled="!!busy" @click="restore">Restore from my backup</button>
        <span class="hint">This folder carries a backup of the database. Bring the memory back from it.</span>
      </div>
    </template>

    <template v-else-if="engine">
      <p class="facts">{{ memory?.name }} · port {{ engine.port }} · running</p>
      <div class="row">
        <button :disabled="!!busy" @click="exportOut">Export to markdown…</button>
        <button v-if="own" :disabled="!!busy" @click="stop">Stop</button>
      </div>

      <label class="toggle" title="Claude Code only. With Codex, or with this off, the companion loads the memory with one command after waking.">
        <input v-model="bootAtWake" type="checkbox" @change="toggleBoot" /> Load the memory when {{ companion }} wakes
      </label>

      <h4>Skills</h4>
      <p v-if="!skills.length" class="hint">No skills installed yet. Once {{ companion }} installs one, it shows here.</p>
      <template v-else>
        <p class="hint">
          Every skill is kept in the database and stays on disk as a file. Tick one to <strong>serve it from the database</strong>:
          its file becomes a short note that tells {{ companion }} how to load the instructions, a section at a time.
          It needs the database running; untick to put the whole file back.
        </p>
        <ul class="skills">
          <li v-for="skill in skills" :key="`${skill.plugin}/${skill.name}`">
            <label class="toggle" :title="skill.description ?? ''">
              <input type="checkbox" :checked="skill.stubbed" :disabled="!!busy" @change="toggleStub(skill)" />
              <span class="skill-name">{{ skill.name }}</span>
              <span class="hint">{{ skill.plugin }}{{ skill.stubbed ? ' · from the database' : '' }}</span>
            </label>
          </li>
        </ul>
      </template>
    </template>

    <label v-if="own && next !== 'install'" class="toggle">
      <input v-model="keepRunning" type="checkbox" @change="toggleKeep" /> Keep the database running when Castle is closed
    </label>

    <div class="row">
      <button :disabled="!!busy" @click="check">Check again</button>
      <span v-if="busy" class="busy">{{ busy }}</span>
      <span v-else-if="problem" class="problem">{{ problem }}</span>
      <span v-else-if="done" class="good">{{ done }}</span>
    </div>
  </div>
</template>

<style scoped>
.database {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.hint {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.steps {
  display: grid;
  gap: 6px;
  margin: 4px 0;
  padding: 0;
  list-style: none;
  font-size: 14px;
  color: var(--text-dim);
}
.steps li.ok {
  color: var(--good);
}
.mark {
  display: inline-block;
  width: 18px;
}
.next {
  margin: 0;
  font-size: 14px;
}
.next.warn,
.problem {
  color: var(--warn);
  font-size: 13px;
}
.facts {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
h4 {
  margin: 8px 0 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.skills {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.skill-name {
  min-width: 180px;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.toggle {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 14px;
}
.busy {
  font-size: 13px;
  color: var(--accent);
}
.good {
  font-size: 13px;
  color: var(--good);
}
input:not([type='checkbox']) {
  flex: 1;
  max-width: 260px;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
button {
  padding: 8px 16px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 14px;
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
