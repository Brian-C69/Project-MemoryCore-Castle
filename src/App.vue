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
import { avatarGet, backupNow, backupStatus, chatMediaOpen, getSettings, hideCastle, memoryHome, onBackupDone, userThemes } from './api/castle.js'
import { DEFAULT_THEME_ID, applyTheme, builtInThemes, chooseTheme, fromUserFolders, placeStyle } from './themes/loader.js'

// The rooms across the top. A room whose MemoryCore feature is not installed stays hidden
// (`needs`); the rest always show.
const allTabs = [
  { id: 'home', label: 'HOME' },
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
const active = ref('home')
// What HOME shows (memory_home), the backup card, the portrait and the CLI chip.
const home = ref(null)
const backup = ref(null)
const avatar = ref(null)
const cliLabel = ref('Claude Code')
const memoryFolder = ref('')
const backupOpen = ref(false)
const tabs = computed(() => allTabs.filter((tab) => !tab.needs || home.value?.features?.[tab.needs]))
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
  const all = [...builtInThemes(), ...fromUserFolders(folders).themes]
  const chosen = chooseTheme(all, wanted)
  theme.value = chosen.theme
  themeNotice.value = chosen.notice
  applyTheme(chosen.theme)
}

function openSettings() {
  // The Settings sheet arrives in Phase 12; until then the gear reopens setup.
  openSetup()
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

// A project or diary file opens in its own app (fenced to the memory folder); the DIARY and
// PROJECTS rooms read them inside Castle.
function openMemoryFile(file) {
  if (!file || !memoryFolder.value) return
  chatMediaOpen(`${memoryFolder.value.replace(/[\\/]+$/, '')}\\${file.replace(/\//g, '\\')}`).catch(() => {})
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
onMounted(async () => {
  await loadSettingsAndTheme()
  loadHome()
  loadBackup()
  // the memory changes as the companion writes: read it again every minute
  refresh = setInterval(loadHome, 60000)
  try {
    stopBackupEvents = await onBackupDone(() => loadBackup())
  } catch {
    // outside Castle's window
  }
})
onBeforeUnmount(() => {
  clearInterval(refresh)
  stopBackupEvents?.()
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
      @select="active = $event"
      @settings="openSettings"
      @hide="hideCastle().catch(() => {})"
    />
    <main class="screen">
      <div v-show="active === 'home'" class="home" :class="theme?.widgetStyle ?? 'glass'">
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
      <div v-if="active !== 'home'" class="room-soon">
        <p>{{ activeLabel }}</p>
      </div>
      <p v-if="themeNotice" class="theme-notice">{{ themeNotice }}</p>
      <ModalSheet v-if="backupOpen" tag="BACKUP" title="Keep the memory safe" @close="backupOpen = false; loadBackup()">
        <BackupPanel :companion="companion" />
      </ModalSheet>
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
