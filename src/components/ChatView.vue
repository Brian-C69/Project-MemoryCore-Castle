<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { chatAttach, chatMedia, chatPoll, clipboardPaste } from '../api/castle.js'
import { localSend, mergeEntries, resetEntries } from './ticks.js'
import { createVoiceQueue } from './voicequeue.js'
import { readChoices } from './choices.js'
import ImageViewer from './ImageViewer.vue'
import MarkdownText from './MarkdownText.vue'
import PlanViewer from './PlanViewer.vue'
import { firstHeading } from './markdown.js'
import { worldSignals } from '../world3d/signals.js'

const props = defineProps({
  // Whether the session is running.
  running: { type: Boolean, default: false },
  // Whether the Chat tab is the one showing (answer buttons and focus). Reading goes on on any
  // tab while Castle is visible, so the companion's voice can play wherever the member is in Castle.
  visible: { type: Boolean, default: false },
  // Sends text into the same session, as if typed in the Log.
  write: { type: Function, required: true },
  // The last lines of the Log screen, for the answer buttons.
  readScreen: { type: Function, default: () => [] },
  // undefined for the main session, 'story' for the STORY room's
  room: { type: String, default: undefined },
  // the companion's name, for labels
  companion: { type: String, default: 'Your companion' },
  // 'claude' or 'codex': answer buttons and send-now are Claude Code's; Codex asks in the Log
  cli: { type: String, default: 'claude' },
})
const emit = defineEmits(['show-log'])

const POLL_MS = 1000
// The typing bubble gives up after this long without a new step (a long build keeps it).
const TYPING_FOR_MS = 10 * 60 * 1000

const entries = ref([])
const status = ref('asleep')
const draft = ref('')
const scroller = ref(null)
const box = ref(null)
const now = ref(Date.now())
// A choice menu read from the Log screen (a permission prompt or a question), or null.
const menu = ref(null)
// Codex is asking for an approval on the Log screen (its prompts are answered there).
const codexAsking = ref(false)
let timer
let menuTimer
let clockTimer

function readMenu() {
  if (!props.visible || !props.running) {
    menu.value = null
    return
  }
  if (props.cli === 'codex') {
    menu.value = null
    codexAsking.value = CODEX_ASK.test(props.readScreen(15).join('\n'))
    return
  }
  menu.value = readChoices(props.readScreen(40))
}

// Words Codex shows while it waits for an approval.
const CODEX_ASK = /allow command|approve|apply (these )?changes|yes, proceed|\(y\/n\)|\[y\/n\]/i

async function choose(number) {
  menu.value = null
  try {
    await props.write(String(number))
  } catch {
    // The menu comes back on the next read.
  }
}

// A conversation between two people: the companion's tool steps stay in Log, and while it works Chat shows
// only a typing bubble. Pictures made one after another sit side by side.
const items = computed(() => {
  const out = []
  const plansSeen = new Set()
  for (const entry of entries.value) {
    const last = out[out.length - 1]
    if (entry.role === 'work') continue
    if (entry.role === 'plan') {
      // The same plan asked, then approved, is one card.
      if (!plansSeen.has(entry.text)) out.push({ kind: 'plan', id: entry.id, entry, ...planFace(entry.text) })
      plansSeen.add(entry.text)
    } else if (entry.role === 'picture' && last?.kind === 'pictures') last.list.push(entry)
    else if (entry.role === 'picture') out.push({ kind: 'pictures', id: entry.id, list: [entry] })
    else if (entry.role === 'voice') out.push({ kind: 'voice', id: entry.id, entry })
    else out.push({ kind: 'message', id: entry.id, entry })
  }
  return out
})

// The companion is typing while its newest entry is a tool step: it is working toward its next words.
const typing = computed(() => {
  if (!props.running) return false
  const newest = entries.value[entries.value.length - 1]
  if (newest?.role !== 'work') return false
  const at = Date.parse(newest.at ?? '')
  return Number.isNaN(at) || now.value - at < TYPING_FOR_MS
})

// The WORLD on the other screens knows when the companion works and when it rests.
watch(typing, (on) => (worldSignals.busy = on), { immediate: true })
watch(() => props.running, (on) => (worldSignals.asleep = !on), { immediate: true })

