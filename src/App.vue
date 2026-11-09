<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import TopBar from './components/TopBar.vue'
import TerminalPanel from './components/TerminalPanel.vue'
import ClockWidget from './components/widgets/ClockWidget.vue'
import GaugeWidget from './components/widgets/GaugeWidget.vue'
import SetupView from './views/SetupView.vue'
import HomeCards from './components/widgets/HomeCards.vue'
import CompanionHeader from './components/CompanionHeader.vue'
import ModalSheet from './components/ModalSheet.vue'
import BackupPanel from './components/BackupPanel.vue'
import AppsView from './views/AppsView.vue'
import MemoryRoom from './views/MemoryRoom.vue'
import ThemeView from './views/ThemeView.vue'
import StudioView from './views/StudioView.vue'
import AppearanceView from './views/AppearanceView.vue'
import StoryView from './views/StoryView.vue'
import WorldView from './views/WorldView.vue'
import ChatRoomView from './views/ChatRoomView.vue'
import SettingsView from './views/SettingsView.vue'
import UpdateChip from './components/UpdateChip.vue'
import QuotaGlow from './components/QuotaGlow.vue'
import NoticeToasts from './components/NoticeToasts.vue'
import { avatarGet, backupNow, backupStatus, getSettings, hideCastle, memoryHome, onBackupDone, onStoryOutput, userThemes } from './api/castle.js'
import { DEFAULT_THEME_ID, applyTheme, builtInThemes, chooseTheme, fromUserFolders, placeStyle } from './themes/loader.js'

// The rooms across the top. A room whose MemoryCore feature is not installed stays hidden
// (`needs`); the rest always show.
const allTabs = [
  { id: 'home', label: 'HOME' },
  { id: 'theme', label: 'THEME' },
  { id: 'apps', label: 'APPS' },
  { id: 'diary', label: 'DIARY', needs: 'diary' },
  { id: 'projects', label: 'PROJECTS', needs: 'projects' },
  { id: 'library', label: 'LIBRARY', needs: 'library' },
  { id: 'appearance', label: 'APPEARANCE' },
  { id: 'studio', label: 'STUDIO' },
  { id: 'story', label: 'STORY' },
  { id: 'world', label: 'WORLD' },
  { id: 'chat', label: 'CHAT' },
]
// dev only: ?tab=<room> opens a room straight away (for checking a room in a browser)
const devTab = import.meta.env.DEV ? new URLSearchParams(location.search).get('tab') : null
const active = ref(devTab || 'home')
// What HOME shows (memory_home), the backup card, the portrait and the CLI chip.
const home = ref(null)
const backup = ref(null)
const avatar = ref(null)
const cliLabel = ref('Claude Code')
const memoryFolder = ref('')
const backupOpen = ref(false)
// the Settings sheet and the section it opens on
const settingsOpen = ref(false)
const settingsSection = ref('companion')
// the update line for Settings > About
const updateInfo = ref(null)
// Rooms that are built; any other room shows its name until its phase lands.
const BUILT = new Set(['home', 'theme', 'apps', 'diary', 'projects', 'library', 'studio', 'appearance', 'story', 'world', 'chat'])
// Tabs with something new waiting (the story speaking while the member is elsewhere).
const marks = ref([])
const memberName = ref('')
const storyOpened = ref(active.value === 'story')
const chatOpened = ref(active.value === 'chat')
watch(active, (room) => {
  if (room === 'story') storyOpened.value = true
  if (room === 'chat') chatOpened.value = true
})
// a file a HOME card asked to open in its room
const roomFile = ref('')
const memoryStamp = ref(0)
// rooms the member switched off in Settings are not available (never removed)
const rooms = ref({})
const tabs = computed(() =>
  allTabs.filter((tab) => (!tab.needs || home.value?.features?.[tab.needs]) && (tab.id === 'home' || rooms.value[tab.id] !== false)),
)
// a room switched off closes: its tab goes, its mark clears, a kept-alive room unmounts
watch(tabs, (list) => {
  const ids = new Set(list.map((t) => t.id))
  if (!ids.has(active.value)) active.value = 'home'
  if (!ids.has('story')) storyOpened.value = false
  if (!ids.has('chat')) chatOpened.value = false
  marks.value = marks.value.filter((m) => ids.has(m))
})
const companion = ref('Your companion')
const cli = ref('claude')
const settingsStamp = ref(0)
// The first-run setup: open until it is done; reopened from the panel or the gear.
const setupOpen = ref(false)
const setupReopened = ref(false)
const theme = ref(null)
const themeNotice = ref('')
const activeLabel = computed(() => allTabs.find((t) => t.id === active.value)?.label ?? '')
const widgets = computed(() => theme.value?.widgets ?? {})

