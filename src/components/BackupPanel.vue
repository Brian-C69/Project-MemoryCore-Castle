<script setup>
// The guided private GitHub backup: four steps, each with its own fix, then Back up now.
// Used by setup's last step and by Settings > Backup. The memory is personal, so the
// repository is always private, and MemoryCore's public template is never pushed to.
import { computed, onMounted, ref } from 'vue'
import { backupCreate, backupNow, backupSignIn, backupStatus, getSettings, openUrl, saveSettings } from '../api/castle.js'

const props = defineProps({
  companion: { type: String, default: 'your companion' },
})

const status = ref(null)
const busy = ref('')
const problem = ref('')
const done = ref('')
const repoName = ref('my-memorycore')
const afterDiary = ref(false)
let settings = null

const steps = computed(() => {
  const s = status.value
  if (!s) return []
  return [
    { label: 'Git installed', ok: s.gitInstalled, fix: 'git' },
    { label: 'GitHub CLI installed', ok: s.ghInstalled, fix: 'gh' },
    { label: s.ghUser ? `Signed in to GitHub as ${s.ghUser}` : 'Signed in to GitHub', ok: s.ghSignedIn, fix: 'signin' },
    { label: 'Your own private backup', ok: s.ready, fix: 'create' },
  ]
})
const firstOpen = computed(() => steps.value.find((step) => !step.ok)?.fix ?? null)

function when(iso) {
  if (!iso) return 'never'
  const date = new Date(iso)
  return Number.isNaN(date.getTime()) ? iso : date.toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' })
}

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
  const next = await attempt('Checking…', backupStatus)
  if (next) status.value = next
}

async function create() {
  const next = await attempt('Creating your private backup on GitHub…', () => backupCreate(repoName.value))
  if (next) {
    status.value = next
    done.value = 'Your private backup is ready.'
  }
}

async function now() {
  const next = await attempt('Backing up…', backupNow)
  if (next) {
    status.value = next
    done.value = 'Backed up.'
  }
}

async function toggleAfterDiary() {
  if (!settings) return
  settings = { ...settings, backupAfterDiary: afterDiary.value }
  const saved = await attempt('Saving…', () => saveSettings(settings))
  if (saved) settings = saved.settings
}

onMounted(async () => {
  try {
    settings = (await getSettings()).settings
    afterDiary.value = settings.backupAfterDiary
  } catch {
    // outside Castle's window
  }
  check()
})
</script>

<template>
  <div class="backup">
    <p class="hint">Keep {{ companion }}'s memory safe on GitHub, in a <strong>private</strong> repository only you can see. Castle never stores your GitHub password.</p>

    <ol class="steps">
      <li v-for="step in steps" :key="step.label" :class="{ ok: step.ok }">
        <span class="mark">{{ step.ok ? '✓' : '○' }}</span> {{ step.label }}
      </li>
    </ol>

    <p v-if="status" class="next" :class="{ warn: status.remoteIsTemplate || status.visibility === 'PUBLIC' }">{{ status.nextStep }}</p>

    <div v-if="firstOpen === 'git'" class="row">
      <button class="primary" @click="openUrl('https://git-scm.com/download/win')">Get Git</button>
    </div>
    <div v-else-if="firstOpen === 'gh'" class="row">
      <button class="primary" @click="openUrl('https://cli.github.com/')">Get the GitHub CLI</button>
    </div>
    <div v-else-if="firstOpen === 'signin'" class="row">
      <button class="primary" @click="attempt('Opening GitHub sign-in…', backupSignIn)">Sign in to GitHub</button>
      <span class="hint">A small window opens: follow it in your browser, then press Check again.</span>
    </div>
    <div v-else-if="firstOpen === 'create'" class="row">
      <input v-model="repoName" aria-label="Backup repository name" />
      <button class="primary" :disabled="!!busy || !repoName.trim()" @click="create">Create my private backup</button>
    </div>
    <template v-else-if="status?.ready">
      <p class="facts">
        Last backup: {{ when(status.lastBackup) }} · {{ status.changed }} {{ status.changed === 1 ? 'file' : 'files' }} changed since
        <span v-if="status.visibility"> · {{ status.visibility.toLowerCase() }}</span>
      </p>
      <div class="row">
        <button class="primary" :disabled="!!busy" @click="now">Back up now</button>
      </div>
      <label class="toggle"><input v-model="afterDiary" type="checkbox" @change="toggleAfterDiary" /> Back up after every Save diary</label>
    </template>

    <div class="row">
      <button :disabled="!!busy" @click="check">Check again</button>
      <span v-if="busy" class="busy">{{ busy }}</span>
      <span v-else-if="problem" class="problem">{{ problem }}</span>
      <span v-else-if="done" class="good">{{ done }}</span>
    </div>
  </div>
</template>

<style scoped>
.backup {
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
  max-width: 280px;
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
