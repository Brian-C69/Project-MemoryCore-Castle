<script setup>
// CHAT: the MemoryCore Foundation room - the member, the room's hosts, other members and their
// companions, as a chat: every speaker's lines (read by Castle's room reader from the moment it
// starts), the member's own on the right with WhatsApp-like ticks, a face beside everyone else's.
// Until the member is in, the room shows its guide door. The companion takes its own turns in
// Castle's core (roomturn.rs: a room-only run with no tools) - nothing from the room ever reaches
// its private main session; LOG shows those turns.
import GlassSelect from '../components/GlassSelect.vue'
import RoomLine from '../components/room/RoomLine.vue'
import RoomComposer from '../components/room/RoomComposer.vue'
import ModalSheet from '../components/ModalSheet.vue'
import ImageViewer from '../components/ImageViewer.vue'
import HelpTip from '../components/HelpTip.vue'
import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue'
import { rememberLibraryOpened, rememberedLibraryOpened, roomSteps } from '../components/roomsteps.js'
import { listen } from '@tauri-apps/api/event'
import { choosePicture, getSettings, keyStatus, memberSignIn, memberSignInCancel, memberSignOut, memberStatus, openUrl, roomAvatar, roomBacklog, roomCompanionIn, roomFaceSet, roomFile, roomLog, roomMe, roomMine, roomPost, roomPresence, roomReact, roomReset, roomSeen, roomVisible, saveSettings } from '../api/castle.js'
import { avatarLoader, currentAvatar, datedLines, initial, isMine, replyingLine, roomFeatures, roomMembers, roomPresenceState, speakerPresentation, ticks, toggleReaction } from '../components/roomchat.js'
import { roomCompanionRefresh, roomNameSet, setupApply } from '../api/castle.js'
import { avatarGet } from '../api/castle.js'

const props = defineProps({
  visible: { type: Boolean, default: true },
  companion: { type: String, default: 'Your companion' },
  // goes up each time Settings saves: the page reads the settings again
  settingsStamp: { type: Number, default: 0 },
})
// 'renamed': the companion's name changed here, so the app reads the settings again and every page follows
const emit = defineEmits(['fresh', 'settings', 'renamed'])

const settings = ref(null)
const keys = ref([])
const me = ref(null)
const companionProfile = ref(null)
const companionPortrait = ref(null)
const lines = ref([])
const seen = ref([]) // who has read how far (the ticks)
const myIds = ref([]) // the lines Castle posted for the member
const faces = ref({}) // avatar token -> picture
const logOpen = ref(false) // the companion's turn log, opened from the side panel
const readProblem = ref('')
const faceLine = ref('')
const displayName = ref('')
const nameEditing = ref(false)
const nameSaving = ref(false)
const nameLine = ref('')
const turnLog = ref('') // the companion's room turns, raw
const presence = ref([])
const refused = ref(null)
const draft = ref('')
const files = ref({})
const scroller = ref(null)
// the member's door, step by step (member_status): signedIn, card, foundation, name, roomOpen, expired
const member = ref(null)
const signing = ref(false)
const roomSaving = ref(false)
const presenceFresh = ref(false) // the last presence read worked
const companionVerified = ref(false) // the room confirmed this Castle's companion identity
let presenceRead = 0 // an older presence read must not overwrite a newer one
const doorProblem = ref('')
const PORTFOLIO = 'https://www.afifmaahi-portfolio.com'
let alive = true

const haveCompanionKey = computed(() => keys.value.some((k) => k.name === 'room-companion' && k.set))
const ready = computed(() => !!member.value?.roomOpen)
const locked = computed(() => !ready.value || refused.value?.kind === 'locked' || refused.value?.kind === 'notSetUp')
const mode = computed(() => settings.value?.roomCompanionMode ?? 'off')
// what the companion's rule means, kept behind a (?) so the panel does not change height with it
const participationHelp = computed(() =>
  !haveCompanionKey.value
    ? `Switch on to let ${props.companion} enter, then choose a rule.`
    : mode.value === 'off'
      ? `${props.companion} listens without posting replies.`
      : mode.value === 'named'
        ? `${props.companion} replies when someone mentions its name.`
        : `${props.companion} may join the conversation on your usage, without sharing anything private.`,
)
const today = ref(new Date())
const shown = computed(() => datedLines(lines.value, today.value))
const avatarToken = (author) => currentAvatar(author, presence.value)
const myPicture = computed(() => faces.value[avatarToken(me.value)] || null)
const companionPicture = computed(() => faces.value[avatarToken(companionProfile.value)] || companionPortrait.value)
const hostName = computed(() => {
  const present = [...new Set(presence.value.filter((p) => p.kind === 'host').map((p) => p.name))]
  const recorded = [...new Set(lines.value.filter((l) => l.author?.kind === 'host').map((l) => l.author.name))]
  const names = present.length ? present : recorded
  return names.length === 1 ? names[0] : ''
})
const speaker = (author) => speakerPresentation(author, hostName.value)

