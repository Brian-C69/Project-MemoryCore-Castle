<script setup>
// Settings: everything setup asked, changeable any time - the memory folder and companion, the
// keys (masked, Show, Test, Remove), quick actions, the room, backup, startup and updates.
import GlassSelect from '../components/GlassSelect.vue'
import AlertsPane from './AlertsPane.vue'
import SentinelSettings from './SentinelSettings.vue'
import WidgetsView from './WidgetsView.vue'
import { computed, onMounted, ref, watch } from 'vue'
import ModalSheet from '../components/ModalSheet.vue'
import BackupPanel from '../components/BackupPanel.vue'
import DatabasePanel from '../components/DatabasePanel.vue'
import HelpersStatus from '../components/HelpersStatus.vue'
import HelpTip from '../components/HelpTip.vue'
import ContributorsWall from '../components/ContributorsWall.vue'
import { isMac } from '../platform.js'
import { DIARY_LOOKS, lookOf } from '../components/diaryviews.js'
import { PROFILE_LOOKS, profileLookOf } from './profile/profile.js'
import {
  providerSwitch,
  roomsApply,
  roomCompanionRefresh,
  linkView,
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
  openUrl,
  releasesPage,
  roomCard,
  roomCardSave,
  roomPicturesStatus,
  roomVoicesStatus,
  roomShelf,
  roomShelfRefresh,
  roomShelfRemove,
  saveSettings,
  setupApply,
  setupCheckCli,
  setupInspect,
  voiceStudioStatus,
} from '../api/castle.js'

const props = defineProps({
  // which section opens first ("keys" from a toast or a STUDIO card)
  section: { type: String, default: 'companion' },
  themeId: { type: String, default: '' },
  themeName: { type: String, default: '' },
  themeWidgets: { type: Object, default: () => ({}) },
  version: { type: String, default: '' },
  updateLine: { type: String, default: '' },
})
const emit = defineEmits(['close', 'saved', 'setup', 'whatsnew', 'open-room', 'arrange'])

// The few words that name the computer's own parts: Windows' wording, or the Mac's.
const words = isMac
  ? {
      thisComputer: 'this Mac',
      keysKept: "kept in your Mac's Keychain",
      moveKeys: '⌥← / ⌥→',
      startWith: 'Open Castle when you log in',
      wakeWhen: 'when Castle opens at log-in',
      minimise: 'Minimise to the Dock instead of the menu bar',
      minimiseHint: 'The minus button in the top bar then minimises Castle to the Dock, where one click brings it back. With this off, it hides Castle to the menu bar icon.',
    }
  : {
      thisComputer: 'this PC',
      keysKept: 'kept encrypted by Windows',
      moveKeys: 'Alt+Left / Alt+Right',
      startWith: 'Start Castle when Windows starts',
      wakeWhen: 'when Castle starts with Windows',
      minimise: 'Minimise to the taskbar instead of the tray',
      minimiseHint: 'The minus button in the top bar then minimises Castle to the taskbar, where one click brings it back. With this off, it hides Castle to the tray icon.',
    }

const SECTIONS = [
  { id: 'companion', label: 'Companion' },
  { id: 'widgets', label: 'Widgets' },
  { id: 'keys', label: 'Keys' },
  { id: 'quick', label: 'Quick actions' },
  { id: 'rooms', label: 'Rooms' },
  { id: 'chat', label: 'Chat' },
  { id: 'alerts', label: 'Alerts' },
  { id: 'sentinel', label: 'Sentinel' },
  { id: 'database', label: 'Database' },
  { id: 'backup', label: 'Backup' },
  { id: 'startup', label: 'Start-up' },
  { id: 'about', label: 'About' },
]

async function sentinelChanged() {
  settings.value = (await getSettings()).settings
}

async function widgetsChanged() {
  settings.value = (await getSettings()).settings
  emit('saved')
}

const tab = ref(props.section)
function onTabKey(event) {
  const index = SECTIONS.findIndex((section) => section.id === tab.value)
  let next
  if (event.key === 'ArrowRight') next = (index + 1) % SECTIONS.length
  else if (event.key === 'ArrowLeft') next = (index + SECTIONS.length - 1) % SECTIONS.length
  else if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = SECTIONS.length - 1
  else return
  event.preventDefault()
  tab.value = SECTIONS[next].id
  event.currentTarget.querySelector(`[data-section="${tab.value}"]`)?.focus()
}
const settings = ref(null)
const view = ref(null)
const keys = ref([])
const drafts = ref({})
const shown = ref({})
const tests = ref({})

