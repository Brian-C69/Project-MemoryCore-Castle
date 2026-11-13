<script setup>
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import TerminalPanel from '../../components/TerminalPanel.vue'
import ContextMeter from '../../components/ContextMeter.vue'
import { typeLine } from '../../components/typeline.js'
import PlanPanel from './PlanPanel.vue'
import SourcePanel from './SourcePanel.vue'
import { useSaveClose } from './useSaveClose.js'
import { devCloseTab, memoryFeatures } from '../../api/castle.js'

// One project's session in the DEV room: its chat in the middle, the Log behind a button, the
// plan beside it on request. It stays mounted while other tabs show, so the session keeps going.
const props = defineProps({
  // { slot, folder, name, running, status: { ready, message, fix } }
  tab: { type: Object, required: true },
  // this tab is the one showing (and the DEV room is open)
  shown: { type: Boolean, default: false },
  companion: { type: String, default: 'Your companion' },
  cli: { type: String, default: 'claude' },
  settingsStamp: { type: Number, default: 0 },
})
// `state`: a fresh dev state (after Wake or Close session); `running`: the session started or ended
const emit = defineEmits(['state', 'running', 'settings', 'setup'])

const panel = ref(null)
const logOpen = ref(false)
const planOpen = ref(false)
const sourceOpen = ref(false)
// what the chat reads off the session: working or not, and how full her context is
const busy = ref(false)
const context = ref(null)
// the member's MemoryCore keeps projects, so `save project` means something to her
const canSave = ref(false)
onMounted(async () => {
  try {
    canSave.value = !!(await memoryFeatures())?.projects
  } catch {
    canSave.value = false
  }
})
// what the session panel heard last (it hears before the room's next read of the tabs)
const liveRunning = ref(null)
const running = computed(() => liveRunning.value ?? !!props.tab.running)
watch(() => props.tab.running, () => (liveRunning.value = null))

// Close session: asked first, in the page, when the project has work not yet committed
const askingClose = ref(false)
const keepButton = ref(null)
const closing = ref(false)
const problem = ref('')

function onRunning(on) {
  liveRunning.value = on
  emit('running', on)
  if (!on) {
    saveClose.sessionEnded()
    context.value = null
  }
}

// Types a line into her session (text, then Enter after a pause, as the quick actions do).
function say(text) {
  return typeLine((data) => panel.value?.writeToSession(data), text)
}

function saveProject() {
  problem.value = ''
  say('save project')
}

// Close session saves first when the project has uncommitted work (or asks, without projects)
const saveClose = useSaveClose({
  slot: () => props.tab.slot,
  say,
  busy,
  running,
  canSave: () => canSave.value,
  close: () => closeNow(),
})
watch(
  () => saveClose.saving.value?.ask,
  async (ask) => {
    askingClose.value = !!ask
    if (ask) {
      // the safe answer has the focus, so Enter keeps it open and Esc does too
      await nextTick()
      keepButton.value?.focus()
    }
  },
)

// a file from the FILES reader, put in her message box
function putInChat(path) {
  panel.value?.addPaths([path])
}
defineExpose({ putInChat })

function wake() {
  problem.value = ''
  panel.value?.wake({ greet: true })
}

async function askClose() {
  problem.value = ''
  await saveClose.requestClose()
}

async function closeNow() {
  askingClose.value = false
  saveClose.dismiss()
  closing.value = true
  try {
    emit('state', await devCloseTab(props.tab.slot))
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    closing.value = false
  }
}
</script>

