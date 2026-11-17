<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import TopBar from './components/TopBar.vue'
import TerminalPanel from './components/TerminalPanel.vue'
import ClockWidget from './components/widgets/ClockWidget.vue'
import GaugeWidget from './components/widgets/GaugeWidget.vue'
import SetupView from './views/SetupView.vue'
import HomeCards from './components/widgets/HomeCards.vue'
import UsageWidget from './components/widgets/UsageWidget.vue'
import CreditWidget from './components/widgets/CreditWidget.vue'
import DatabaseOffer from './components/DatabaseOffer.vue'
import { shouldOffer } from './components/database.js'
import CompanionHeader from './components/CompanionHeader.vue'
import { onOpenChat, onProviderChanged } from './api/castle.js'
import { prepareStudio } from './composables/useStudio.js'
import { loaderBegin, loaderDone, loaderEnd, loaderExpect, loaderRemember } from './loader/loader.js'
import ModalSheet from './components/ModalSheet.vue'
import BackupPanel from './components/BackupPanel.vue'
import AppsView from './views/AppsView.vue'
import WidgetsView from './views/WidgetsView.vue'
import MyProfileView from './views/myprofile/MyProfileView.vue'
import MemoryRoom from './views/MemoryRoom.vue'
import LibraryView from './views/LibraryView.vue'
import MarketView from './views/MarketView.vue'
import ThemeView from './views/ThemeView.vue'
import StudioView from './views/StudioView.vue'
import AppearanceView from './views/AppearanceView.vue'
import StoryView from './views/StoryView.vue'
import WorldView from './views/WorldView.vue'
import ChatRoomView from './views/ChatRoomView.vue'
import DevView from './views/DevView.vue'
import SettingsView from './views/SettingsView.vue'
import UpdateChip from './components/UpdateChip.vue'
import QuotaGlow from './components/QuotaGlow.vue'
import UsageReveal from './components/UsageReveal.vue'
import WhatsNew from './components/WhatsNew.vue'
import { sectionFor, unseenSections } from './components/changelog.js'
import { moveTab, orderTabs } from './components/taborder.js'
import { SIZES, WIDGETS, dragBegins, dropPlace, isOn, memberPlace, placeFor, placePatch, resetPatch, sizeOf, startsOnControl, hasItsKey } from './components/widgets/widgetplaces.js'
import { isCreditWidget } from './components/widgets/credits.js'
import changelog from '../CHANGELOG.md?raw'
import { version as appVersion } from '../package.json'
import NoticeToasts from './components/NoticeToasts.vue'
import MeterAlert from './components/MeterAlert.vue'
import AmbientMusic from './components/AmbientMusic.vue'
import { avatarGet, backupNow, backupStatus, dbMemory, dbStart, dbStatus, getSettings, hideCastle, keyStatus, memoryHome, saveSettings, onBackupDone, onStoryOutput, userThemes, versionSeen } from './api/castle.js'
import { DEFAULT_THEME_ID, applyTheme, builtInThemes, chooseTheme, fromUserFolders, placeStyle } from './themes/loader.js'