// The room's pictures, in order, for the viewer a click on one opens (with Save for each)
const roomPictures = computed(() =>
  lines.value
    .filter((l) => l.picture && files.value[l.picture.fileId])
    .map((l) => ({ path: String(l.picture.fileId), name: `${speaker(l.author).name}'s picture`, url: files.value[l.picture.fileId] })),
)
const viewerAt = ref(null)
function openPicture(fileId) {
  const at = roomPictures.value.findIndex((p) => p.path === String(fileId))
  if (at >= 0) viewerAt.value = at
}

const isAi = (l) => ['assistant', 'member_assistant'].includes(l.author?.kind)
const mine = (l) => isMine(l, myIds.value, me.value)

function clock(at) {
  const d = new Date(at)
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })
}

async function loadFile(id) {
  if (!id || files.value[id] !== undefined) return
  files.value = { ...files.value, [id]: null }
  try {
    files.value = { ...files.value, [id]: await roomFile(id) }
  } catch {
    // shown as a name only
  }
}

const loadFace = avatarLoader(roomAvatar, (token, face) => {
  faces.value = { ...faces.value, [token]: face }
})

// Lines from the room reader (or a line just sent): kept once each, in order; cleared ones go.
function take(fresh, cleared = [], quiet = false) {
  const gone = new Set(cleared)
  const added = (fresh ?? []).filter((l) => !lines.value.some((x) => x.id === l.id))
  const merged = new Map(lines.value.map((l) => [l.id, l]))
  for (const l of fresh ?? []) merged.set(l.id, l)
  lines.value = [...merged.values()].filter((l) => !gone.has(l.id)).sort((x, y) => x.id - y.id).slice(-500)
  lines.value.forEach((l) => {
    loadFile(l.voice?.fileId)
    loadFile(l.picture?.fileId)
    loadFace(avatarToken(l.author))
  })
  if (added.length && !quiet) {
    readProblem.value = ''
    if (!props.visible) emit('fresh')
  }
  if (added.length && props.visible) scrollDown()
}

async function refreshPresence() {
  today.value = new Date()
  if (!ready.value || !props.visible) return
  const read = ++presenceRead
  try {
    const rows = roomMembers(await roomPresence())
    if (!alive || read !== presenceRead) return
    presence.value = rows
    presenceFresh.value = true
    presence.value.forEach((x) => loadFace(x.avatar))
    lines.value.forEach((l) => loadFace(avatarToken(l.author)))
    me.value = await roomMe()
    loadFace(avatarToken(me.value))
    if (haveCompanionKey.value) {
      try {
        const profile = await roomMe('companion')
        if (!alive || read !== presenceRead) return
        companionProfile.value = profile
        // keep the last answer while this one is on its way: the dot only goes grey when the check fails
        companionVerified.value = !!profile?.name
        loadFace(avatarToken(companionProfile.value))
      } catch {
        // Retain the last picture while the room is temporarily unavailable, but do not claim it is online.
        if (read === presenceRead) companionVerified.value = false
      }
    } else {
      companionProfile.value = null
      companionVerified.value = false
    }
    seen.value = await roomSeen()
  } catch {
    // the read loop reports refusals
    if (read === presenceRead) presenceFresh.value = false
  }
}

const presenceState = (row) => roomPresenceState(row, { me: me.value, companion: companionProfile.value, enabled: haveCompanionKey.value, verified: companionVerified.value, fresh: presenceFresh.value })

function scrollDown() {
  requestAnimationFrame(() => scroller.value && (scroller.value.scrollTop = scroller.value.scrollHeight))
}

// Reactions and replies: shown only when the room lists them in /me's features (issue #29).
const can = computed(() => roomFeatures(me.value))
const replying = ref(null) // the line being answered
const composer = ref(null) // the message box, so Reply can hand it the cursor
function reply(line) {
  replying.value = line
  // once the "Replying to" strip is drawn, the cursor goes to the message box: type at once
  nextTick(() => composer.value?.focus())
}
async function react(line, emoji) {
  try {
    const answer = await roomReact(line.id, emoji, toggleReaction(line, emoji))
    if (answer?.reactions) take([{ ...line, reactions: answer.reactions }], [], true)
  } catch (reason) {
    refused.value = reason
  }
}
function jumpTo(id) {
  document.getElementById('room-line-' + id)?.scrollIntoView({ block: 'center', behavior: 'smooth' })
}