// ---------- The companion's plans ----------

// A plan card shows only its title and how many phases; the reader holds the whole plan.
function planFace(text) {
  const title = firstHeading(text) || 'Plan'
  const phases = (text.match(/^#{2,3}\s+Phase\b/gim) || []).length
  return { title, phases }
}

const planOpen = ref(null)

// ---------- The companion's pictures and voice ----------

// path -> data URL, loaded once from the core (which serves only files shown as the companion's)
const media = reactive({})
const durations = reactive({})
const playing = ref(null)
const viewerStart = ref(null)
const player = new Audio()
// one voice at a time, never cut: a voice that lands while another plays waits its turn
const voices = createVoiceQueue()
const isPlaying = () => !!playing.value && !player.paused && !player.ended
player.addEventListener('ended', () => {
  playing.value = null
  const next = voices.ended()
  if (next) startVoice(next)
})

const fileName = (path) => path.split(/[\\/]/).pop()

// Every picture in this chat in order - the ones the companion made and the ones the member shared - for the viewer.
const allPictures = computed(() =>
  entries.value.flatMap((e) => {
    if (e.role === 'picture') return [{ path: e.path, name: e.text, url: media[e.path] }]
    if (e.role === 'member' && e.pictures?.length) return e.pictures.map((p) => ({ path: p, name: fileName(p), url: media[p] }))
    return []
  }),
)

// ---------- Pictures the member shares ----------

// Picture paths in the box as the member writes: dropped, pasted or typed.
const PICTURE_PATH = /[A-Za-z]:[\\/][^"'<>|*?\r\n]*?\.(?:png|jpe?g|webp)/gi
const draftPictures = computed(() => [...new Set(draft.value.match(PICTURE_PATH) ?? [])])
const draftViewer = ref(null)

function loadPath(path) {
  return loadMedia({ path, role: 'picture' })
}

// A path the member typed or pasted as text is the member's too: the core checks it is a picture, then it shows.
watch(draftPictures, async (paths) => {
  const fresh = paths.filter((p) => !media[p])
  if (!fresh.length) return
  try {
    await chatAttach(fresh)
  } catch {
    // Shown by name only.
  }
  fresh.forEach(loadPath)
})

const quoted = (path) => (/\s/.test(path) ? `"${path}"` : path)

/** Files dropped on Chat: their paths go into the box at the cursor; pictures show above it. */
async function addPaths(paths) {
  try {
    await chatAttach(paths)
  } catch {
    // Not previewed; the paths still go in.
  }
  const el = box.value
  const start = el?.selectionStart ?? draft.value.length
  const end = el?.selectionEnd ?? draft.value.length
  const before = draft.value.slice(0, start)
  const insert = (before && !/\s$/.test(before) ? ' ' : '') + paths.map(quoted).join(' ') + ' '
  draft.value = before + insert + draft.value.slice(end)
  await nextTick()
  grow()
  box.value?.focus()
}

function removeDraftPicture(path) {
  draft.value = draft.value.replace(`"${path}"`, '').replace(path, '').replace(/ {2,}/g, ' ')
}

function openSharedPicture(path) {
  const at = allPictures.value.findIndex((p) => p.path === path)
  viewerStart.value = at < 0 ? 0 : at
}

async function loadMedia(entry) {
  if (!entry.path || media[entry.path]) return media[entry.path]
  try {
    media[entry.path] = await chatMedia(entry.path, props.room)
    if (entry.role === 'voice') {
      const probe = new Audio(media[entry.path])
      probe.addEventListener('loadedmetadata', () => (durations[entry.path] = probe.duration))
    }
  } catch {
    media[entry.path] = null
  }
  return media[entry.path]
}

// the member's click: this voice plays now and the waiting line is cleared
function playVoice(entry) {
  return startVoice(voices.choose(entry))
}

async function startVoice(entry) {
  const url = await loadMedia(entry)
  if (!url) {
    const next = voices.ended()
    if (next) startVoice(next)
    return
  }
  player.pause()
  player.src = url
  player.currentTime = 0
  try {
    await player.play()
    playing.value = entry.path
  } catch {
    playing.value = null
  }
}

function toggleVoice(entry) {
  if (playing.value === entry.path) {
    player.pause()
    voices.stop()
    playing.value = null
  } else {
    playVoice(entry)
  }
}

function openViewer(entry) {
  const at = allPictures.value.findIndex((p) => p.path === entry.path)
  viewerStart.value = at < 0 ? 0 : at
}

function length(seconds) {
  if (!seconds || !Number.isFinite(seconds)) return ''
  const s = Math.round(seconds)
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
}

function nearBottom() {
  const el = scroller.value
  return !el || el.scrollHeight - el.scrollTop - el.clientHeight < 80
}

let lastPollAt = 0

async function poll() {
  if (document.visibilityState !== 'visible') return
  // A voice plays by itself only when it arrives while Castle was in view; one made while
  // Castle was hidden arrives in a catch-up read and waits for "play".
  const live = Date.now() - lastPollAt < POLL_MS * 3.5
  lastPollAt = Date.now()
  let answer
  try {
    answer = await chatPoll(props.room)
  } catch {
    return
  }
  status.value = answer.status
  answer.entries.filter((e) => e.path).forEach(loadMedia)
  answer.entries.filter((e) => e.pictures?.length).forEach((e) => e.pictures.forEach(loadPath))
  // the STORY stage reads the story's replies too (its fallback when no beat card is written)
  if (props.room === 'story' && !answer.reset) {
    answer.entries.filter((e) => e.role === 'companion' && e.text).forEach((e) => window.dispatchEvent(new CustomEvent('castle-story-reply', { detail: e.text })))
  }
  if (!answer.reset && live) {
    const newest = [...answer.entries].reverse().find((e) => e.role === 'voice')
    if (newest) {
      const now = voices.arrive(newest, isPlaying())
      if (now) startVoice(now)
    }
  }
  if (answer.reset) {
    entries.value = resetEntries(entries.value, answer.entries)
  } else if (answer.entries.length) {
    const follow = nearBottom()
    entries.value = mergeEntries(entries.value, answer.entries)
    if (!follow) return
  } else {
    return
  }
  await nextTick()
  if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight
}

function clock(at) {
  if (!at) return ''
  const when = new Date(at)
  return Number.isNaN(when.getTime()) ? '' : when.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })
}

async function send() {
  const text = draft.value.replace(/\s+$/, '')
  if (!text || !props.running) return
  // Several lines go in as one paste, so a new line never sends the message early.
  const payload = text.includes('\n') ? `\u001b[200~${text}\u001b[201~` : text
  try {
    await props.write(payload)
    await props.write('\r')
    worldSignals.sentAt = Date.now() // in the WORLD the crystal brightens
    draft.value = ''
    // the member's bubble at once, one grey tick until the companion's record holds it
    entries.value = [...entries.value, localSend(text)]
    await nextTick()
    grow()
    if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight
  } catch {
    // The words stay in the box to try again.
  }
}

// A queued message goes in now instead of after the companion's current step (Claude Code's Ctrl+X Ctrl+S).
async function sendNow() {
  try {
    await props.write('\u0018\u0013')
  } catch {
    // it stays queued
  }
}

const TICK_TITLES = {
  sending: 'Sent to the terminal',
  queued: 'Waiting in the queue - read after the current step',
  read: 'Read',
}

// Text pastes as usual; a picture or copied files come from the Windows clipboard as paths,
// put where the cursor is.
async function onPaste(event) {
  const data = event.clipboardData
  const hasText = data && Array.from(data.types).includes('text/plain') && data.getData('text/plain')
  if (hasText) return
  event.preventDefault()
  try {
    const pasted = await clipboardPaste()
    if (pasted.kind === 'empty') return
    const el = box.value
    const start = el?.selectionStart ?? draft.value.length
    const end = el?.selectionEnd ?? draft.value.length
    draft.value = draft.value.slice(0, start) + pasted.text + draft.value.slice(end)
    await nextTick()
    grow()
  } catch {
    // Nothing pasted.
  }
}

function onKey(event) {
  if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
    event.preventDefault()
    send()
  }
}