async function loadSettingsAndTheme() {
  let wanted = DEFAULT_THEME_ID
  try {
    const view = await getSettings()
    companion.value = view.companionLabel
    cli.value = view.settings.cli
    cliLabel.value = view.cliLabel
    memoryFolder.value = view.settings.memoryFolder ?? ''
    memberName.value = view.settings.memberName ?? ''
    rooms.value = view.settings.rooms ?? {}
    if (!view.settings.setupDone) {
      setupOpen.value = true
      setupReopened.value = false
    }
    wanted = view.settings.theme ?? DEFAULT_THEME_ID
  } catch {
    // Outside Castle's window (a browser preview): defaults.
  }
  let folders = []
  try {
    folders = await userThemes()
  } catch {
    // Built-in themes only.
  }
  // dev only: ?theme=<id> shows a theme in a browser check
  const devTheme = import.meta.env.DEV ? new URLSearchParams(location.search).get('theme') : null
  if (devTheme) wanted = devTheme
  const all = [...builtInThemes(), ...fromUserFolders(folders).themes]
  const chosen = chooseTheme(all, wanted)
  theme.value = chosen.theme
  themeNotice.value = chosen.notice
  applyTheme(chosen.theme)
}

function openSettings(section = 'companion') {
  settingsSection.value = typeof section === 'string' ? section : 'companion'
  settingsOpen.value = true
}

function openKeys() {
  openSettings('keys')
}

async function settingsSaved() {
  await loadSettingsAndTheme()
  settingsStamp.value += 1
}

function openSetup() {
  setupReopened.value = true
  setupOpen.value = true
}

async function setupDone() {
  setupOpen.value = false
  await loadSettingsAndTheme()
  settingsStamp.value += 1
}