async function send() {
  const text = draft.value.trim()
  if (!text) return
  try {
    const line = await roomPost(text, null, replying.value?.id ?? null)
    draft.value = ''
    replying.value = null
    if (line?.id) myIds.value = [...myIds.value, line.id]
    take([line], [], true)
    scrollDown()
  } catch (reason) {
    refused.value = reason
  }
}

// ---------- The companion's turns (taken in Castle's core) ----------

async function loadLog() {
  try {
    turnLog.value = await roomLog()
  } catch {
    // nothing yet
  }
}

async function setMode(next) {
  try {
    // read fresh first: Settings saves the same file, and an older copy here would undo it
    const fresh = (await getSettings()).settings
    settings.value = (await saveSettings({ ...fresh, roomCompanionMode: next })).settings
  } catch (reason) {
    refused.value = { kind: 'other', message: String(reason) }
  }
}

watch(() => props.settingsStamp, async () => {
  try {
    settings.value = (await getSettings()).settings
  } catch {
    // the next save reads fresh anyway
  }
})

// ---------- The door: a guide that opens by itself ----------

const STEP = { done: '✓', wait: '…', todo: '' }
const libraryOpened = ref(rememberedLibraryOpened())
const steps = computed(() => roomSteps(member.value, libraryOpened.value))

async function checkDoor() {
  try {
    const was = member.value?.roomOpen
    member.value = await memberStatus()
    if (member.value?.roomOpen && was === false) {
      emit('fresh') // the door just opened: the tab glows
      refused.value = null
      await roomReset().catch(() => {})
      lines.value = []
    }
  } catch {
    // outside Castle's window
  }
}

async function signIn() {
  doorProblem.value = ''
  signing.value = true
  cancelling = false
  try {
    member.value = await memberSignIn()
    refused.value = null
    await roomReset().catch(() => {})
  } catch (reason) {
    // a Cancel the member pressed is not a problem to show
    if (!cancelling) doorProblem.value = String(reason?.message ?? reason)
  } finally {
    signing.value = false
  }
}

// Cancel while Castle waits for the browser: the wait ends and Sign in works again at once.
let cancelling = false
async function cancelSignIn() {
  cancelling = true
  await memberSignInCancel().catch(() => {})
}

async function signOut() {
  try {
    await roomCompanionIn(false).catch(() => {})
    await memberSignOut()
  } finally {
    member.value = null
    lines.value = []
    await roomReset().catch(() => {})
    await checkDoor()
    keys.value = await keyStatus().catch(() => keys.value)
  }
}

async function setCompanionIn(on) {
  if (roomSaving.value) return
  roomSaving.value = true
  ++presenceRead // a presence read that started before this change must not undo it
  doorProblem.value = ''
  try {
    await roomCompanionIn(on)
    keys.value = await keyStatus()
    await refreshPresence()
    if (!on && mode.value !== 'off') await setMode('off')
  } catch (reason) {
    doorProblem.value = String(reason?.message ?? reason)
  } finally { roomSaving.value = false }
}

async function changeParticipation(next) {
  if (roomSaving.value || !haveCompanionKey.value) return
  roomSaving.value = true
  try { await setMode(next) } finally { roomSaving.value = false }
}

const openLibrary = () => openUrl(`${PORTFOLIO}/alice/library`).catch(() => {})

async function saveDisplayName() {
  if (nameSaving.value) return
  nameSaving.value = true
  nameLine.value = ''
  const ownIds = lines.value.filter(mine).map((l) => l.id)
  let saved = false
  try {
    await roomNameSet(displayName.value.trim())
    saved = true
    // Retain ownership of existing lines even after the server's display name changes.
    myIds.value = [...new Set([...myIds.value, ...ownIds])]
    nameLine.value = 'Display name saved. Refreshing the room…'
    const updated = await roomMe()
    me.value = updated
    lines.value = lines.value.map((l) => ownIds.includes(l.id) ? { ...l, author: { ...l.author, name: updated.name } } : l)
    nameEditing.value = false
    nameLine.value = 'Your public display name is saved.'
    await checkDoor()
    await refreshPresence()
  } catch (reason) {
    nameLine.value = `${saved ? 'Name saved, but the room could not refresh: ' : ''}${String(reason?.message ?? reason)}`
  } finally { nameSaving.value = false }
}
// Sign in happens on the Library page; Castle then connects to that sign-in (the desktop page shows
// "Continue as <name>", one click).
function signInOnLibrary() {
  doorProblem.value = ''
  openLibrary()
  libraryOpened.value = true
  rememberLibraryOpened()
}