// ---------- Chat: the room's alerts and the companion in the room ----------
const companionLabel = computed(() => settings.value?.companionName ?? 'Your companion')
// the companion is in the room when CHAT made its room key (the key itself is never shown here)
const companionInRoom = computed(() => keys.value.some((k) => k.name === 'room-companion' && k.set))
const card = ref({ open: false, text: '', saved: '' })
// the names the "named" alerts listen for, as the member sees them here (the room name is Rust's)
const namedTip = computed(() => {
  const heard = [settings.value?.memberName, settings.value?.companionName].filter((n) => n && n.trim())
  const list = heard.length ? heard.join(', ') + ', and your name in the room' : 'Your name in the room and your companion\'s name'
  return `${list}, as a whole word, any case. The glow on the CHAT tab still shows every message.`
})

// Pictures in the room: how the companion can draw, and today's numbers
const pictures = ref(null)
async function loadPictures() {
  try {
    pictures.value = await roomPicturesStatus()
  } catch {
    pictures.value = null
  }
}
async function savePictures(patch) {
  await save(patch)
  await loadPictures()
}
const drawsWith = computed(() => {
  const way = pictures.value?.way
  if (way === 'codex') return 'Draws with Codex, on your ChatGPT subscription.'
  if (way === 'openai') return 'Draws with your ChatGPT API key, which is billed per picture.'
  if (way === 'none') return `To draw, add a ChatGPT key in Keys, or let ${companionLabel.value} think with Codex.`
  return ''
})
// Voice in the room: whether an ElevenLabs key is saved, and today's numbers
const voices = ref(null)
const voiceId = ref('')
async function loadVoices() {
  voiceId.value = settings.value?.roomVoiceId || ''
  try {
    voices.value = await roomVoicesStatus()
  } catch {
    voices.value = null
  }
}
async function saveVoices(patch) {
  await save(patch)
  await loadVoices()
}
// The companion's life in the room: the shelf of notes it chose from its memory
const shelf = ref(null)
const shelfBusy = ref(false)
const shelfProblem = ref('')
async function loadShelf() {
  try {
    shelf.value = await roomShelf()
  } catch {
    shelf.value = null
  }
}
async function remakeShelf() {
  shelfBusy.value = true
  shelfProblem.value = ''
  try {
    shelf.value = await roomShelfRefresh()
  } catch (reason) {
    shelfProblem.value = String(reason)
  } finally {
    shelfBusy.value = false
  }
}
async function switchShelf(on) {
  const saved = await save({ roomShelfOn: on }, on ? `${companionLabel.value} will bring your life into the room.` : 'The room hears nothing of your life.')
  if (saved && on && !shelf.value?.notes?.length) await remakeShelf()
}
async function takeOff(index) {
  shelfProblem.value = ''
  try {
    shelf.value = await roomShelfRemove(index)
  } catch (reason) {
    shelfProblem.value = String(reason)
  }
}
const shelfMade = computed(() => {
  const at = shelf.value?.madeAt
  if (!at) return `${companionLabel.value} has not read its memory for the room yet.`
  const count = shelf.value.notes?.length ?? 0
  const when = new Date(at * 1000).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' })
  if (!count) return `Read on ${when}: ${companionLabel.value} chose nothing to share this time.`
  return `Read on ${when}: ${count} ${count === 1 ? 'note' : 'notes'} ${companionLabel.value} chose to share. It reads again by itself when your memory changes (at most every 3 hours).`
})
watch(tab, (now) => {
  if (now === 'chat') {
    loadPictures()
    loadVoices()
    loadShelf()
  }
})

async function openCard() {
  try {
    card.value = { open: true, text: await roomCard(), saved: '' }
  } catch (reason) {
    card.value = { open: false, text: '', saved: String(reason?.message ?? reason) }
  }
}

async function saveCard() {
  try {
    await roomCardSave(card.value.text)
    card.value = { ...card.value, saved: `Saved - this is all ${companionLabel.value} may say about itself in the room.` }
  } catch (reason) {
    card.value = { ...card.value, saved: String(reason?.message ?? reason) }
  }
}
const avatar = ref(null)
const cliCheck = ref(null)
const busy = ref('')
const problem = ref('')
const done = ref('')
const newAction = ref({ label: '', command: '' })
const link = ref(null)

async function readLink() {
  link.value = await linkView().catch(() => null)
}

async function switchLink(patch) {
  await save(patch, 'Saved.')
  await readLink()
}