<template>
  <section class="dev-tab" :aria-label="tab.name">
    <header class="tab-head">
      <span class="dot" :class="{ on: running }" :title="running ? 'Working' : 'Asleep'"></span>
      <div class="names">
        <h2 :title="tab.name">{{ tab.name }}</h2>
        <span class="folder" :title="tab.folder">{{ tab.folder }}</span>
      </div>
      <div class="actions">
        <ContextMeter :context="context" />
        <button v-if="!running" class="primary" :disabled="!tab.status?.ready" :title="tab.status?.ready ? '' : tab.status?.message" @click="wake">Wake</button>
        <button v-if="running && canSave" :disabled="!!saveClose.saving.value" title="Ask her to save the project: its record in your memory, and a commit" @click="saveProject">Save project</button>
        <button :class="{ on: sourceOpen }" :aria-pressed="sourceOpen" @click="sourceOpen = !sourceOpen">Source</button>
        <button :class="{ on: planOpen }" :aria-pressed="planOpen" @click="planOpen = !planOpen">Plan</button>
        <button :class="{ on: logOpen }" :aria-pressed="logOpen" @click="logOpen = !logOpen">Log</button>
        <button :disabled="closing" @click="askClose">{{ closing ? 'Closing…' : 'Close session' }}</button>
      </div>
    </header>
    <p v-if="problem" class="problem" role="alert">{{ problem }}</p>
    <div v-if="saveClose.saving.value && !saveClose.saving.value.ask" class="saving" role="status">
      <span>{{ saveClose.saving.value.note }}</span>
      <button @click="saveClose.closeAnyway()">Close without saving</button>
      <button class="primary" @click="saveClose.dismiss()">Keep it open</button>
    </div>

    <div class="tab-body">
      <TerminalPanel
        ref="panel"
        :dev-slot="tab.slot"
        :shown="shown"
        :show-log="logOpen"
        :companion="companion"
        :cli="cli"
        :settings-stamp="settingsStamp"
        @view="logOpen = $event === 'log'"
        @busy="busy = $event"
        @context="context = $event"
        @running="onRunning"
        @state="emit('state', $event)"
        @settings="emit('settings', $event)"
        @setup="emit('setup')"
      />
      <SourcePanel v-if="sourceOpen" class="plan-side" :slot="tab.slot" :companion="companion" :shown="shown && sourceOpen" @ask="say" />
      <PlanPanel v-if="planOpen" class="plan-side" :index="tab.slot" :shown="shown && planOpen" />
    </div>

    <div v-if="askingClose" class="close-sheet" role="dialog" aria-modal="true" aria-labelledby="close-sheet-title" @click.self="askingClose = false; saveClose.dismiss()" @keydown.esc.stop="askingClose = false; saveClose.dismiss()">
      <div class="close-card">
        <h3 id="close-sheet-title">Close this session?</h3>
        <p>{{ tab.name }} has changes that are not committed yet. Closing the session keeps the files as they are, but {{ companion }} stops working here.</p>
        <div class="close-buttons">
          <button ref="keepButton" class="primary" @click="askingClose = false; saveClose.dismiss()">Keep it open</button>
          <button @click="closeNow">Close anyway</button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.dev-tab {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.saving {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  font-size: 13px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 12px;
}
.saving span {
  flex: 1;
}
.tab-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px 12px;
  padding: 10px 14px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}

.dot {
  flex: none;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--text-dim);
  opacity: 0.5;
}

.dot.on {
  background: var(--good);
  opacity: 1;
  box-shadow: 0 0 8px var(--good);
}

.names {
  display: flex;
  flex-direction: column;
  flex: 1 1 160px;
  min-width: 0;
}

h2 {
  margin: 0;
  overflow: hidden;
  font-size: 16px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.folder {
  overflow: hidden;
  font-size: 11px;
  color: var(--text-dim);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

button {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}

button.on {
  border-color: var(--accent);
  background: var(--hover);
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

button:hover:not(:disabled) {
  filter: brightness(1.12);
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.problem {
  margin: 0;
  padding: 0 4px;
  font-size: 13px;
  color: var(--warn);
}

.tab-body {
  display: flex;
  gap: 12px;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.plan-side {
  flex: 0 1 320px;
  min-width: 0;
}

.close-sheet {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  padding: 16px;
  border-radius: 14px;
  background: rgba(6, 4, 20, 0.42);
  backdrop-filter: blur(3px);
}

.close-card {
  width: min(420px, 100%);
  padding: 20px 22px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.45);
  box-sizing: border-box;
}

.close-card h3 {
  margin: 0 0 8px;
  font-size: 16px;
  font-weight: 600;
}

.close-card p {
  margin: 0 0 16px;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-dim);
  overflow-wrap: anywhere;
}

.close-buttons {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}
</style>
