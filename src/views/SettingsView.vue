<script setup>
// Settings: everything setup asked, changeable any time - the memory folder and companion, the
// keys (masked, Show, Test, Remove), quick actions, the room, backup, startup and updates.
import { computed, onMounted, ref } from 'vue'
import ModalSheet from '../components/ModalSheet.vue'
import BackupPanel from '../components/BackupPanel.vue'
import {
  avatarClear,
  avatarGet,
  avatarSet,
  chooseFolder,
  choosePicture,
  getSettings,
  keyForget,
  keyReveal,
  keySave,
  keyStatus,
  keyTest,
  saveSettings,
  setupApply,
  setupCheckCli,
  setupInspect,
} from '../api/castle.js'

const props = defineProps({
  // which section opens first ("keys" from a toast or a STUDIO card)
  section: { type: String, default: 'companion' },
  version: { type: String, default: '' },
  updateLine: { type: String, default: '' },
})
const emit = defineEmits(['close', 'saved', 'setup'])

const SECTIONS = [
  { id: 'companion', label: 'Companion' },
  { id: 'keys', label: 'Keys' },
  { id: 'quick', label: 'Quick actions' },
  { id: 'backup', label: 'Backup' },
  { id: 'startup', label: 'Start-up' },
  { id: 'about', label: 'About' },
]

const tab = ref(props.section)
const settings = ref(null)
const view = ref(null)
const keys = ref([])
const drafts = ref({})
const shown = ref({})
const tests = ref({})
const avatar = ref(null)
const cliCheck = ref(null)
const busy = ref('')
const problem = ref('')
const done = ref('')
const newAction = ref({ label: '', command: '' })

const shownKeys = computed(() => keys.value.filter((k) => !k.name.startsWith('room') && !(k.name === 'openai' && settings.value?.cli === 'codex')))

async function attempt(label, work) {
  problem.value = ''
  done.value = ''
  busy.value = label
  try {
    return await work()
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
    return null
  } finally {
    busy.value = ''
  }
}

async function save(patch, message = 'Saved.') {
  const next = { ...settings.value, ...patch }
  const saved = await attempt('Saving…', () => saveSettings(next))
  if (saved) {
    view.value = saved
    settings.value = saved.settings
    done.value = message
    emit('saved')
  }
  return saved
}

// ---------- Companion ----------

async function changeFolder() {
  const folder = await chooseFolder('Choose your MemoryCore folder')
  if (!folder) return
  const state = await attempt('Looking at the folder…', () => setupInspect(folder))
  if (!state) return
  if (!state.isMemorycore) {
    problem.value = 'That folder has no master-memory.md, so it is not a MemoryCore folder.'
    return
  }
  await save({ memoryFolder: folder }, 'The memory folder is changed. Wake your companion again to use it.')
}

async function saveNames() {
  const s = settings.value
  if (!s.companionName?.trim() || !s.memberName?.trim()) {
    problem.value = 'Both names are needed.'
    return
  }
  const saved = await save({}, 'Names saved.')
  if (saved && s.memoryFolder) await attempt('Updating CLAUDE.md and AGENTS.md…', () => setupApply(s.memoryFolder, s.companionName, s.memberName, s.relationshipStyle ?? ''))
}

async function switchCli(cli) {
  if (settings.value.cli === cli) return
  cliCheck.value = await attempt('Checking…', () => setupCheckCli(cli))
  await save({ cli }, `${cli === 'codex' ? 'Codex' : 'Claude Code'} is chosen. Wake your companion again to switch.`)
}

async function changeAvatar() {
  const path = await choosePicture('Choose a picture for your companion')
  if (path) avatar.value = await attempt('Saving…', () => avatarSet(path))
  emit('saved')
}

async function clearAvatar() {
  await attempt('Saving…', avatarClear)
  avatar.value = await avatarGet()
  emit('saved')
}

// ---------- Keys ----------

async function saveKey(name) {
  const value = (drafts.value[name] ?? '').trim()
  if (!value) return
  const next = await attempt('Keeping the key safe…', () => keySave(name, value))
  if (next) {
    keys.value = next
    drafts.value[name] = ''
    shown.value[name] = ''
    done.value = 'Key kept. Wake your companion again to use it.'
  }
}