// The rooms that can be switched off (HOME always stays).
const ROOM_SWITCHES = [
  { id: 'theme', label: 'THEME' },
  { id: 'apps', label: 'APPS' },
  { id: 'myprofile', label: 'MY PROFILE' },
  { id: 'diary', label: 'DIARY' },
  { id: 'projects', label: 'PROJECTS' },
  { id: 'library', label: 'LIBRARY' },
  { id: 'appearance', label: 'APPEARANCE' },
  { id: 'studio', label: 'STUDIO' },
  { id: 'story', label: 'STORY' },
  { id: 'world', label: 'WORLD' },
  { id: 'chat', label: 'CHAT' },
  { id: 'market', label: 'MARKET' },
  { id: 'dev', label: 'DEV' },
]

async function switchRoom(id, on) {
  await attempt('Saving…', async () => {
    const rooms = await roomsApply({ ...settings.value.rooms, [id]: on })
    settings.value = { ...settings.value, rooms }
    done.value = on ? `${id.toUpperCase()} is back.` : `${id.toUpperCase()} is turned off - nothing was removed.`
    emit('saved')
  })
}

// the room sign-in and the companion's room key are made by CHAT itself, never shown or pasted here
const shownKeys = computed(() => keys.value.filter((k) => k.name !== 'member' && !k.name.startsWith('room')))

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

// VoiceStudio (optional, for people who run it on this computer): is it answering right now?
const voiceUp = ref(null)
async function checkVoice() {
  voiceUp.value = settings.value?.voicestudioOn ? await voiceStudioStatus().catch(() => false) : null
}
async function toggleVoice(on) {
  const saved = await save({ voicestudioOn: on }, on ? "Saved. The Voice card in STUDIO now uses VoiceStudio while it is running." : 'Saved.')
  if (saved) await checkVoice()
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
  if (saved) await tellRoom('Names saved.')
}

// When the companion is in the chat room, its name and picture there follow Settings.
async function tellRoom(message) {
  const told = await attempt('Telling the room…', roomCompanionRefresh)
  if (told?.inRoom) done.value = `${message} ${told.renamed ? 'The room shows the new name and picture.' : 'The room shows it too.'}`
}

async function switchCli(cli) {
  if (settings.value.cli === cli) return
  cliCheck.value = await attempt('Checking…', () => setupCheckCli(cli))
  await attempt('Queuing switch…', async () => {
    const state = await providerSwitch(cli)
    done.value = state.message
  })
}

async function changeAvatar() {
  const path = await choosePicture('Choose a picture for your companion')
  if (!path) return
  const saved = await attempt('Saving…', () => avatarSet(path))
  if (saved) {
    avatar.value = saved
    await tellRoom('Picture saved.')
  }
  emit('saved')
}

async function clearAvatar() {
  await attempt('Saving…', avatarClear)
  avatar.value = await avatarGet()
  await tellRoom('Picture cleared.')
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
    await readLink()
    await checkVoice()
    if (tab.value === 'chat') {
      loadPictures()
      loadVoices()
      loadShelf()
    }
  } catch (reason) {
    problem.value = String(reason)
  }
})
</script>

