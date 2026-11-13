<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import DevTab from './dev/DevTab.vue'
import FileTree from './dev/FileTree.vue'
import FileView from './dev/FileView.vue'
import NewProject from './dev/NewProject.vue'
import { chooseFolder, devForget, devOpen, devState } from '../api/castle.js'
import { MAX_TABS, folderName, nextActive, orderTabs, recentProjects, tabForFolder } from './dev/devtabs.js'

// The DEV room: a session per project folder, up to eight tabs. The projects opened before are
// listed on the left; each open project has a tab, and a tab keeps working while another shows.
const props = defineProps({
  // the DEV room is the one showing
  visible: { type: Boolean, default: false },
  companion: { type: String, default: 'Your companion' },
  cli: { type: String, default: 'claude' },
  settingsStamp: { type: Number, default: 0 },
})
const emit = defineEmits(['settings', 'setup'])

// the tabs' running dots are read again this often while the room shows
const READ_EVERY_MS = 3000

const state = ref({ tabs: [], projects: [], memoryFolder: '' })
const activeSlot = ref(null)
const problem = ref('')
const busy = ref(false)
let timer

const tabs = computed(() => orderTabs(state.value.tabs).map((t) => ({ ...t, name: t.name || folderName(t.folder) })))
const projects = computed(() => recentProjects(state.value.projects, state.value.tabs, state.value.memoryFolder))
const full = computed(() => tabs.value.length >= MAX_TABS)

// the left column: the recent PROJECTS, or the open project's FILES
const side = ref('projects')
// the file open in the reader ({ name, rel }), for the active tab
const reading = ref(null)
const tabViews = {}
const activeTab = computed(() => tabs.value.find((t) => t.slot === activeSlot.value) ?? null)
watch(activeSlot, () => (reading.value = null))
watch(activeTab, (tab) => {
  if (!tab && side.value === 'files') side.value = 'projects'
})
// Put in chat: the file goes into the active tab's message box, as a dropped file does
function putInChat(path) {
  reading.value = null
  tabViews[activeSlot.value]?.putInChat(path)
}

function apply(next, wantedFolder = null) {
  if (!next) return
  state.value = { tabs: next.tabs ?? [], projects: next.projects ?? [], memoryFolder: next.memoryFolder ?? '' }
  activeSlot.value = nextActive(state.value.tabs, activeSlot.value, wantedFolder)
}

async function refresh() {
  if (document.visibilityState !== 'visible') return
  try {
    apply(await devState())
  } catch {
    // outside Castle's window, or the core is busy: the last list stays
  }
}

async function openProject(folder) {
  problem.value = ''
  const open = tabForFolder(state.value.tabs, folder)
  if (open) {
    activeSlot.value = open.slot
    return
  }
  busy.value = true
  try {
    apply(await devOpen(folder), folder)
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    busy.value = false
  }
}

async function openFolder() {
  problem.value = ''
  let folder
  try {
    folder = await chooseFolder('Open a project folder')
  } catch (reason) {
    problem.value = String(reason)
    return
  }
  if (folder) await openProject(folder)
}

// New project: the sheet clones or creates the repo, then the new folder opens as a tab
const making = ref(false)
async function made(folder) {
  making.value = false
  if (folder) await openProject(folder)
}

async function forget(folder) {
  problem.value = ''
  try {
    apply(await devForget(folder))
  } catch (reason) {
    problem.value = String(reason)
  }
}

// a tab's session started or ended: its dot changes now, before the next read
function markRunning(slot, on) {
  state.value = { ...state.value, tabs: state.value.tabs.map((t) => (t.slot === slot ? { ...t, running: on } : t)) }
}