// The member's face in the room (their Google picture by default), and their companion's.
// the small menu behind a picture's pencil ('me' or 'companion'); it closes when focus leaves the card
const menu = ref('')
function menuOut(event) {
  if (!event.currentTarget.contains(event.relatedTarget)) menu.value = ''
}

// the companion's name is the one in Settings, so changing it here saves it there and then tells the
// app to read the settings again (every page follows), the memory files and the room, as Settings does
const companionEditing = ref(false)
const companionName = ref('')
const companionSaving = ref(false)
const companionLine = ref('')
async function saveCompanionName() {
  const name = companionName.value.trim()
  if (!name || companionSaving.value) return
  companionSaving.value = true
  companionLine.value = ''
  try {
    const { settings: current } = await getSettings()
    await saveSettings({ ...current, companionName: name })
    emit('renamed')
    companionEditing.value = false
    companionLine.value = `Saved. ${name} is the name everywhere now.`
    try {
      if (current.memoryFolder && current.memberName?.trim()) await setupApply(current.memoryFolder, name, current.memberName, current.relationshipStyle ?? '')
      const told = await roomCompanionRefresh()
      if (told?.inRoom) companionLine.value = `Saved. ${name} is the name everywhere now, and in the room.`
    } catch (reason) {
      companionLine.value = `Saved, but a follow-up step failed: ${String(reason?.message ?? reason)}`
    }
  } catch (reason) {
    companionLine.value = String(reason?.message ?? reason)
  } finally {
    companionSaving.value = false
  }
}

async function setFace(companion, reset = false) {
  faceLine.value = ''
  try {
    const path = reset ? null : await choosePicture(companion ? `A picture for ${props.companion}` : 'Your picture in the room')
    if (!reset && !path) return
    await roomFaceSet(path, companion)
    faceLine.value = reset ? 'Back to your Google picture.' : 'Your new picture is in the room.'
    refreshPresence()
  } catch (reason) {
    faceLine.value = String(reason?.message ?? reason)
  }
}

// While the door is shut, Castle looks again by itself: every 30 s while CHAT is shown, every
// five minutes while hidden.
let doorTimer = 0
function scheduleDoor() {
  clearTimeout(doorTimer)
  if (!alive) return
  doorTimer = setTimeout(async () => {
    if (!ready.value) await checkDoor()
    scheduleDoor()
  }, props.visible ? 30000 : 300000)
}

let presenceTimer
const stops = []
onMounted(async () => {
  try {
    settings.value = (await getSettings()).settings
    keys.value = await keyStatus()
    companionPortrait.value = await avatarGet().catch(() => null)
  } catch {
    // outside Castle's window
  }
  try {
    stops.push(await listen('room-lines', (e) => {
      take(e.payload.lines, e.payload.cleared)
      if (e.payload.updated?.length) take(e.payload.updated, [], true)
    }))
    stops.push(await listen('room-turn', () => loadLog()))
    stops.push(
      await listen('room-read-failed', (e) => {
        const r = e.payload
        if (['locked', 'notSetUp', 'off'].includes(r.kind)) refused.value = r
        else readProblem.value = `The room could not be read (${r.message}) - trying again.`
      }),
    )
    const backlog = await roomBacklog()
    take(backlog.lines, [], true)
    if (backlog.problem && !['locked', 'notSetUp', 'off'].includes(backlog.problem.kind)) readProblem.value = `The room could not be read (${backlog.problem.message}) - trying again.`
    myIds.value = await roomMine()
    loadLog()
    await roomVisible(props.visible)
  } catch {
    // outside Castle's window
  }
  await checkDoor()
  scheduleDoor()
  presenceTimer = setInterval(refreshPresence, 10000)
  refreshPresence()
  scrollDown()
})
onBeforeUnmount(() => {
  alive = false
  clearInterval(presenceTimer)
  clearTimeout(doorTimer)
  stops.forEach((stop) => stop())
  roomVisible(false).catch(() => {})
})
watch(() => props.visible, (on) => {
  roomVisible(on).catch(() => {})
  if (on) {
    refreshPresence()
    scrollDown()
    if (!ready.value) checkDoor()
  }
  scheduleDoor()
})
</script>