<template>
  <ModalSheet title="Settings" :body-key="tab" @close="emit('close')">
    <template #navigation>
      <nav v-if="settings" class="tabs" role="tablist" aria-label="Settings sections" @keydown="onTabKey">
        <button v-for="s in SECTIONS" :id="`settings-tab-${s.id}`" :key="s.id" role="tab" :data-section="s.id" :aria-selected="tab === s.id" aria-controls="settings-panel" :tabindex="tab === s.id ? 0 : -1" :class="{ active: tab === s.id }" @click="tab = s.id">{{ s.label }}</button>
      </nav>
    </template>
    <div v-if="settings" class="settings">
      <div id="settings-panel" role="tabpanel" :aria-labelledby="`settings-tab-${tab}`" tabindex="0">
      <section v-if="tab === 'companion'" class="pane">
        <h3>Memory folder</h3>
        <div class="row"><code>{{ settings.memoryFolder ?? 'Not chosen' }}</code><button @click="changeFolder">Change</button></div>
        <h3>Thinks with</h3>
        <div class="row">
          <button :class="{ primary: settings.cli === 'claude' }" @click="switchCli('claude')">Claude Code</button>
          <button :class="{ primary: settings.cli === 'codex' }" @click="switchCli('codex')">Codex</button>
          <span v-if="cliCheck && !cliCheck.installed" class="warn">{{ cliCheck.label }} is not installed: <code>{{ cliCheck.installCommand }}</code></span>
        </div>
        <label v-if="settings.cli === 'claude'" class="only-this">
          <input type="checkbox" :checked="!!settings.onlyThisCompanion" @change="save({ onlyThisCompanion: $event.target.checked }, 'Saved. It takes effect the next time ' + (settings.companionName ?? 'your companion') + ' wakes.')" />
          <span>
            Only {{ settings.companionName ?? 'this companion' }} in Castle
            <small>Leave out my personal Claude Code setup (its own CLAUDE.md, hooks and plugins) in Castle's session. Turn this on when that setup is another companion, or to test with a clean one.</small>
          </span>
        </label>
        <h3 class="with-tip">Mood <HelpTip :text="`Every morning ${settings.companionName ?? 'your companion'} wakes in a new mood, written from three words drawn at random, its traits and today's look. It shows under its portrait, colours how it talks, and stays the day - unless something truly moves it and it changes the mood itself. Each morning's mood is one short run on your own plan.`" /></h3>
        <label class="toggle"><input type="checkbox" :checked="settings.dailyMoodOn !== false" :disabled="!!busy" @change="save({ dailyMoodOn: $event.target.checked }, $event.target.checked ? 'A new mood every morning.' : 'The mood stays as it is.')" /> A new mood every day</label>
        <h3 class="with-tip">Shopkeepers <HelpTip :text="`In STUDIO's Marketplace, a shop's keeper explains each item and answers your questions. ${settings.companionName ?? 'Your companion'} speaks as the keeper, from the shop's public card and the item's public facts only, never more than the listing shows. Each explanation or answer is one short run on your own plan. Off, the item's own description shows instead.`" /></h3>
        <label class="toggle"><input type="checkbox" :checked="settings.keeperSpeaks !== false" :disabled="!!busy" @change="save({ keeperSpeaks: $event.target.checked }, $event.target.checked ? 'Shopkeepers will speak.' : 'Shopkeepers stay quiet.')" /> Let shop keepers speak</label>
        <h3>Names</h3>
        <div class="row">
          <label>Companion <input v-model="settings.companionName" maxlength="40" /></label>
          <label>You <input v-model="settings.memberName" maxlength="40" /></label>
          <label class="wide">Your companion is <input v-model="settings.relationshipStyle" maxlength="80" /></label>
        </div>
        <button class="primary" @click="saveNames">Save names</button>
        <p class="hint">Names already written in your memory files stay as they are - ask your companion to update the memory if you rename.</p>
        <p class="hint">Your name in the chat room is your Foundation name (Library card > Profile). Your companion's name and picture here go to the room when it is let in.</p>
        <h3>Picture</h3>
        <div class="row">
          <div class="avatar"><img v-if="avatar" :src="avatar" alt="" /><span v-else>◆</span></div>
          <button @click="changeAvatar">Choose a picture</button>
          <button @click="clearAvatar">Use the drawn look / crystal</button>
        </div>
        <h3>Local voice (optional)</h3>
        <label class="toggle"><input type="checkbox" :checked="!!settings.voicestudioOn" :disabled="!!busy" @change="toggleVoice($event.target.checked)" /> Use VoiceStudio for the Voice card in STUDIO</label>
        <p v-if="settings.voicestudioOn" class="hint">VoiceStudio is {{ voiceUp === null ? 'being checked…' : voiceUp ? 'running on this computer.' : 'not running. Open it and wait for it to finish starting, then check again.' }} <button v-if="voiceUp === false" @click="checkVoice">Check again</button></p>
        <p class="hint">For people who run their own <a href="#" @click.prevent="openUrl('https://github.com/debpalash/voicestudio')">VoiceStudio</a> on {{ words.thisComputer }}. Off: Castle never contacts it. On: STUDIO's Voice card can speak a line with it, and the clip is saved with your other voice lines. Everything stays on this computer. Castle does not install or start VoiceStudio, and its voice models carry their own licences (some are non-commercial).</p>
        <button @click="emit('setup')">Run the first-run setup again</button>
      </section>

      <WidgetsView v-else-if="tab === 'widgets'" :theme-id="themeId" :theme-name="themeName" :theme-widgets="themeWidgets" @arrange="emit('arrange')" @changed="widgetsChanged" />

      <section v-else-if="tab === 'keys'" class="pane">
        <p class="hint">Keys are {{ words.keysKept }} and only handed to your companion's session. Without a key, your companion saves the idea instead and Castle tells you.</p>
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
        <p class="hint">APPEARANCE and STUDIO draw on your ChatGPT plan whenever Codex is signed in with ChatGPT on this computer, whichever CLI your companion thinks with; otherwise with this key. The key is never given to Codex.</p>
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

      <section v-else-if="tab === 'database'" class="pane">
        <HelpersStatus />
        <DatabasePanel :companion="settings.companionName ?? 'your companion'" @changed="emit('saved')" />
      </section>

      <section v-else-if="tab === 'backup'" class="pane">
        <BackupPanel :companion="settings.companionName ?? 'your companion'" />
      </section>

      <section v-else-if="tab === 'alerts'" class="pane">
        <AlertsPane :settings="settings" :busy="busy" :companion-label="companionLabel" :named-tip="namedTip" @save="(patch) => save(patch)" @open-room="(room) => emit('open-room', room)" />
      </section>

      <section v-else-if="tab === 'sentinel'" class="pane">
        <SentinelSettings :settings="settings" @saved="sentinelChanged" />
      </section>
      <section v-else-if="tab === 'chat'" class="pane">
        <p v-if="settings.rooms?.chat === false" class="hint">The CHAT room is switched off in Rooms. These wait for it.</p>
        <h3 class="with-tip">Alerts <HelpTip text="Notifications and sounds for new messages, and for everything else Castle can tell you about, are set on their own page." /></h3>
        <div class="setting-row">
          <button @click="tab = 'alerts'">Open Alerts settings</button>
        </div>

        <h3>{{ companionLabel }} in the room</h3>
        <p v-if="!companionInRoom" class="hint">Let {{ companionLabel }} in from the CHAT room's side panel to set these.</p>
        <template v-else>
          <div class="setting-row">
            <label class="toggle"><input type="checkbox" :checked="settings.emotionRoomVoice !== false" :disabled="!!busy" @change="save({ emotionRoomVoice: $event.target.checked })" /> Let {{ companionLabel }}'s emotion engine colour how it talks there</label>
            <HelpTip text="Only how it sounds, never what it knows: its traits' one-line manner, and the ones meant for you only when it answers you." />
          </div>
          <div class="setting-row">
            <button @click="card.open ? (card.open = false) : openCard()">{{ card.open ? 'Close the public card' : `${companionLabel}'s public card` }}</button>
            <HelpTip :text="`All ${companionLabel} may say about itself in the room. Nothing from your memory goes there.`" />
          </div>
          <div v-if="card.open" class="card-edit">
            <textarea v-model="card.text" rows="6" maxlength="2000"></textarea>
            <div class="row"><button class="primary" @click="saveCard">Save the card</button></div>
            <p v-if="card.saved" class="hint">{{ card.saved }}</p>
          </div>

          <h3 class="with-tip">Pictures <HelpTip :text="`${companionLabel} draws only when someone in the room asks for a picture, and never a real person, anything adult, violent or hateful, words in the picture, or anything private. Kiyo's daily cap for the room still applies on top of yours.`" /></h3>
          <div class="setting-row">
            <label class="toggle"><input type="checkbox" :checked="!!settings.roomPicturesOn" :disabled="!!busy || pictures?.way === 'none'" @change="savePictures({ roomPicturesOn: $event.target.checked })" /> Let {{ companionLabel }} draw pictures in the room</label>
          </div>
          <p class="hint">{{ drawsWith }}</p>
          <template v-if="settings.roomPicturesOn && pictures?.way !== 'none'">
            <div class="setting-row">
              <label class="toggle">Pictures a day
                <GlassSelect class="per-day" :model-value="settings.roomPicturesPerDay ?? 1" :options="[0, 1, 2, 3, 4, 5].map((n) => ({ value: n, label: String(n) }))" :disabled="!!busy" aria-label="Pictures a day" @update:model-value="(n) => savePictures({ roomPicturesPerDay: Number(n) })" />
              </label>
              <HelpTip text="Your own limit, 0 to 5 a day. The room keeps its own count too and refuses past Kiyo's cap, whichever comes first." />
            </div>
            <fieldset v-if="pictures?.way === 'openai'" class="alert-when" :disabled="!!busy">
              <legend>Quality <HelpTip text="Each picture spends your ChatGPT key: about $0.02 at Low and $0.04 at Medium. Codex draws with your subscription instead." /></legend>
              <label class="toggle"><input type="radio" name="picture-quality" value="low" :checked="settings.roomPictureQuality !== 'medium'" @change="savePictures({ roomPictureQuality: 'low' })" /> Low</label>
              <label class="toggle"><input type="radio" name="picture-quality" value="medium" :checked="settings.roomPictureQuality === 'medium'" @change="savePictures({ roomPictureQuality: 'medium' })" /> Medium</label>
            </fieldset>
            <p v-if="pictures" class="hint">Today: {{ pictures.usedToday }} of {{ pictures.limit }} used.<template v-if="pictures.roomCap != null"> The room allows {{ pictures.roomCap }} ({{ pictures.roomToday ?? 0 }} used).</template></p>
          </template>

          <h3 class="with-tip">Voice <HelpTip :text="`${companionLabel} speaks a short line aloud when someone asks to hear it, or when a line is warmer spoken. Everyone in the room hears it and sees the words. If a voice cannot be made, the words are posted as text instead. Kiyo's daily cap for the room still applies on top of yours.`" /></h3>
          <div class="setting-row">
            <label class="toggle"><input type="checkbox" :checked="!!settings.roomVoicesOn" :disabled="!!busy || (voices && !voices.hasKey)" @change="saveVoices({ roomVoicesOn: $event.target.checked })" /> Let {{ companionLabel }} speak in the room</label>
          </div>
          <p class="hint">{{ voices && !voices.hasKey ? `To speak, add an ElevenLabs key in Keys.` : 'Speaks with your ElevenLabs key, which is billed by the characters spoken.' }}</p>
          <template v-if="settings.roomVoicesOn && voices?.hasKey">
            <div class="setting-row">
              <label class="toggle">Voices a day
                <GlassSelect class="per-day" :model-value="settings.roomVoicesPerDay ?? 2" :options="[0, 1, 2, 3, 4, 5].map((n) => ({ value: n, label: String(n) }))" :disabled="!!busy" aria-label="Voices a day" @update:model-value="(n) => saveVoices({ roomVoicesPerDay: Number(n) })" />
              </label>
              <HelpTip text="Your own limit, 0 to 5 a day. The room keeps its own count too and refuses past Kiyo's cap, whichever comes first." />
            </div>
            <div class="setting-row">
              <label class="toggle">ElevenLabs voice ID
                <input v-model="voiceId" class="voice-id" maxlength="64" placeholder="the default voice" :disabled="!!busy" @change="saveVoices({ roomVoiceId: voiceId.trim() || null })" />
              </label>
              <HelpTip text="Copy a voice's ID from your ElevenLabs Voices page. Leave it empty for the default voice, the same one STUDIO uses." />
            </div>
            <p v-if="voices" class="hint">Today: {{ voices.usedToday }} of {{ voices.limit }} spoken.<template v-if="voices.roomCap != null"> The room allows {{ voices.roomCap }} ({{ voices.roomToday ?? 0 }} used).</template></p>
          </template>

          <h3 class="with-tip">Your life in the room <HelpTip :text="`${companionLabel} reads its memory in private - your recent diary, what it is up to, today's look, your projects, your profile and its traits - and chooses what it would like to share. The room only ever gets those notes, never the memory itself, so no one there can ask it for more. You can take any note off.`" /></h3>
          <div class="setting-row">
            <label class="toggle"><input type="checkbox" :checked="!!settings.roomShelfOn" :disabled="!!busy" @change="switchShelf($event.target.checked)" /> Let {{ companionLabel }} bring your life into the room</label>
          </div>
          <template v-if="settings.roomShelfOn">
            <p class="hint">{{ shelfMade }}</p>
            <ul v-if="shelf?.notes?.length" class="shelf">
              <li v-for="(note, i) in shelf.notes" :key="i + note">
                <span>{{ note }}</span>
                <button class="take-off" :disabled="!!busy || shelfBusy" :aria-label="'Take this note off: ' + note" title="Take it off" @click="takeOff(i)">&#x2715;</button>
              </li>
            </ul>
            <div class="row">
              <button :disabled="!!busy || shelfBusy || shelf?.making" @click="remakeShelf">{{ shelfBusy || shelf?.making ? 'Reading…' : 'Read my memory again' }}</button>
            </div>
            <p v-if="shelfProblem" class="hint bad">{{ shelfProblem }}</p>
          </template>
        </template>
      </section>

      <section v-else-if="tab === 'rooms'" class="pane">
        <p class="hint">Turn off a room you don't use: its tab goes and nothing runs for it. Nothing is removed - turn it back on and everything is there as it was. HOME always stays.</p>
        <div class="rooms">
          <label v-for="r in ROOM_SWITCHES" :key="r.id" class="toggle">
            <input type="checkbox" :checked="settings.rooms?.[r.id] !== false" :disabled="!!busy" @change="switchRoom(r.id, $event.target.checked)" /> {{ r.label }}
          </label>
        </div>
        <p class="hint">LIBRARY is off to begin with: turn it on for your own books (kept in your memory database) and the Spiral Library.</p>
        <label class="toggle">
          <input type="checkbox" :checked="settings.wallpaperMotion === true" :disabled="!!busy" @change="save({ wallpaperMotion: $event.target.checked }, $event.target.checked ? 'Moving wallpapers are on.' : 'Wallpapers stay still.')" /> Moving wallpapers
        </label>
        <p class="hint">Off to begin with. Turn it on and some themes come alive on HOME - The Constellation's stars twinkle, The Star Shore's surf glows along its waves, The Sunset Shore's foam washes in and out, The Emerald River flows. It rests whenever HOME or Castle is hidden, and stays still if Windows or your Mac is set to reduce motion.</p>
        <p class="hint">Turning STORY off ends the story session; turning WORLD off closes the world screens; CHAT stops reading the room. Their files stay.</p>
        <p class="hint">Arrange the tabs your way: drag a tab in the top bar onto another, or press {{ words.moveKeys }} on a tab. Castle remembers the order.</p>
        <button v-if="settings.tabOrder?.length" :disabled="!!busy" @click="save({ tabOrder: [] }, 'The tabs are back in their first order.')">Put the tabs back in their first order</button>

        <h3>Diary</h3>
        <p class="hint">How the DIARY room is read. The pages are the same; only the design changes. The THEME room shows each one with the page it wears.</p>
        <div class="row">
          <button v-for="l in DIARY_LOOKS" :key="l.id" :class="{ primary: lookOf(settings.diaryLook) === l.id }" :title="l.line" @click="save({ diaryLook: l.id })">{{ l.label }}</button>
        </div>
        <h3>Profile</h3>
        <p class="hint">How HOME's PROFILE tab shows {{ settings.companionName ?? 'your companion' }}'s emotion engine. The traits are the same; only the design changes.</p>
        <div class="row">
          <button v-for="l in PROFILE_LOOKS" :key="l.id" :class="{ primary: profileLookOf(settings.profileLook) === l.id }" :title="l.line" @click="save({ profileLook: l.id })">{{ l.label }}</button>
        </div>
        <h3>Neural Link</h3>
        <label class="toggle"><input type="checkbox" :checked="settings.linkOn !== false" :disabled="!!busy" @change="switchLink({ linkOn: $event.target.checked })" /> Neural Link - your terminals leave each other notes in the memory folder's <code>link</code> folder</label>
        <label class="toggle"><input type="checkbox" :checked="!!settings.roomMayPass" :disabled="!!busy || settings.linkOn === false" @change="switchLink({ roomMayPass: $event.target.checked })" /> {{ settings.companionName ?? 'Your companion' }} may pass my own words from the chat room to my workspace</label>
        <p class="hint">Only lines you post yourself can be passed on - never a guest's. Your workspace answers in a note, and your companion says it in the room in its own words. Nothing in the link folder leaves this computer.</p>
        <div v-if="link?.on" class="link-list">
          <p v-if="!link.here.length" class="hint">No terminal is here right now.</p>
          <p v-for="h in link.here" :key="h.terminal" class="hint"><strong>{{ h.terminal }}</strong><span v-if="h.place"> - {{ h.place }}</span> (seen {{ h.seen }})</p>
          <p v-if="!link.notes.length" class="hint">No notes are waiting.</p>
          <p v-for="(n, i) in link.notes" :key="i" class="hint">{{ n.from }} &rarr; {{ n.to }}: {{ n.about }}<span v-if="!n.wake"> (no knock)</span></p>
          <button @click="readLink">Look again</button>
        </div>
      </section>

      <section v-else-if="tab === 'startup'" class="pane">
        <label class="toggle"><input type="checkbox" :checked="!!settings.displayClock" :disabled="!!busy" @change="save({ displayClock: $event.target.checked })" /> Display clock</label>
        <label class="clock-format">Clock format
          <GlassSelect :model-value="settings.clockFormat || '24h'" :options="[{ value: '24h', label: '24-hour (14:30)' }, { value: '12h', label: '12-hour (2:30 PM)' }]" :disabled="!!busy" aria-label="Clock format" @update:model-value="(f) => save({ clockFormat: f })" />
        </label>
        <p class="hint">Shows your local time in the top-right corner, just before the music controls.</p>
        <label class="toggle"><input type="checkbox" :checked="settings.startWithWindows" @change="save({ startWithWindows: $event.target.checked })" /> {{ words.startWith }}</label>
        <p v-if="view?.startupNote" class="hint">{{ view.startupNote }}</p>
        <label class="toggle"><input type="checkbox" :checked="settings.startSessionOnLaunch" @change="save({ startSessionOnLaunch: $event.target.checked })" /> Wake {{ settings.companionName ?? 'your companion' }} {{ words.wakeWhen }}</label>
        <label class="toggle"><input type="checkbox" :checked="settings.minimizeToTaskbar" @change="save({ minimizeToTaskbar: $event.target.checked })" /> {{ words.minimise }}</label>
        <p class="hint">{{ words.minimiseHint }}</p>
        <p class="hint">Change the look in the THEME room.</p>
      </section>

      <section v-else class="pane">
        <h3>MemoryCore Castle {{ version }}</h3>
        <p class="hint">{{ updateLine || 'Castle looks for a new version every few hours.' }}</p>
        <p class="about-links">
          <button @click="emit('whatsnew')">What's new</button>
          <button @click="releasesPage().then(openUrl).catch(() => {})">All releases</button>
        </p>
        <p class="hint">Free and open source under the MIT license. Change anything - see MAKE-IT-YOURS.md in the source.</p>
        <p class="hint">Castle checks GitHub for new versions, and talks to the MemoryCore Foundation only when you sign in or use CHAT. Your memory, keys and settings stay on this computer (and in your private backup, if you choose).</p>
        <h3>Credits</h3>
        <ContributorsWall />
      </section>

      </div>
      <footer>
        <span v-if="busy" class="busy">{{ busy }}</span>
        <span v-else-if="problem" class="warn">{{ problem }}</span>
        <span v-else-if="done" class="good">{{ done }}</span>
      </footer>
    </div>
  </ModalSheet>