// The member's local date, so "days" count in their own time.
function today() {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

async function loadHome() {
  memoryStamp.value += 1
  try {
    home.value = await memoryHome(today())
    avatar.value = await avatarGet()
  } catch {
    // outside Castle's window
  }
  // a hidden room may not stay open
  if (!tabs.value.some((t) => t.id === active.value)) active.value = 'home'
}

async function loadBackup() {
  try {
    backup.value = await backupStatus()
  } catch {
    backup.value = null
  }
}

// A project or diary card opens its page in the DIARY or PROJECTS room.
function openMemoryFile(file) {
  if (!file) return
  const room = file.startsWith('projects/') ? 'projects' : 'diary'
  if (!tabs.value.some((t) => t.id === room)) return
  roomFile.value = file
  active.value = room
}

async function onBackupCard(mode) {
  if (mode === 'setup') {
    backupOpen.value = true
    return
  }
  try {
    backup.value = await backupNow()
  } catch {
    backupOpen.value = true
  }
}

let refresh
let stopBackupEvents
let stopStoryEvents
onMounted(async () => {
  await loadSettingsAndTheme()
  loadHome()
  loadBackup()
  // the memory changes as the companion writes: read it again every minute
  refresh = setInterval(loadHome, 60000)
  try {
    stopBackupEvents = await onBackupDone(() => loadBackup())
    stopStoryEvents = await onStoryOutput(() => {
      if (active.value !== 'story' && !marks.value.includes('story')) marks.value = [...marks.value, 'story']
    })
  } catch {
    // outside Castle's window
  }
})
onBeforeUnmount(() => {
  clearInterval(refresh)
  stopBackupEvents?.()
  stopStoryEvents?.()
})
watch(settingsStamp, () => {
  loadHome()
  loadBackup()
})
</script>

<template>
  <div class="castle">
    <TopBar
      :tabs="tabs"
      :active="active"
      :marks="marks"
      :mark-titles="{ story: 'The story has new lines', chat: 'New in the room' }"
      @select="active = $event; marks = marks.filter((m) => m !== $event)"
      @settings="openSettings"
      @hide="hideCastle().catch(() => {})"
    />
    <main class="screen">
      <div v-show="active === 'home'" class="home" :class="theme?.widgetStyle ?? 'glass'">
        <QuotaGlow v-if="theme?.quotaGlow" :theme="theme" :visible="active === 'home'" />
        <TerminalPanel
          :settings-stamp="settingsStamp"
          :companion="companion"
          :cli="cli"
          @settings="openSettings"
          @setup="openSetup"
        >
          <template #header><CompanionHeader :name="companion" :cli-label="cliLabel" :avatar="avatar" /></template>
        </TerminalPanel>
        <div class="widget-slot" :style="placeStyle(widgets.clock)"><ClockWidget :size="widgets.clock?.size ?? 'm'" /></div>
        <div class="widget-slot" :style="placeStyle(widgets.gauges)"><GaugeWidget /></div>
        <div v-for="kind in ['today', 'projects', 'diary', 'growth', 'backup']" :key="kind" class="widget-slot" :style="placeStyle(widgets[kind])">
          <HomeCards v-if="widgets[kind]" :kind="kind" :data="home" :backup="backup" @open="openMemoryFile" @backup="onBackupCard" />
        </div>
      </div>
      <AppsView v-if="active === 'apps'" class="room" />
      <ChatRoomView v-show="active === 'chat'" v-if="chatOpened" :visible="active === 'chat'" :companion="companion" @fresh="!marks.includes('chat') && (marks = [...marks, 'chat'])" />
      <WorldView v-if="active === 'world'" :active="active === 'world'" :companion="companion" @show-chat="active = 'home'" />
      <StoryView v-show="active === 'story'" v-if="storyOpened" :companion="companion" :member="memberName" :cli="cli" :visible="active === 'story'" />
      <AppearanceView v-if="active === 'appearance'" :companion="companion" :stamp="memoryStamp" @keys="openKeys" @show-chat="active = 'home'" />
      <StudioView v-if="active === 'studio'" :companion="companion" :stamp="memoryStamp" @keys="openKeys" />
      <ThemeView v-if="active === 'theme'" :companion="companion" @changed="loadSettingsAndTheme" @show-chat="active = 'home'" />
      <MemoryRoom v-if="['diary', 'projects', 'library'].includes(active)" :room="active" :stamp="memoryStamp" :open-file="roomFile" />
      <div v-if="!BUILT.has(active)" class="room-soon">
        <p>{{ activeLabel }}</p>
      </div>
      <p v-if="themeNotice" class="theme-notice">{{ themeNotice }}</p>
      <SettingsView
        v-if="settingsOpen"
        :section="settingsSection"
        :version="updateInfo?.version ?? ''"
        :update-line="updateInfo?.message ?? ''"
        @close="settingsOpen = false"
        @saved="settingsSaved"
        @setup="settingsOpen = false; openSetup()"
      />
      <ModalSheet v-if="backupOpen" tag="BACKUP" title="Keep the memory safe" @close="backupOpen = false; loadBackup()">
        <BackupPanel :companion="companion" />
      </ModalSheet>
      <NoticeToasts @keys="openKeys" />
      <UpdateChip @status="updateInfo = $event" />
      <SetupView v-if="setupOpen" :reopened="setupReopened" @done="setupDone" @close="setupOpen = false" />
    </main>
  </div>
</template>

<style>
:root {
  --bar-height: 48px;
  --background: #050b1f;
  --bar-background: rgba(6, 12, 34, 0.92);
  --line: rgba(150, 190, 255, 0.34);
  --text: #eef3ff;
  --text-dim: #a6b8de;
  --accent: #8fb8ff;
  --hover: rgba(150, 190, 255, 0.14);
  --panel-background: rgba(10, 20, 52, 0.78);
  --widget-background: rgba(12, 24, 60, 0.62);
  --glass-blur: 14px;
  --good: #8fe3b8;
  --warn: #ffd59a;
  --bad: #ff9aa8;
  --wallpaper: none;
  --wallpaper-position: center;
}

html,
body,
#app {
  height: 100%;
  margin: 0;
  overflow: hidden;
}

body {
  background: var(--background);
  color: var(--text);
  font-family: 'Segoe UI', system-ui, sans-serif;
}

.castle {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--background);
  background-image: var(--wallpaper);
  background-position: var(--wallpaper-position);
  background-size: cover;
  background-repeat: no-repeat;
}

.screen {
  position: relative;
  flex: 1;
  min-height: 0;
}

.home {
  position: absolute;
  inset: 0;
}

.widget-slot {
  position: absolute;
}

/* Widget frames: glass themes give them a pane, bare themes let the wallpaper show through. */
.home.glass .widget {
  background: var(--widget-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}

.home.bare .widget {
  background: transparent;
  text-shadow: 0 2px 10px rgba(0, 0, 0, 0.55);
}

.room {
  position: absolute;
  inset: 0;
}

.room-soon {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  color: var(--text-dim);
  letter-spacing: 0.3em;
}

.theme-notice {
  position: absolute;
  bottom: 12px;
  left: 50%;
  margin: 0;
  padding: 6px 14px;
  font-size: 13px;
  color: #07142e;
  background: var(--warn);
  border-radius: 8px;
  transform: translateX(-50%);
}
</style>