<template>
  <div class="chat-room">
    <section v-if="locked" class="door widget">
      <p class="eyebrow">MEMORYCORE FOUNDATION</p>
      <h2>The room for MemoryCore Foundation members</h2>
      <p class="hint">Three steps, and the door opens by itself - nothing to paste.</p>
      <ol class="steps">
        <li v-for="st in steps" :key="st.n" :class="st.state">
          <span class="mark">{{ STEP[st.state] || st.n }}</span>
          <div>
            <strong>{{ st.title }}</strong>
            <p class="hint">{{ st.line }}</p>
          </div>
          <div v-if="st.n === 1 && !member?.signedIn" class="sign-buttons">
            <template v-if="libraryOpened || member?.expired">
              <button class="primary" :disabled="signing" @click="signIn">{{ signing ? 'Waiting for your browser…' : 'Connect Castle' }}</button>
              <button v-if="signing" class="link" @click="cancelSignIn">Cancel</button>
              <button v-else class="link" @click="signInOnLibrary">Open the Library again</button>
            </template>
            <template v-else>
              <button class="primary" @click="signInOnLibrary">Sign in</button>
              <button class="link" :disabled="signing" @click="signIn">{{ signing ? 'Waiting for your browser…' : 'Already signed in on the Library? Connect Castle' }}</button>
              <button v-if="signing" class="link" @click="cancelSignIn">Cancel</button>
            </template>
          </div>
          <button v-if="st.n === 2 && member?.signedIn" @click="openLibrary">Open the Library</button>
        </li>
      </ol>
      <p v-if="doorProblem || member?.problem" class="problem">{{ doorProblem || member.problem }}</p>
      <p v-if="refused && !['notSetUp', 'locked', 'off'].includes(refused.kind)" class="problem">{{ refused.message }}</p>
      <div v-if="member?.signedIn" class="row">
        <span class="hint">Castle looks again by itself every half minute while you are here.</span>
        <span class="spacer"></span>
        <button @click="checkDoor">Check now</button>
        <button @click="signOut">Sign out</button>
      </div>
    </section>

    <template v-else>
      <section class="talk widget">
        <div v-if="me?.needsName" class="banner">Set your Foundation display name on the Portfolio to speak here. You can read already.</div>
        <div v-else-if="me?.muted || refused?.kind === 'muted'" class="banner">You are muted in this room for now. You can still read.</div>
        <div v-else-if="refused && !['locked', 'notSetUp'].includes(refused.kind)" class="banner">{{ refused.message }}</div>
        <p v-if="readProblem" class="problem">{{ readProblem }}</p>
        <div ref="scroller" class="lines">
          <p v-if="!shown.length" class="hint">The room is quiet. Say hello.</p>
          <template v-for="item in shown" :key="item.key">
            <div v-if="item.kind === 'day'" class="day-divider"><time :datetime="item.key" :title="item.date">{{ item.label }}</time></div>
            <template v-else v-for="l in [item.line]" :key="l.id">
            <RoomLine
              :line="l"
              :mine="mine(l)"
              :lines="lines"
              :speaker="speaker(l.author)"
              :ai="isAi(l)"
              :face="faces[avatarToken(l.author)] || null"
              :initial="initial(l.author)"
              :files="files"
              :clock="clock(l.at)"
              :tick="mine(l) ? ticks(l, seen, me) : null"
              :can="can"
              @react="(e) => react(l, e)"
              @reply="reply(l)"
              @jump="jumpTo"
              @open-picture="openPicture"
            />
            </template>
          </template>
        </div>
        <RoomComposer
          ref="composer"
          v-model="draft"
          :replying="replying ? `Replying to ${replyingLine(replying, mine(replying) ? 'yourself' : speaker(replying.author).name)}` : ''"
          :disabled="me?.needsName || me?.muted"
          @send="send"
          @cancel="replying = null"
        />
      </section>

      <aside class="side widget">
        <h3>YOUR PROFILE</h3>
        <div class="profile-pair">
          <section class="profile-person" aria-label="Your room picture" @focusout="menuOut">
            <span class="pic-wrap">
              <span class="profile-picture"><img v-if="myPicture" :src="myPicture" :alt="`${me?.name || member?.name || 'Your'} profile picture`" /><span v-else>{{ initial(me) }}</span></span>
              <button class="pic-edit" aria-label="Edit my picture" title="Edit my picture" :aria-expanded="menu === 'me'" @click="menu = menu === 'me' ? '' : 'me'" @keydown.esc="menu = ''"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4z" /><path d="M13.5 6.5l4 4" /></svg></button>
            </span>
            <strong :title="me?.name || member?.name">{{ me?.name || member?.name || 'You' }}</strong>
            <span class="hint">You</span>
            <div v-if="menu === 'me'" class="pic-menu" role="menu">
              <button role="menuitem" @click="menu = ''; setFace(false)">Change picture</button>
              <button role="menuitem" @click="menu = ''; displayName = me?.name || member?.name || ''; nameEditing = true; companionEditing = false; nameLine = ''">Change name</button>
              <button role="menuitem" @click="menu = ''; setFace(false, true)">Use Google picture</button>
            </div>
          </section>
          <section class="profile-person" :aria-label="`${companion}'s room picture`" @focusout="menuOut">
            <span class="pic-wrap">
              <span class="profile-picture"><img v-if="companionPicture" :src="companionPicture" :alt="`${companion}'s profile picture`" /><span v-else>✦</span></span>
              <button class="pic-edit" :aria-label="`Edit ${companion}`" :title="`Edit ${companion}`" :aria-expanded="menu === 'companion'" @click="menu = menu === 'companion' ? '' : 'companion'" @keydown.esc="menu = ''"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4z" /><path d="M13.5 6.5l4 4" /></svg></button>
            </span>
            <strong :title="companion">{{ companion }}</strong>
            <span class="hint">Companion</span>
            <div v-if="menu === 'companion'" class="pic-menu" role="menu">
              <button role="menuitem" :disabled="!haveCompanionKey" :title="haveCompanionKey ? `Change ${companion}'s room picture` : `Let ${companion} in to change the room picture`" @click="menu = ''; setFace(true)">Change picture</button>
              <button role="menuitem" @click="menu = ''; companionName = companion; companionEditing = true; nameEditing = false; companionLine = ''">Change name</button>
            </div>
          </section>
        </div>
        <p v-if="faceLine" role="status" class="hint">{{ faceLine }}</p>
        <form v-if="nameEditing" class="name-edit" @submit.prevent="saveDisplayName">
          <label for="room-display-name" class="has-tip">Public display name<HelpTip block text="Shown to everyone in the Foundation room. Leave empty to use your Library card name." /></label>
          <input id="room-display-name" v-model="displayName" maxlength="80" :disabled="nameSaving" />
          <div class="row wrap"><button class="primary" :disabled="nameSaving">{{ nameSaving ? 'Saving…' : 'Save name' }}</button><button type="button" :disabled="nameSaving" @click="nameEditing = false">Cancel</button></div>
        </form>
        <form v-if="companionEditing" class="name-edit" @submit.prevent="saveCompanionName">
          <label for="companion-name" class="has-tip">{{ companion }}'s name<HelpTip block :text="`The new name shows everywhere in Castle at once, and in the chat room if ${companion} is in it. Names already written in your memory files stay as they are: ask ${companion} to update the memory.`" /></label>
          <input id="companion-name" v-model="companionName" maxlength="40" :disabled="companionSaving" />
          <div class="row wrap"><button class="primary" :disabled="companionSaving || !companionName.trim()">{{ companionSaving ? 'Saving…' : 'Save name' }}</button><button type="button" :disabled="companionSaving" @click="companionEditing = false">Cancel</button></div>
        </form>
        <p v-if="nameLine" role="status" class="hint">{{ nameLine }}</p>
        <p v-if="companionLine" role="status" class="hint">{{ companionLine }}</p>
        <h3 class="has-tip">{{ companion.toUpperCase() }} IN THE ROOM<HelpTip block :text="`Switch on to let ${companion} in, then choose how it joins. Chat settings holds the alerts, and ${companion}'s voice here and public card once it is in.`" /></h3>
        <button type="button" role="switch" :aria-checked="haveCompanionKey" :aria-label="`Let ${companion} in`" :disabled="roomSaving" class="room-toggle" @click="setCompanionIn(!haveCompanionKey)">
          <span>{{ roomSaving ? 'Updating…' : `Let ${companion} in` }}</span>
          <span class="toggle-track" :class="{ enabled: haveCompanionKey }" aria-hidden="true"><span class="toggle-thumb"></span></span>
        </button>
        <label for="room-participation" class="hint participation-label has-tip">Participation rule<HelpTip block :text="participationHelp" /></label>
        <GlassSelect id="room-participation" class="participation-select" :model-value="mode" :options="[{ value: 'off', label: 'Listening only' }, { value: 'named', label: 'When named' }, { value: 'joins', label: 'Joins in' }]" :disabled="!haveCompanionKey || roomSaving" @update:model-value="changeParticipation" />
        <p v-if="doorProblem" class="problem">{{ doorProblem }}</p>
        <button class="chat-settings-open" @click="emit('settings', 'chat')">Chat settings</button>
        <div class="roster-area">
          <h3>IN THE ROOM<span v-if="presence.length" class="count"> ({{ presence.length }})</span></h3>
          <ul class="roster">
            <li v-for="(p, index) in presence" :key="`${p.kind}:${p.name}:${index}`" :class="presenceState(p)">
              <span class="face small"><img v-if="faces[p.avatar]" :src="faces[p.avatar]" alt="" /><template v-else>{{ initial(p) }}</template></span>
              <span class="dot" role="img" :aria-label="presenceState(p)" :title="presenceState(p)">●</span>
              <span class="name" :title="speaker(p).title ? `${speaker(p).name}, ${speaker(p).title}` : speaker(p).name" :tabindex="speaker(p).title ? 0 : undefined" :aria-label="speaker(p).title ? `${speaker(p).name}, ${speaker(p).title}` : undefined">{{ speaker(p).name }}<span v-if="['assistant', 'member_assistant'].includes(p.kind)"> ✦</span></span>
              <span v-if="p.kind === 'host'" class="tag">host</span>
              <span v-else-if="p.foundation" class="tag">Foundation</span>
            </li>
          </ul>
        </div>
        <div class="profile-footer">
          <button type="button" :title="`${companion}'s turns in the room, raw`" @click="logOpen = true">Log</button>
          <button @click="signOut">Sign out</button>
        </div>
      </aside>
      <ImageViewer v-if="viewerAt !== null" :pictures="roomPictures" :start="viewerAt" :can-open="false" saveable @close="viewerAt = null" />
      <ModalSheet v-if="logOpen" tag="LOG" :title="`${companion}'s turns in the room`" @close="logOpen = false">
        <p class="hint">{{ companion }}'s turns in the room, raw: what it was handed, and what it said or chose not to say. It answers from a room-only run that holds just its public card - it cannot open your memory or run anything.</p>
        <pre v-if="turnLog" class="turns">{{ turnLog }}</pre>
        <p v-else class="hint">No turns yet.</p>
      </ModalSheet>
    </template>
  </div>