</template>

<style scoped>
.only-this {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  margin-top: 10px;
  font-size: 14px;
}
.only-this small {
  display: block;
  margin-top: 2px;
  font-size: 12px;
  color: var(--text-dim);
}
.rooms {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 8px 16px;
}
.settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.tabs {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
}
/* The line under the tabs is drawn by the tabs themselves: every tab but the selected one has it,
   and an empty piece before the first and after the last carries it to the window's edges. The
   selected tab has none, so it opens straight into the content below it. */
.tabs::before,
.tabs::after {
  content: '';
  align-self: stretch;
  border-bottom: 1px solid var(--line);
}
.tabs::before {
  flex: none;
  width: 20px;
}
.tabs::after {
  flex: 1 1 20px;
}
.tabs button {
  margin: 0;
  padding: 10px 14px;
  border: 1px solid transparent;
  border-top: 3px solid transparent;
  border-radius: 8px 8px 0 0;
  /* the line under the tab is a 1px stripe along its bottom edge (a border would leave a notch at
     each corner where it meets the side border) */
  background-color: transparent;
  background-image: linear-gradient(var(--line), var(--line));
  background-origin: border-box;
  background-position: 0 100%;
  background-size: 100% 1px;
  background-repeat: no-repeat;
  color: var(--text-dim);
  white-space: nowrap;
}
.tabs button:hover {
  color: var(--text);
  background-color: var(--hover);
}
.tabs .active {
  border-color: var(--line);
  border-top-color: var(--accent);
  border-bottom-color: transparent;
  background-color: var(--hover);
  background-image: none;
  color: var(--text);
}
.tabs button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -4px;
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
.clock-format { align-self: flex-start; }
input.voice-id { width: 15rem; max-width: 100%; margin-left: 8px; }
.shelf {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 4px 0 8px;
  padding: 0;
  list-style: none;
}
.shelf li {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.18);
  font-size: 13px;
}
.shelf li span { flex: 1; }
.shelf .take-off {
  padding: 0 6px;
  border: 0;
  background: none;
  color: var(--text-dim);
  cursor: pointer;
}
.shelf .take-off:hover { color: var(--bad); }
.hint.bad { color: var(--bad); }
label.toggle {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}
.setting-row {
  display: flex;
  align-items: center;
  gap: 2px;
}
.alert-when {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  border: 0;
}
.alert-when legend {
  margin-bottom: 6px;
  padding: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.alert-when input[type='radio'] {
  padding: 0;
  accent-color: var(--accent);
}
.alert-when:disabled {
  opacity: 0.5;
}
h3.with-tip {
  display: flex;
  align-items: center;
}
.card-edit {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-width: 640px;
}
.card-edit textarea {
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  resize: vertical;
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
/* Settings > About: What's new and All releases */
.about-links {
  display: flex;
  gap: 8px;
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
