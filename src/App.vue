<script setup>
import { computed, onMounted, ref } from 'vue'
import TopBar from './components/TopBar.vue'
import TerminalPanel from './components/TerminalPanel.vue'
import ClockWidget from './components/widgets/ClockWidget.vue'
import GaugeWidget from './components/widgets/GaugeWidget.vue'
import SetupView from './views/SetupView.vue'
import { getSettings, hideCastle, userThemes } from './api/castle.js'
import { DEFAULT_THEME_ID, applyTheme, builtInThemes, chooseTheme, fromUserFolders, placeStyle } from './themes/loader.js'

// The rooms across the top. Later phases fill each one and hide the rooms whose
// MemoryCore feature is not installed.
const tabs = [
  { id: 'home', label: 'HOME' },
  { id: 'apps', label: 'APPS' },
  { id: 'diary', label: 'DIARY' },
  { id: 'projects', label: 'PROJECTS' },
  { id: 'library', label: 'LIBRARY' },
  { id: 'appearance', label: 'APPEARANCE' },
  { id: 'studio', label: 'STUDIO' },
  { id: 'story', label: 'STORY' },
  { id: 'world', label: 'WORLD' },
  { id: 'chat', label: 'CHAT' },
]
const active = ref('home')
const companion = ref('Your companion')
const cli = ref('claude')
const settingsStamp = ref(0)
// The first-run setup: open until it is done; reopened from the panel or the gear.
const setupOpen = ref(false)
const setupReopened = ref(false)
const theme = ref(null)
const themeNotice = ref('')
const activeLabel = computed(() => tabs.find((t) => t.id === active.value)?.label ?? '')
const widgets = computed(() => theme.value?.widgets ?? {})

async function loadSettingsAndTheme() {
  let wanted = DEFAULT_THEME_ID
  try {
    const view = await getSettings()
    companion.value = view.companionLabel
    cli.value = view.settings.cli
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

onMounted(loadSettingsAndTheme)
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
        />
        <div class="widget-slot" :style="placeStyle(widgets.clock)"><ClockWidget :size="widgets.clock?.size ?? 'm'" /></div>
        <div class="widget-slot" :style="placeStyle(widgets.gauges)"><GaugeWidget /></div>
      </div>
      <div v-if="active !== 'home'" class="room-soon">
        <p>{{ activeLabel }}</p>
      </div>
      <p v-if="themeNotice" class="theme-notice">{{ themeNotice }}</p>
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