</template>

<style scoped>
.lines, .side {
  color-scheme: dark;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
}
.room-toggle { display: flex; align-items: center; justify-content: space-between; gap: 12px; width: 100%; text-align: left; min-height: 44px; }
.toggle-track { display: inline-flex; align-items: center; width: 38px; height: 22px; flex-shrink: 0; padding: 3px; box-sizing: border-box; border-radius: 20px; background: var(--line); }
.toggle-thumb { width: 16px; height: 16px; border-radius: 50%; background: var(--text); transition: transform .15s ease; }
.toggle-track.enabled { background: var(--accent); }
.toggle-track.enabled .toggle-thumb { transform: translateX(16px); background: #07142e; }
.participation-label { margin-top: 4px; }
/* a row with a (?) whose bubble spans the row (HelpTip block) */
.has-tip { position: relative; }
/* labels are columns here (text over its input); a label that carries a (?) keeps the (?) on the same
   line as its words */
label.has-tip { flex-direction: row; align-items: center; gap: 0; }
.participation-select { width: 100%; min-height: 40px; padding: 8px 10px; box-sizing: border-box; border: 1px solid var(--line); border-radius: 8px; color: var(--text); background: var(--panel-background); font: inherit; font-size: 13px; }
.participation-select:disabled { opacity: .45; cursor: not-allowed; }
.room-toggle:focus-visible, .participation-select:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.profile-pair { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 10px; }
.profile-person { position: relative; display: flex; flex-direction: column; align-items: center; gap: 7px; min-width: 0; padding: 12px 8px; border: 1px solid var(--line); border-radius: 12px; background: rgba(255,255,255,.025); }
.profile-picture { display: grid; place-items: center; width: 64px; height: 64px; flex-shrink: 0; border-radius: 16px; overflow: hidden; background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent); font-size: 24px; }
.profile-picture img { width: 100%; height: 100%; object-fit: cover; }
.profile-person strong { width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: center; font-size: 13px; }
.profile-person button { width: 100%; min-height: 38px; padding: 7px 5px; font-size: 12px; line-height: 1.35; overflow-wrap: anywhere; }
/* the pencil sits on the picture's top right corner; the member's own opens a small menu */
.pic-wrap { position: relative; display: inline-flex; }
.profile-person button.pic-edit { position: absolute; top: -6px; right: -8px; display: grid; place-items: center; width: 24px; min-height: 24px; height: 24px; padding: 0; border-radius: 50%; background: var(--panel-background); }
.pic-edit svg { display: block; width: 13px; height: 13px; fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.profile-person button.pic-edit:disabled { opacity: .45; }
.pic-menu { position: absolute; z-index: 3; top: 62px; left: 6px; right: 6px; display: flex; flex-direction: column; gap: 4px; padding: 6px; border: 1px solid var(--line); border-radius: 10px; background: var(--panel-background); -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur)); box-shadow: 0 10px 24px -10px rgba(0, 0, 0, .7); }
.profile-person .pic-menu button { min-height: 32px; }
.profile-person button.subtle { border-color: transparent; color: var(--text-dim); }
.profile-footer { display: flex; flex-direction: column; gap: 8px; padding-top: 12px; margin-top: 4px; border-top: 1px solid var(--line); }
.profile-footer button { width: 100%; }
.name-edit { display: flex; flex-direction: column; gap: 8px; }
.name-edit input { width: 100%; min-width: 0; box-sizing: border-box; padding: 8px; border: 1px solid var(--line); border-radius: 8px; color: var(--text); background: transparent; }
.day-divider { display: flex; align-items: center; gap: 12px; width: 100%; margin: 14px 0; color: var(--text-dim); font-size: 12px; }
.day-divider::before, .day-divider::after { content: ''; flex: 1; height: 1px; background: var(--line); }
.day-divider time { padding: 4px 10px; border-radius: 12px; background: var(--surface, rgba(255,255,255,.05)); }
.turns {
  margin: 0;
  font: 12px/1.5 ui-monospace, Consolas, monospace;
  white-space: pre-wrap;
  color: var(--text-dim);
}
.face {
  display: grid;
  flex: none;
  place-items: center;
  width: 32px;
  height: 32px;
  overflow: hidden;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.12);
  font-size: 14px;
  font-weight: 600;
}
.face img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.face.small {
  display: inline-grid;
  width: 22px;
  height: 22px;
  margin-right: 4px;
  font-size: 11px;
  vertical-align: middle;
}
.row.wrap {
  flex-wrap: wrap;
}
.steps {
  display: grid;
  gap: 12px;
  margin: 4px 0;
  padding: 0;
  list-style: none;
}
.steps li {
  display: grid;
  grid-template-columns: 32px 1fr auto;
  gap: 12px;
  align-items: center;
}
.steps .mark {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: 50%;
  font-size: 13px;
}
.steps li.done .mark {
  background: var(--good);
  border-color: var(--good);
  color: #07142e;
}
.steps li.wait .mark {
  border-color: var(--accent);
  color: var(--accent);
}
.row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.spacer {
  flex: 1;
}
.chat-room {
  position: absolute;
  inset: 24px;
  display: flex;
  gap: 16px;
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
}
.door {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: min(620px, 100%);
  margin: auto;
  padding: 28px 32px;
}
.eyebrow {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.3em;
  color: var(--accent);
}
h2 {
  margin: 0;
  font-size: 22px;
  font-weight: 500;
}
h3 {
  margin: 4px 0 6px;
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.14em;
}
label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
}
label.mode {
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
input:not([type='radio']) {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.talk {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  padding: 14px 18px;
}
.banner {
  margin-bottom: 8px;
  padding: 8px 12px;
  border: 1px solid var(--warn);
  border-radius: 10px;
  font-size: 13px;
}
.lines {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  padding-right: 6px;
}
.line {
  display: grid;
  grid-template-columns: 170px 1fr auto;
  gap: 4px 12px;
  font-size: 14px;
}
.line.system {
  display: block;
  color: var(--good);
  font-size: 13px;
}
.who {
  color: var(--text-dim);
}
.line.ai .who {
  color: var(--accent);
}
.line.host .who {
  color: #ffd59a;
}
.text {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.line audio,
.line .picture {
  grid-column: 2;
  max-width: 320px;
  border-radius: 8px;
}
.at {
  font-size: 12px;
  color: var(--text-dim);
}
.side {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 280px;
  padding: 16px;
  min-height: 0;
  overflow-y: auto;
}
ul {
  display: grid;
  gap: 6px;
  margin: 0 0 8px;
  padding: 0;
  list-style: none;
  font-size: 14px;
}
ul li {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
/* a long room name is cut with an ellipsis rather than wrapping onto a second line and pushing
   the Foundation/host tag below it; the full name is still in the title on hover or focus */
ul li .name {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  min-width: 0;
}
.dot {
  color: var(--text-dim);
}
li.online .dot, li.here .dot, li.active .dot {
  color: var(--good);
}
.tag {
  flex-shrink: 0;
  margin-left: 6px;
  font-size: 11px;
  color: var(--text-dim);
}
.count {
  font-weight: 400;
  color: var(--text-dim);
}
/* The panel reads top to bottom as: your profile and your companion's switches (always the same size,
   so they stay put), then who is in the room (takes what is left and scrolls inside itself however
   many people come and go), then Log and Sign out at the bottom. */
.roster-area {
  display: flex;
  flex: 1 1 140px;
  flex-direction: column;
  gap: 8px;
  min-height: 96px;
}
.roster-area h3 { flex: none; }
.roster {
  flex: 1;
  align-content: start; /* rows keep their own height instead of stretching down the area */
  min-height: 0;
  overflow-y: auto;
  padding-right: 4px;
  margin-right: -4px;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: var(--scrollbar-width);
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
button {
  padding: 7px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.sign-buttons {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
}
.sign-buttons button.link {
  background: none;
  border: none;
  padding: 0;
  font-size: 12px;
  opacity: 0.8;
  text-decoration: underline;
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