async function toggleShow(name) {
  shown.value[name] = shown.value[name] ? '' : (await keyReveal(name)) ?? ''
}

async function test(name) {
  tests.value[name] = { ok: null, message: 'Checking…' }
  tests.value[name] = (await attempt('', () => keyTest(name))) ?? { ok: false, message: problem.value }
}

async function remove(name) {
  const next = await attempt('Removing…', () => keyForget(name))
  if (next) keys.value = next
}

// ---------- Quick actions ----------

async function addAction() {
  const a = { label: newAction.value.label.trim(), command: newAction.value.command.trim() }
  if (!a.label || !a.command) return
  if (await save({ quickActions: [...(settings.value.quickActions ?? []), a] }, 'Button added.')) newAction.value = { label: '', command: '' }
}

async function removeAction(i) {
  const list = [...settings.value.quickActions]
  list.splice(i, 1)
  await save({ quickActions: list }, 'Button removed.')
}

onMounted(async () => {
  try {
    view.value = await getSettings()
    settings.value = view.value.settings
    keys.value = await keyStatus()
    avatar.value = await avatarGet()
  } catch (reason) {
    problem.value = String(reason)
  }
})
</script>

<template>
  <ModalSheet tag="SETTINGS" title="Settings" @close="emit('close')">
    <div v-if="settings" class="settings">
      <nav class="tabs">
        <button v-for="s in SECTIONS" :key="s.id" :class="{ active: tab === s.id }" @click="tab = s.id">{{ s.label }}</button>
      </nav>

      <section v-if="tab === 'companion'" class="pane">
        <h3>Memory folder</h3>
        <div class="row"><code>{{ settings.memoryFolder ?? 'Not chosen' }}</code><button @click="changeFolder">Change</button></div>
        <h3>Thinks with</h3>
        <div class="row">
          <button :class="{ primary: settings.cli === 'claude' }" @click="switchCli('claude')">Claude Code</button>
          <button :class="{ primary: settings.cli === 'codex' }" @click="switchCli('codex')">Codex</button>
          <span v-if="cliCheck && !cliCheck.installed" class="warn">{{ cliCheck.label }} is not installed: <code>{{ cliCheck.installCommand }}</code></span>
        </div>
        <h3>Names</h3>
        <div class="row">
          <label>Companion <input v-model="settings.companionName" maxlength="40" /></label>
          <label>You <input v-model="settings.memberName" maxlength="40" /></label>
          <label class="wide">Your companion is <input v-model="settings.relationshipStyle" maxlength="80" /></label>
        </div>
        <button class="primary" @click="saveNames">Save names</button>
        <p class="hint">Names already written in your memory files stay as they are - ask your companion to update the memory if you rename.</p>
        <h3>Picture</h3>
        <div class="row">
          <div class="avatar"><img v-if="avatar" :src="avatar" alt="" /><span v-else>◆</span></div>
          <button @click="changeAvatar">Choose a picture</button>
          <button @click="clearAvatar">Use the drawn look / crystal</button>
        </div>
        <button @click="emit('setup')">Run the first-run setup again</button>
      </section>

      <section v-else-if="tab === 'keys'" class="pane">
        <p class="hint">Keys are kept encrypted by Windows and only handed to your companion's session. Without a key, your companion saves the idea instead and Castle tells you.</p>
        <div v-for="k in shownKeys" :key="k.name" class="key">
          <div class="row">
            <strong>{{ k.label }}</strong>
            <span v-if="k.set" class="good">{{ shown[k.name] || k.masked }}</span>
            <span v-else class="hint">not set</span>
            <span class="spacer"></span>
            <template v-if="k.set">
              <button @click="toggleShow(k.name)">{{ shown[k.name] ? 'Hide' : 'Show' }}</button>
              <button @click="test(k.name)">Test</button>
              <button class="danger" @click="remove(k.name)">Remove</button>
            </template>
          </div>
          <div class="row">
            <input v-model="drafts[k.name]" type="password" :placeholder="k.set ? 'Paste a new key to replace' : 'Paste key'" autocomplete="off" />
            <button :disabled="!(drafts[k.name] ?? '').trim()" @click="saveKey(k.name)">Save</button>
          </div>
          <p v-if="tests[k.name]" :class="tests[k.name].ok === false ? 'warn' : 'good'">{{ tests[k.name].message }}</p>
        </div>
        <p v-if="settings.cli === 'codex'" class="hint">The ChatGPT key is hidden: Codex draws with your ChatGPT account.</p>
        <p class="hint">Room keys for the MemoryCore Foundation room are set at the CHAT room's door.</p>
      </section>

      <section v-else-if="tab === 'quick'" class="pane">
        <p class="hint">Your own buttons, after Save diary / Save project / Save memory. A button types its words into your companion's session.</p>
        <div v-for="(a, i) in settings.quickActions" :key="a.label + i" class="row">
          <strong>{{ a.label }}</strong><code>{{ a.command }}</code><span class="spacer"></span>
          <button class="danger" @click="removeAction(i)">Remove</button>
        </div>
        <div class="row">
          <input v-model="newAction.label" placeholder="Plan my day" maxlength="24" />
          <input v-model="newAction.command" class="wide" placeholder="the words to type, e.g. plan my day" maxlength="200" />
          <button class="primary" :disabled="!newAction.label.trim() || !newAction.command.trim()" @click="addAction">Add</button>
        </div>
      </section>

      <section v-else-if="tab === 'backup'" class="pane">
        <BackupPanel :companion="settings.companionName ?? 'your companion'" />
      </section>

      <section v-else-if="tab === 'startup'" class="pane">
        <label class="toggle"><input type="checkbox" :checked="settings.startWithWindows" @change="save({ startWithWindows: $event.target.checked })" /> Start Castle when Windows starts</label>
        <p v-if="view?.startupNote" class="hint">{{ view.startupNote }}</p>
        <label class="toggle"><input type="checkbox" :checked="settings.startSessionOnLaunch" @change="save({ startSessionOnLaunch: $event.target.checked })" /> Wake {{ settings.companionName ?? 'your companion' }} when Castle starts with Windows</label>
        <p class="hint">Change the look in the THEME room.</p>
      </section>

      <section v-else class="pane">
        <h3>MemoryCore Castle {{ version }}</h3>
        <p class="hint">{{ updateLine || 'Updates: checked automatically in official builds.' }}</p>
        <p class="hint">Free and open source under the MIT license. Change anything - see MAKE-IT-YOURS.md in the source.</p>
        <p class="hint">Castle keeps no account and talks to no server of its own. Your memory, keys and settings stay on this computer (and in your private backup, if you choose).</p>
      </section>

      <footer>
        <span v-if="busy" class="busy">{{ busy }}</span>
        <span v-else-if="problem" class="warn">{{ problem }}</span>
        <span v-else-if="done" class="good">{{ done }}</span>
      </footer>
    </div>
  </ModalSheet>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.tabs .active {
  border-color: var(--accent);
  background: var(--hover);
}
.pane {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
h3 {
  margin: 8px 0 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.1em;
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}
.spacer {
  flex: 1;
}
label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
label.wide,
input.wide {
  flex: 1;
}
label.toggle {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}
input:not([type='checkbox']) {
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.key {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
}
.key input {
  flex: 1;
}
code {
  padding: 4px 8px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.3);
  font-size: 13px;
}
.avatar {
  display: grid;
  place-items: center;
  width: 56px;
  height: 56px;
  overflow: hidden;
  border: 2px solid var(--accent);
  border-radius: 50%;
  color: #cfe0ff;
  font-size: 24px;
}
.avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.hint {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.good {
  margin: 0;
  font-size: 13px;
  color: var(--good);
}
.warn {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.busy {
  font-size: 13px;
  color: var(--accent);
}
footer {
  min-height: 20px;
}
button {
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
button.danger {
  border-color: var(--bad);
  color: var(--bad);
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