function grow() {
  const el = box.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 160)}px`
}

watch(
  () => props.visible,
  (shown) => {
    if (shown) {
      poll()
      nextTick(() => box.value?.focus())
    }
  },
)

onMounted(() => {
  poll()
  timer = setInterval(poll, POLL_MS)
  menuTimer = setInterval(readMenu, 700)
  clockTimer = setInterval(() => (now.value = Date.now()), 5000)
  document.addEventListener('visibilitychange', poll)
})

onBeforeUnmount(() => {
  clearInterval(timer)
  clearInterval(menuTimer)
  clearInterval(clockTimer)
  document.removeEventListener('visibilitychange', poll)
  voices.stop()
  player.pause()
})

defineExpose({ poll, addPaths })
</script>

<template>
  <div class="chat">
    <div ref="scroller" class="messages" aria-live="polite">
      <p v-if="status === 'waiting'" class="quiet">The chat appears as soon as {{ companion }} starts talking.</p>
      <p v-else-if="status === 'unreadable'" class="quiet">
        Chat cannot read this session.
        <button class="link" @click="emit('show-log')">Use Log</button>
      </p>

      <template v-for="item in items" :key="item.id">
        <div v-if="item.kind === 'message'" class="row" :class="item.entry.role">
          <div class="bubble">
            <MarkdownText :text="item.entry.text" size="chat" />
            <div v-if="item.entry.pictures?.length" class="shared">
              <button
                v-for="path in item.entry.pictures"
                :key="path"
                class="shared-thumb"
                :title="fileName(path)"
                @click="openSharedPicture(path)"
              >
                <img v-if="media[path]" :src="media[path]" :alt="fileName(path)" />
                <span v-else class="thumb-wait">{{ media[path] === null ? fileName(path) : 'Loading…' }}</span>
              </button>
            </div>
            <span class="time">
              <button v-if="item.entry.state === 'queued' && cli === 'claude'" class="send-now" title="Send it now (Ctrl+X Ctrl+S)" @click="sendNow">send now</button>
              {{ clock(item.entry.at) }}
              <span v-if="item.entry.role === 'member' && item.entry.state" class="ticks" :class="item.entry.state" :title="TICK_TITLES[item.entry.state]">{{ item.entry.state === 'read' ? '✓✓' : '✓' }}</span>
            </span>
          </div>
        </div>

        <div v-else-if="item.kind === 'plan'" class="row companion">
          <button class="plan" :title="item.title" @click="planOpen = item.entry.text">
            <span class="plan-tag">PLAN</span>
            <span class="plan-title">{{ item.title }}</span>
            <span class="plan-meta">{{ item.phases ? `${item.phases} ${item.phases === 1 ? 'phase' : 'phases'} - ` : '' }}tap to read<template v-if="clock(item.entry.at)"> · {{ clock(item.entry.at) }}</template></span>
          </button>
        </div>

        <div v-else-if="item.kind === 'pictures'" class="row companion">
          <div class="pictures">
            <button
              v-for="picture in item.list"
              :key="picture.id"
              class="thumb"
              :title="picture.text"
              @click="openViewer(picture)"
            >
              <img v-if="media[picture.path]" :src="media[picture.path]" :alt="picture.text" />
              <span v-else class="thumb-wait">{{ media[picture.path] === null ? 'Not available' : 'Loading…' }}</span>
            </button>
          </div>
        </div>

        <div v-else-if="item.kind === 'voice'" class="row companion">
          <div class="voice" :class="{ playing: playing === item.entry.path }">
            <button class="play" :aria-label="playing === item.entry.path ? 'Pause' : 'Play'" @click="toggleVoice(item.entry)">
              {{ playing === item.entry.path ? '❚❚' : '▶' }}
            </button>
            <span class="voice-label">{{ companion }}'s voice</span>
            <span class="voice-length">{{ length(durations[item.entry.path]) }}</span>
            <button class="again" aria-label="Play again" title="Play again" @click="playVoice(item.entry)">↻</button>
          </div>
        </div>

      </template>

      <div v-if="codexAsking" class="codex-ask" role="status">
        {{ companion }} is asking for your approval.
        <button class="link" @click="emit('show-log')">Answer in Log</button>
      </div>

      <div v-if="typing" class="row companion" :aria-label="`${companion} is typing`">
        <div class="typing" role="status"><span></span><span></span><span></span></div>
      </div>
    </div>

    <div v-if="menu" class="choices" role="group" :aria-label="menu.question || `${companion} is asking`">
      <p class="ask">{{ menu.question || `${companion} is asking` }}</p>
      <div class="options">
        <button v-for="option in menu.options" :key="option.number" class="option" :title="option.hint" @click="choose(option.number)">
          <span class="number">{{ option.number }}</span>{{ option.label }}
        </button>
        <button class="option log" @click="emit('show-log')">Open Log</button>
      </div>
    </div>

    <ImageViewer v-if="viewerStart !== null" :pictures="allPictures" :start="viewerStart" @close="viewerStart = null" />
    <PlanViewer v-if="planOpen !== null" :plan="planOpen" @close="planOpen = null" />

    <div v-if="draftPictures.length" class="attached" aria-label="Pictures in your message">
      <div v-for="path in draftPictures" :key="path" class="attached-item">
        <button class="attached-thumb" :title="fileName(path)" @click="draftViewer = path">
          <img v-if="media[path]" :src="media[path]" :alt="fileName(path)" />
          <span v-else class="thumb-wait">{{ fileName(path) }}</span>
        </button>
        <button class="attached-remove" aria-label="Remove this picture" @click="removeDraftPicture(path)">&times;</button>
      </div>
    </div>
    <ImageViewer
      v-if="draftViewer"
      :pictures="draftPictures.map((p) => ({ path: p, name: fileName(p), url: media[p] }))"
      :start="Math.max(0, draftPictures.indexOf(draftViewer))"
      :can-open="false"
      @close="draftViewer = null"
    />

    <form class="composer" @submit.prevent="send">
      <textarea
        ref="box"
        v-model="draft"
        rows="1"
        :placeholder="running ? `Talk to ${companion}` : `${companion} is resting`"
        :disabled="!running"
        :aria-label="`Message to ${companion}`"
        @keydown="onKey"
        @input="grow"
        @paste="onPaste"
      ></textarea>
      <button type="submit" class="send" :disabled="!running || !draft.trim()" aria-label="Send">&#10148;</button>
    </form>
  </div>
</template>

<style scoped>
.chat {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  padding-right: 10px;
}

.messages {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 4px 8px 0;
  /* no grey bar; the wheel and touch still scroll */
  scrollbar-width: none;
}

.row {
  display: flex;
}

.row.member {
  justify-content: flex-end;
}

.bubble {
  position: relative;
  max-width: 82%;
  padding: 8px 12px 18px;
  border-radius: 14px;
  font-size: 14px;
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.companion .bubble {
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid var(--line);
  border-top-left-radius: 4px;
}

.member .bubble {
  background: color-mix(in srgb, var(--accent) 34%, transparent);
  border-top-right-radius: 4px;
}

.time {
  position: absolute;
  right: 10px;
  bottom: 4px;
  font-size: 10px;
  color: var(--text-dim);
}

/* The member's ticks: one grey while it travels, one while it waits in the queue, two blue once read. */
.ticks {
  margin-left: 3px;
  font-size: 11px;
  letter-spacing: -3px;
}

.ticks.sending {
  opacity: 0.45;
}

.ticks.read {
  color: #7fd8ff;
}

.send-now {
  margin-right: 6px;
  padding: 0 6px;
  border: 1px solid currentColor;
  border-radius: 8px;
  background: none;
  color: inherit;
  font: inherit;
  font-size: 10px;
  cursor: pointer;
}

/* room for the time and the ticks (and "send now") under a short message */
.member .bubble {
  min-width: 96px;
}

.member .bubble:has(.send-now) {
  min-width: 170px;
}

.send-now:hover {
  color: var(--text);
}

/* Three dots rising in turn, as a messenger shows someone typing. */
.typing {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 12px 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  border-top-left-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
}

.typing span {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-dim);
  animation: typing 1.2s infinite ease-in-out;
}

.typing span:nth-child(2) {
  animation-delay: 0.15s;
}

.typing span:nth-child(3) {
  animation-delay: 0.3s;
}

@keyframes typing {
  0%,
  60%,
  100% {
    transform: translateY(0);
    opacity: 0.45;
  }
  30% {
    transform: translateY(-5px);
    opacity: 1;
  }
}

@media (prefers-reduced-motion: reduce) {
  .typing span {
    animation: none;
  }
}

.quiet {
  margin: auto;
  font-size: 13px;
  color: var(--text-dim);
  text-align: center;
}

button {
  font: inherit;
  cursor: pointer;
}

.link {
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent);
  font-size: 12px;
  text-decoration: underline;
}


.pictures {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  max-width: 82%;
}

.thumb {
  display: grid;
  place-items: center;
  width: 180px;
  height: 120px;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.25);
  cursor: zoom-in;
}

.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-wait {
  font-size: 11px;
  color: var(--text-dim);
}

.plan {
  display: flex;
  flex-direction: column;
  gap: 3px;
  width: min(320px, 82%);
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-left: 3px solid var(--accent);
  border-radius: 4px 14px 14px 14px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--text);
  text-align: left;
}

.plan:hover {
  background: var(--hover);
}

.plan-tag {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.18em;
  color: var(--accent);
}

.plan-title {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.plan-meta {
  font-size: 11px;
  color: var(--text-dim);
}

.voice {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.08);
}

.row.companion:has(.voice-words) {
  flex-direction: column;
  align-items: flex-start;
}

.voice-words {
  max-width: 82%;
  margin-top: 6px;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: 4px 14px 14px 14px;
  background: rgba(255, 255, 255, 0.06);
  font-size: 13.5px;
  line-height: 1.5;
}

.voice-words p {
  margin: 0;
  white-space: pre-wrap;
}

.voice-words .english {
  margin-top: 6px;
  color: var(--text-dim);
  font-style: italic;
}

.voice.playing {
  border-color: var(--accent);
}

.voice .play,
.voice .again {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  font-size: 12px;
}

.voice .play {
  background: var(--accent);
  color: #160f33;
}

.voice .again {
  background: none;
  color: var(--text-dim);
  font-size: 16px;
}

.voice-label {
  font-size: 13px;
}

.voice-length {
  min-width: 32px;
  font-size: 12px;
  color: var(--text-dim);
}

.choices {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 8px;
  padding: 10px 12px;
  border: 1px solid var(--accent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
}

.ask {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.options {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.option {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font-size: 12.5px;
  text-align: left;
}

.option:hover {
  background: var(--hover);
}

.option .number {
  display: grid;
  place-items: center;
  flex: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--accent);
  color: #160f33;
  font-size: 11px;
  font-weight: 700;
}

.option.log {
  color: var(--text-dim);
}

.shared {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.shared-thumb,
.attached-thumb {
  display: grid;
  place-items: center;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  cursor: zoom-in;
}

.shared-thumb {
  width: 150px;
  height: 100px;
}

.shared-thumb img,
.attached-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.attached {
  display: flex;
  gap: 8px;
  padding: 8px 2px 0;
  overflow-x: auto;
}

.attached-item {
  position: relative;
  flex: none;
}

.attached-thumb {
  width: 84px;
  height: 64px;
  font-size: 10px;
  color: var(--text-dim);
  overflow-wrap: anywhere;
}

.attached-remove {
  position: absolute;
  top: -6px;
  right: -6px;
  width: 20px;
  height: 20px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--accent);
  color: #160f33;
  font-size: 14px;
  line-height: 20px;
}

.composer {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--line);
}

textarea {
  flex: 1;
  min-height: 38px;
  max-height: 160px;
  padding: 9px 12px;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: rgba(0, 0, 0, 0.22);
  color: var(--text);
  font: inherit;
  font-size: 14px;
  line-height: 1.4;
  resize: none;
  box-sizing: border-box;
  scrollbar-width: none;
}

textarea:disabled {
  opacity: 0.55;
}

.send {
  width: 38px;
  height: 38px;
  border: 0;
  border-radius: 50%;
  background: var(--accent);
  color: #160f33;
  font-size: 16px;
}

.send:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

textarea:focus-visible,
button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.codex-ask {
  align-self: center;
  margin: 8px 0;
  padding: 8px 14px;
  border: 1px solid var(--warn);
  border-radius: 10px;
  color: var(--text);
  font-size: 13px;
}
</style>