// The rooms across the top. A room whose MemoryCore feature is not installed stays hidden
// (`needs`); the rest always show.
const allTabs = [
  { id: 'home', label: 'HOME' },
  { id: 'widgets', label: 'WIDGETS' },
  { id: 'theme', label: 'THEME' },
  { id: 'apps', label: 'APPS' },
  { id: 'myprofile', label: 'MY PROFILE' },
  { id: 'diary', label: 'DIARY', needs: 'diary' },
  { id: 'projects', label: 'PROJECTS', needs: 'projects' },
  { id: 'library', label: 'LIBRARY' },
  { id: 'appearance', label: 'APPEARANCE' },
  { id: 'studio', label: 'STUDIO' },
  { id: 'story', label: 'STORY' },
  { id: 'world', label: 'WORLD' },
  { id: 'chat', label: 'CHAT' },
  { id: 'dev', label: 'DEV' },
  { id: 'market', label: 'MARKET' },
]
// dev only: ?tab=<room> opens a room straight away (for checking a room in a browser)
const devTab = import.meta.env.DEV ? new URLSearchParams(location.search).get('tab') : null
const active = ref(devTab || 'home')
// What HOME shows (memory_home), the backup card, the portrait and the CLI chip.
const home = ref(null)
const backup = ref(null)
// the DATABASE card: { engine, memory }
const database = ref(null)
// the one-time offer after an update (memory still in files, no database yet)
const databaseOffer = ref(false)
const databaseBytes = ref(0)
const offerFacts = ref({ offerSeen: true, hadCastleBefore: false, hasFolder: false })
const avatar = ref(null)
const memoryFolder = ref('')
const backupOpen = ref(false)
// the Settings sheet and the section it opens on
const settingsOpen = ref(false)
const settingsSection = ref('companion')
// the update line for Settings > About
const updateInfo = ref(null)
// Rooms that are built; any other room shows its name until its phase lands.
const BUILT = new Set(['home', 'widgets', 'theme', 'apps', 'myprofile', 'diary', 'projects', 'library', 'studio', 'appearance', 'story', 'world', 'chat', 'dev', 'market'])
// Tabs with something new waiting (the story speaking while the member is elsewhere).
const marks = ref([])
const memberName = ref('')
const storyOpened = ref(active.value === 'story')
const chatOpened = ref(active.value === 'chat')
// DEV stays mounted once opened, so its project sessions keep going behind the other rooms
const devOpened = ref(active.value === 'dev')
watch(active, (room) => {
  if (room !== 'home') arranging.value = false
  if (room === 'story') storyOpened.value = true
  if (room === 'chat') chatOpened.value = true
  if (room === 'dev') devOpened.value = true
})
// a file a HOME card asked to open in its room
const roomFile = ref('')
const memoryStamp = ref(0)
// rooms the member switched off in Settings are not available (never removed)
const rooms = ref({})
const barClock = ref({ enabled: false, format: '24h' })
const usageCard = ref(true)
// HOME's widgets as the member arranged them (WIDGETS): on or off, size, and places per theme
const widgetPrefs = ref({})
const arranging = ref(false)
const drag = ref(null) // { id, x, y, dx, dy, box }
const homeEl = ref(null)
// the member's own tab order (dragged in the top bar); empty = Castle's order
const tabOrder = ref([])
const tabs = computed(() =>
  orderTabs(allTabs, tabOrder.value).filter((tab) => (!tab.needs || home.value?.features?.[tab.needs]) && (tab.id === 'home' || rooms.value[tab.id] !== false)),
)
// a room switched off closes: its tab goes, its mark clears, a kept-alive room unmounts
watch(tabs, (list) => {
  const ids = new Set(list.map((t) => t.id))
  if (!ids.has(active.value)) active.value = 'home'
  if (!ids.has('story')) storyOpened.value = false
  if (!ids.has('chat')) chatOpened.value = false
  if (!ids.has('dev')) devOpened.value = false
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
// The keys saved in Settings > Keys: a credit widget shows only once its key is there.
const savedKeys = ref([])
// The widgets HOME draws: switched on, placed by the theme (all but the clock and the gauges),
// and for a credit widget, its key saved.
const shownWidgets = computed(() =>
  WIDGETS.map((w) => w.id).filter(
    (id) => isOn(id, { ...widgetPrefs.value, showUsageCard: usageCard.value }) && (['clock', 'gauges'].includes(id) || widgets.value[id]) && hasItsKey(id, savedKeys.value),
  ),
)

function slotStyle(id) {
  const style = placeStyle(placeFor(widgets.value[id], memberPlace(widgetPrefs.value, theme.value?.id, id)))
  const scale = SIZES[sizeOf(id, widgetPrefs.value)]
  const moving = drag.value?.id === id ? `translate(${drag.value.dx}px, ${drag.value.dy}px) ` : ''
  if (moving || scale !== 1) style.transform = `${moving}scale(${scale})`
  return style
}

async function saveWidgets(patch) {
  widgetPrefs.value = { ...widgetPrefs.value, ...patch }
  try {
    const { settings } = await getSettings()
    await saveSettings({ ...settings, ...patch })
  } catch {
    // outside Castle's window: the arrangement holds until the page reloads
  }
}

// A widget follows the pointer and is put down where it is dropped, kept on HOME - in arrange mode
// at once, and on HOME itself once the pointer has moved a few pixels (a still press stays a click,
// and a press on a button, link or field never drags).
let pressed = null // { id, x, y, box, pointerId, el } until the drag begins
function startDrag(event, id) {
  if (event.button !== 0) return
  const tags = event.composedPath().filter((n) => n instanceof Element && event.currentTarget.contains(n)).map((n) => n.tagName)
  if (!arranging.value && startsOnControl(tags)) return
  const home = homeEl.value.getBoundingClientRect()
  const rect = event.currentTarget.getBoundingClientRect()
  pressed = { id, x: event.clientX, y: event.clientY, pointerId: event.pointerId, el: event.currentTarget, box: { left: rect.left - home.left, top: rect.top - home.top, width: rect.width, height: rect.height } }
  if (arranging.value) beginDrag()
}

function beginDrag() {
  pressed.el.setPointerCapture(pressed.pointerId)
  drag.value = { id: pressed.id, x: pressed.x, y: pressed.y, dx: 0, dy: 0, box: pressed.box }
}

function moveDrag(event) {
  if (pressed && !drag.value && dragBegins(event.clientX - pressed.x, event.clientY - pressed.y)) beginDrag()
  if (!drag.value) return
  drag.value = { ...drag.value, dx: event.clientX - drag.value.x, dy: event.clientY - drag.value.y }
}

async function endDrag() {
  pressed = null
  const d = drag.value
  if (!d) return
  drag.value = null
  if (!d.dx && !d.dy) return
  // the release after a drag is not a click on whatever card row it ended over
  const swallow = (e) => { e.stopPropagation(); e.preventDefault() }
  window.addEventListener('click', swallow, { capture: true, once: true })
  setTimeout(() => window.removeEventListener('click', swallow, { capture: true }), 0)
  const home = homeEl.value.getBoundingClientRect()
  const place = dropPlace({ ...d.box, left: d.box.left + d.dx, top: d.box.top + d.dy }, { width: home.width, height: home.height })
  await saveWidgets(placePatch(widgetPrefs.value, theme.value?.id, d.id, place))
}

function arrange() {
  active.value = 'home'
  arranging.value = true
}

const onArrangeKey = (event) => event.key === 'Escape' && arranging.value && (arranging.value = false)

// "What's new" after an update: the sections since the version last seen, once. The very first
// launch only records the version. dev only: ?whatsnew=<version> shows that version's notes.
const whatsNew = ref(null)
let whatsNewChecked = false
async function checkWhatsNew(lastSeen) {
  if (whatsNewChecked) return
  whatsNewChecked = true
  const devWhatsNew = import.meta.env.DEV ? new URLSearchParams(location.search).get('whatsnew') : null
  if (devWhatsNew) {
    const s = sectionFor(changelog, devWhatsNew)
    whatsNew.value = { version: devWhatsNew, sections: s ? [s] : [] }
    return
  }
  if (lastSeen === appVersion) return
  const sections = unseenSections(changelog, lastSeen, appVersion)
  try {
    await versionSeen(appVersion)
  } catch {
    return // outside Castle's window
  }
  if (sections.length) whatsNew.value = { version: appVersion, sections }
}

function showWhatsNew() {
  const s = sectionFor(changelog, appVersion)
  whatsNew.value = { version: appVersion, sections: s ? [s] : [] }
}

async function loadSettingsAndTheme() {
  let wanted = DEFAULT_THEME_ID
  let lastSeen = null
  loaderBegin('settings', 'Reading your settings', 'settings.json')
  try {
    const view = await getSettings()
    lastSeen = view.settings.lastSeenVersion ?? null
    offerFacts.value = { offerSeen: !!view.settings.dbOfferSeen, hadCastleBefore: lastSeen !== null, hasFolder: !!view.settings.memoryFolder }
    companion.value = view.companionLabel
    cli.value = view.settings.cli
    memoryFolder.value = view.settings.memoryFolder ?? ''
    memberName.value = view.settings.memberName ?? ''
    rooms.value = view.settings.rooms ?? {}
    barClock.value = { enabled: !!view.settings.displayClock, format: view.settings.clockFormat ?? '24h' }
    usageCard.value = view.settings.showUsageCard !== false
    widgetPrefs.value = { widgetsOff: view.settings.widgetsOff ?? [], widgetSizes: view.settings.widgetSizes ?? {}, widgetPlaces: view.settings.widgetPlaces ?? {} }
    tabOrder.value = view.settings.tabOrder ?? []
    try {
      savedKeys.value = (await keyStatus()).filter((k) => k.set).map((k) => k.name)
    } catch {
      // the keys stay as they were
    }
    if (!view.settings.setupDone) {
      setupOpen.value = true
      setupReopened.value = false
    }
    wanted = view.settings.theme ?? DEFAULT_THEME_ID
  } catch {
    // Outside Castle's window (a browser preview): defaults.
  }
  loaderEnd('settings')
  loaderBegin('themes', 'Choosing your theme', `themes/${wanted}`)
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
  loaderRemember(chosen.theme)
  loaderEnd('themes')
  if (!setupOpen.value) checkWhatsNew(lastSeen)
}

function openSettings(section = 'companion') {
  settingsSection.value = typeof section === 'string' ? section : 'companion'
  settingsOpen.value = true
}

// A tab dragged into a new place (or moved with Alt+arrow): the order covers every tab, the hidden
// rooms too, so a room switched back on returns to where the member left it.
async function reorderTabs({ from, to }) {
  const ids = orderTabs(allTabs, tabOrder.value).map((t) => t.id)
  const next = moveTab(ids, from, to)
  if (next === ids) return
  tabOrder.value = next
  try {
    const { settings } = await getSettings()
    await saveSettings({ ...settings, tabOrder: next })
  } catch {
    // outside Castle's window: the order holds until the page reloads
  }
}

function openKeys() {
  openSettings('keys')
}

// THEME's own saves (the main theme, and the diary/story/profile looks) go through here too, so
// every settingsStamp listener (HOME's PROFILE tab among them) hears about them the same as a
// save from Settings does.
async function themeChanged() {
  await loadSettingsAndTheme()
  settingsStamp.value += 1
}

async function settingsSaved() {
  await loadSettingsAndTheme()
  settingsStamp.value += 1
  // the database may have been installed, started or moved into from Settings
  loadDatabase()
  loadHome()
}

function openSetup() {
  setupReopened.value = true
  setupOpen.value = true
}

async function setupDone() {
  setupOpen.value = false
  // the setup's last step already showed the database panel: the one-time offer is not asked again
  try {
    const { settings } = await getSettings()
    if (!settings.dbOfferSeen) await saveSettings({ ...settings, dbOfferSeen: true })
  } catch {
    // outside Castle's window
  }
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

async function loadDatabase() {
  try {
    const status = await dbStatus()
    databaseBytes.value = status.downloadBytes
    database.value = { engine: status.engine, memory: await dbMemory() }
  } catch {
    database.value = null
    return
  }
  // asked once, after "What's new" has had its turn
  if (!whatsNew.value && !databaseOffer.value && shouldOffer({ ...offerFacts.value, setupOpen: setupOpen.value }, database.value.engine, database.value.memory)) {
    databaseOffer.value = true
  }
}

function closeDatabaseOffer() {
  databaseOffer.value = false
  offerFacts.value = { ...offerFacts.value, offerSeen: true }
  loadDatabase()
  loadHome()
}

// The DATABASE card: Start runs here; everything else opens Settings > Database.
async function onDatabaseCard(action) {
  if (action === 'start') {
    try {
      await dbStart()
    } catch {
      openSettings('database')
    }
    await loadDatabase()
    loadHome()
    return
  }
  openSettings('database')
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
  // a page of the database opens under db/project/... or db/diary/...
  const room = file.startsWith('projects/') || file.startsWith('db/project/') ? 'projects' : 'diary'
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
let stopProviderEvents
let stopOpenChat
onMounted(async () => {
  window.addEventListener('keydown', onArrangeKey)
  stopProviderEvents = await onProviderChanged(async () => { await settingsSaved(); window.dispatchEvent(new Event('castle-provider-changed')) }).catch(() => null)
  stopOpenChat = await onOpenChat(() => {
    active.value = 'chat'
    marks.value = marks.value.filter((m) => m !== 'chat')
  }).catch(() => null)
  // the start-up screen reports what is loading, then goes when it is all in (or after a wait too long)
  loaderExpect(6)
  await loadSettingsAndTheme()
  const piece = (id, label, file, work) => {
    loaderBegin(id, label, file)
    return Promise.resolve(work()).catch(() => {}).finally(() => loaderEnd(id))
  }
  const folder = memoryFolder.value.split(String.fromCharCode(92)).join('/').split('/').filter(Boolean).pop() ?? ''
  const loading = Promise.allSettled([
    piece('memory', 'Reading your memory', folder, loadHome),
    piece('backup', 'Checking the backup', 'backup status', loadBackup),
    piece('database', 'Waking the memory database', 'memory database', loadDatabase),
    piece('studio', "Preparing STUDIO's gallery", 'media-generation/', () => prepareStudio((done, total) => loaderBegin('studio', 'Making small copies of your pictures', `${done} / ${total}`))),
  ])
  // long enough for a first start to make a large gallery's small copies; after that it is quick
  await Promise.race([loading, new Promise((resolve) => setTimeout(resolve, 25000))])
  loaderDone()
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
  window.removeEventListener('keydown', onArrangeKey)
  stopProviderEvents?.()
  stopOpenChat?.()
  clearInterval(refresh)
  stopBackupEvents?.()
  stopStoryEvents?.()
})
watch(settingsStamp, () => {
  loadHome()
  loadBackup()
  loadDatabase()
})
</script>

<template>
  <div class="castle">
    <TopBar
      :tabs="tabs"
      :active="active"
      :display-clock="barClock.enabled"
      :version="appVersion"
      :clock-format="barClock.format"
      :marks="marks"
      :mark-titles="{ story: 'The story has new lines', chat: 'New in the room' }"
      @select="active = $event; marks = marks.filter((m) => m !== $event)"
      @reorder="reorderTabs"
      @settings="openSettings"
      @hide="hideCastle().catch(() => {})"
    />
    <main class="screen">
      <div v-show="active === 'home'" ref="homeEl" class="home" :class="[theme?.widgetStyle ?? 'glass', { arranging }]">
        <UsageReveal v-if="theme?.usageReveal" :theme="theme" :visible="active === 'home'" />
        <QuotaGlow v-if="theme?.quotaGlow" :theme="theme" :visible="active === 'home'" />
        <TerminalPanel
          :settings-stamp="settingsStamp"
          :companion="companion"
          :cli="cli"
          @settings="openSettings"
          @setup="openSetup"
        >
          <template #header="{ running, busy }"><CompanionHeader :name="companion" :avatar="avatar" :running="running" :busy="busy" /></template>
        </TerminalPanel>
        <div
          v-for="id in shownWidgets"
          :key="id"
          class="widget-slot"
          :class="{ movable: arranging, moving: drag?.id === id }"
          :style="slotStyle(id)"
          @pointerdown="startDrag($event, id)"
          @pointermove="moveDrag"
          @pointerup="endDrag"
          @pointercancel="endDrag"
        >
          <ClockWidget v-if="id === 'clock'" :size="widgets.clock?.size ?? 'm'" />
          <GaugeWidget v-else-if="id === 'gauges'" />
          <UsageWidget v-else-if="id === 'usage'" :stamp="settingsStamp" />
          <CreditWidget v-else-if="isCreditWidget(id)" :widget="id" />
          <HomeCards v-else :kind="id" :data="home" :backup="backup" :database="database" @open="openMemoryFile" @backup="onBackupCard" @database="onDatabaseCard" />
        </div>
        <div v-if="arranging" class="arrange-bar" role="toolbar" aria-label="Arrange HOME">
          <span>Drag a widget to move it. Each theme keeps its own places.</span>
          <button @click="saveWidgets(resetPatch(widgetPrefs, theme?.id))">Back to the theme's places</button>
          <button class="primary" @click="arranging = false">Done</button>
        </div>
      </div>
      <AppsView v-if="active === 'apps'" class="room" />
      <WidgetsView v-if="active === 'widgets'" :theme-id="theme?.id ?? ''" :theme-name="theme?.name ?? ''" :theme-widgets="widgets" @arrange="arrange" @changed="loadSettingsAndTheme" />
      <MyProfileView v-if="active === 'myprofile'" :companion="companion" @settings="openSettings('database')" />
      <ChatRoomView v-show="active === 'chat'" v-if="chatOpened" :visible="active === 'chat'" :companion="companion" :settings-stamp="settingsStamp" @settings="openSettings" @renamed="settingsSaved" @fresh="!marks.includes('chat') && (marks = [...marks, 'chat'])" />
      <DevView v-show="active === 'dev'" v-if="devOpened" :visible="active === 'dev'" :companion="companion" :cli="cli" :settings-stamp="settingsStamp" @settings="openSettings" @setup="openSetup" />
      <WorldView v-if="active === 'world'" :active="active === 'world'" :companion="companion" @show-chat="active = 'home'" />
      <StoryView v-show="active === 'story'" v-if="storyOpened" :companion="companion" :member="memberName" :cli="cli" :visible="active === 'story'" />
      <AppearanceView v-if="active === 'appearance'" :companion="companion" :stamp="memoryStamp" @keys="openKeys" />
      <StudioView v-if="active === 'studio'" :companion="companion" :stamp="memoryStamp" @keys="openKeys" />
      <ThemeView v-if="active === 'theme'" :companion="companion" @changed="themeChanged" @show-chat="active = 'home'" />
      <MemoryRoom v-if="['diary', 'projects'].includes(active)" :room="active" :stamp="memoryStamp" :open-file="roomFile" />
      <LibraryView v-if="active === 'library'" :companion="companion" :stamp="memoryStamp" />
      <MarketView v-if="active === 'market'" :companion="companion" />
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
        @whatsnew="showWhatsNew"
      />
      <WhatsNew v-if="whatsNew" :version="whatsNew.version" :sections="whatsNew.sections" @close="whatsNew = null; loadDatabase()" />
      <DatabaseOffer v-if="databaseOffer" :companion="companion" :download-bytes="databaseBytes" @close="closeDatabaseOffer" @changed="loadHome" />
      <ModalSheet v-if="backupOpen" tag="BACKUP" title="Keep the memory safe" @close="backupOpen = false; loadBackup()">
        <BackupPanel :companion="companion" />
      </ModalSheet>
      <NoticeToasts @keys="openKeys" />
      <MeterAlert />
      <AmbientMusic />
      <UpdateChip @status="updateInfo = $event" @about="openSettings('about')" />
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
  transform-origin: top left;
}

/* Arrange mode: every widget shows its edge and follows the pointer; nothing inside is pressed. */
.widget-slot.movable {
  cursor: grab;
  outline: 2px dashed color-mix(in srgb, var(--accent) 70%, transparent);
  outline-offset: 4px;
  border-radius: 14px;
  touch-action: none;
}

.widget-slot.movable > * {
  pointer-events: none;
}

.widget-slot.moving {
  cursor: grabbing;
  user-select: none;
  z-index: 5;
  opacity: 0.9;
}

.arrange-bar {
  position: absolute;
  left: 50%;
  bottom: 18px;
  transform: translateX(-50%);
  z-index: 6;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 12px;
  backdrop-filter: blur(var(--glass-blur));
  color: var(--text);
  font-size: 0.9rem;
}

.arrange-bar button {
  font: inherit;
  color: var(--text);
  background: var(--hover);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 6px 12px;
  cursor: pointer;
}

.arrange-bar button.primary {
  background: var(--accent);
  color: #07142e;
  border-color: transparent;
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