watch(
  () => props.visible,
  (on) => {
    clearInterval(timer)
    if (!on) return
    refresh()
    timer = setInterval(refresh, READ_EVERY_MS)
  },
)
onMounted(() => {
  refresh()
  if (props.visible) timer = setInterval(refresh, READ_EVERY_MS)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <div class="dev-room">
    <aside class="projects widget" :aria-label="side === 'files' ? 'Files' : 'Projects'">
      <div class="side-switch" role="tablist" aria-label="Left column">
        <button role="tab" :aria-selected="side === 'projects'" :class="{ on: side === 'projects' }" @click="side = 'projects'">PROJECTS</button>
        <button role="tab" :aria-selected="side === 'files'" :class="{ on: side === 'files' }" :disabled="!activeTab" :title="activeTab ? `The files of ${activeTab.name}` : 'Open a project to see its files'" @click="side = 'files'">FILES</button>
      </div>
      <FileTree
        v-if="activeTab"
        v-show="side === 'files'"
        :key="`files:${activeTab.slot}:${activeTab.folder}`"
        :slot="activeTab.slot"
        :folder="activeTab.folder"
        :shown="visible && side === 'files'"
        :selected="reading?.rel"
        @open="reading = $event"
      />
      <template v-if="side === 'projects'">
      <button class="open-folder new-project" :disabled="busy" title="Clone a repo from a link, or create a new one on GitHub" @click="making = true">+ New project</button>
      <button class="open-folder" :disabled="busy" :title="full ? `All ${MAX_TABS} tabs are in use - close a session to open another project` : ''" @click="openFolder">
        {{ busy ? 'Opening…' : 'Open a folder…' }}
      </button>
      <p v-if="problem" class="problem" role="alert">{{ problem }}</p>
      <p v-if="!projects.length" class="empty">The project folders you open are kept here.</p>
      <ul class="recent">
        <li v-for="project in projects" :key="project.folder" :class="{ active: project.slot !== null && project.slot === activeSlot }">
          <button class="project" :title="project.folder" :disabled="busy" @click="openProject(project.folder)">
            <span class="name">
              <span v-if="project.slot !== null" class="dot" :class="{ on: tabs.find((t) => t.slot === project.slot)?.running }"></span>
              {{ project.name }}
            </span>
            <span class="folder">{{ project.folder }}</span>
          </button>
          <button class="forget" title="Take off the list" :aria-label="`Take ${project.name} off the list`" @click="forget(project.folder)">&times;</button>
        </li>
      </ul>
      </template>
    </aside>

    <div class="work">
      <nav v-if="tabs.length" class="tab-strip" role="tablist" aria-label="Open projects">
        <button
          v-for="tab in tabs"
          :key="tab.slot"
          role="tab"
          class="strip-tab"
          :class="{ active: tab.slot === activeSlot }"
          :aria-selected="tab.slot === activeSlot"
          :title="tab.folder"
          @click="activeSlot = tab.slot"
        >
          <span class="dot" :class="{ on: tab.running }"></span>
          <span class="label">{{ tab.name }}</span>
        </button>
        <span class="count">{{ tabs.length }} / {{ MAX_TABS }}</span>
      </nav>

      <DevTab
        v-for="tab in tabs"
        v-show="tab.slot === activeSlot"
        :key="`${tab.slot}:${tab.folder}`"
        :ref="(view) => (view ? (tabViews[tab.slot] = view) : delete tabViews[tab.slot])"
        :tab="tab"
        :shown="visible && tab.slot === activeSlot"
        :companion="companion"
        :cli="cli"
        :settings-stamp="settingsStamp"
        @state="apply($event)"
        @running="markRunning(tab.slot, $event)"
        @settings="emit('settings', $event)"
        @setup="emit('setup')"
      />

      <FileView v-if="reading && activeTab" :slot="activeTab.slot" :folder="activeTab.folder" :entry="reading" @put="putInChat" @close="reading = null" />

      <div v-if="!tabs.length" class="nothing widget">
        <p class="title">No project is open.</p>
        <p>Choose one on the left, open a folder, or start a new project, and {{ companion }} works there in a session of its own.</p>
      </div>
    </div>

    <NewProject v-if="making" @made="made" @close="making = false" />
  </div>
</template>

<style scoped>
.dev-room {
  position: absolute;
  inset: 24px;
  display: grid;
  grid-template-columns: minmax(180px, 260px) minmax(0, 1fr);
  gap: 18px;
  min-width: 0;
}

.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}

.side-switch {
  display: flex;
  gap: 6px;
  margin-bottom: 4px;
}
.side-switch button {
  flex: 1;
  font-size: 12px;
  letter-spacing: 0.12em;
  padding: 6px 8px;
  opacity: 0.7;
}
.side-switch button.on {
  opacity: 1;
  border-color: var(--accent, currentColor);
}
.projects {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  min-height: 0;
  padding: 18px 14px;
  overflow-y: auto;
}

h2 {
  margin: 0 0 4px;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}

button {
  font: inherit;
  cursor: pointer;
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.open-folder {
  padding: 7px 12px;
  border: 1px solid var(--accent);
  border-radius: 8px;
  background: var(--hover);
  color: var(--text);
  font-size: 13px;
  text-align: left;
}

.open-folder:hover:not(:disabled) {
  filter: brightness(1.12);
}

.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
  overflow-wrap: anywhere;
}

.empty {
  margin: 4px 0;
  font-size: 13px;
  color: var(--text-dim);
}

.recent {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.recent li {
  display: flex;
  align-items: stretch;
  min-width: 0;
  border: 1px solid transparent;
  border-radius: 8px;
}

.recent li:hover {
  background: var(--hover);
}

.recent li.active {
  border-color: var(--accent);
  background: var(--hover);
}

.project {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  padding: 6px 4px 6px 10px;
  border: 0;
  background: none;
  color: var(--text);
  text-align: left;
}

.project .name {
  display: flex;
  align-items: center;
  gap: 7px;
  overflow: hidden;
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project .folder {
  overflow: hidden;
  font-size: 11px;
  color: var(--text-dim);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.forget {
  flex: none;
  width: 28px;
  border: 0;
  border-radius: 6px;
  background: none;
  color: var(--text-dim);
  font-size: 16px;
  opacity: 0.6;
}

.forget:hover,
.forget:focus-visible {
  color: var(--text);
  opacity: 1;
}

.dot {
  flex: none;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-dim);
  opacity: 0.5;
}

.dot.on {
  background: var(--good);
  opacity: 1;
  box-shadow: 0 0 6px var(--good);
}

.work {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  min-height: 0;
}

.tab-strip {
  display: flex;
  flex: none;
  align-items: center;
  gap: 4px;
  min-width: 0;
  overflow-x: auto;
  border-bottom: 1px solid var(--line);
  scrollbar-width: thin;
  scrollbar-color: var(--line) transparent;
}

.strip-tab {
  display: flex;
  flex: 0 1 auto;
  align-items: center;
  gap: 7px;
  min-width: 70px;
  max-width: 200px;
  padding: 7px 12px;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 8px 8px 0 0;
  background: none;
  color: var(--text-dim);
  font-size: 13px;
}

.strip-tab:hover {
  background: var(--hover);
}

.strip-tab.active {
  color: var(--text);
  border-bottom-color: var(--accent);
  background: var(--panel-background);
}

.strip-tab .label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.count {
  flex: none;
  margin-left: auto;
  padding: 0 6px;
  font-size: 11px;
  color: var(--text-dim);
}

.nothing {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 24px;
  text-align: center;
  color: var(--text-dim);
  font-size: 14px;
}

.nothing p {
  margin: 0;
  max-width: 420px;
}

.nothing .title {
  color: var(--text);
  font-size: 17px;
  font-weight: 600;
}
</style>
