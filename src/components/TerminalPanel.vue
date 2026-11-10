<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import '@xterm/xterm/css/xterm.css'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import ChatView from './ChatView.vue'
import QuickActions from './QuickActions.vue'
import {
  clipboardCopy,
  clipboardPaste,
  onStoryExit,
  onStoryOutput,
  onTerminalExit,
  onTerminalOutput,
  onWakeRequest,
  takeAutoWake,
  terminalResize,
  terminalStart,
  terminalState,
  terminalWrite,
} from '../api/castle.js'

const props = defineProps({
  // Changes whenever Settings saves, so the session status is read again.
  settingsStamp: { type: Number, default: 0 },
  // undefined for the companion's main session, 'story' for the STORY room's
  room: { type: String, default: undefined },
  // the companion's name, for the buttons and messages
  companion: { type: String, default: 'Your companion' },
  // 'claude' or 'codex'
  cli: { type: String, default: 'claude' },
})
const emit = defineEmits(['settings', 'setup'])

const WIDTH_KEY = 'castle.panelWidth'
const VIEW_KEY = 'castle.panelView'
const MIN_WIDTH = 25
const MAX_WIDTH = 75

const surface = ref(null)
const panel = ref(null)
const running = ref(false)
let starting = false
const activeProvider = ref(null)
const ended = ref(false)
const status = ref({ ready: false, message: '', fix: null })
const problem = ref('')
const width = ref(readWidth())
// CHAT or LOG under the companion's portrait; the wide button gives the panel the whole screen.
const view = ref(readView())
const expanded = ref(false)
const chat = ref(null)
// A file is being dragged over Castle while this panel shows.
const dropping = ref(false)

const quoted = (path) => (/\s/.test(path) ? `"${path}"` : path)

// Files dropped on Castle land where the member is: in Chat's box (pictures previewed), or typed into
// the Log. Only while this panel is the one showing.
function onDragDrop(event) {
  if (!panel.value || panel.value.offsetParent === null) return
  const drop = event.payload
  if (drop.type === 'enter' || drop.type === 'over') dropping.value = true
  else if (drop.type === 'leave') dropping.value = false
  else if (drop.type === 'drop') {
    dropping.value = false
    const paths = drop.paths ?? []
    if (!paths.length) return
    if (view.value === 'chat') chat.value?.addPaths(paths)
    else if (running.value) term.paste(paths.map(quoted).join(' '))
  }
}

function readView() {
  try {
    const saved = localStorage.getItem(VIEW_KEY)
    if (saved === 'chat' || saved === 'log') return saved
  } catch {
    // Storage can be unavailable; Chat is shown.
  }
  return 'chat'
}

function setView(next) {
  view.value = next
  try {
    localStorage.setItem(VIEW_KEY, next)
  } catch {
    // Not kept; the next start shows Log.
  }
}

let term
let fit
let observer
let fitTimer
const stopListening = []

function readWidth() {
  try {
    const saved = Number(localStorage.getItem(WIDTH_KEY))
    if (saved >= MIN_WIDTH && saved <= MAX_WIDTH) return saved
  } catch {
    // Storage can be unavailable; the theme's width is used.
  }
  return null
}

function cssVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function show(state) {
  activeProvider.value = state.provider ?? null
  running.value = state.running
  status.value = state.status
}

function fitNow() {
  if (!fit || !surface.value || surface.value.clientWidth === 0) return
  fit.fit()
  if (running.value) terminalResize(term.cols, term.rows, props.room).catch(() => {})
}

function scheduleFit() {
  clearTimeout(fitTimer)
  fitTimer = setTimeout(fitNow, 60)
}

// The clipboard goes through Windows itself (the core), never the browser's: a paste brings
// text, copied files as their paths, or a picture saved as a PNG whose path is pasted.
async function pasteIntoSession() {
  if (!running.value) return
  try {
    const pasted = await clipboardPaste()
    if (pasted.kind !== 'empty') term.paste(pasted.text)
  } catch {
    // Nothing pasted; the clipboard could not be read.
  }
}

function copySelection() {
  const text = term.getSelection()
  if (text) clipboardCopy(text).catch(() => {})
  term.clearSelection()
}

// Ctrl+V, Ctrl+Shift+V and Shift+Insert paste; Ctrl+C copies a selection and otherwise goes to
// the session as usual (interrupt). preventDefault stops the browser's own paste, so nothing doubles.
function clipboardKeys(event) {
  if (event.type !== 'keydown') return true
  const key = event.key.toLowerCase()
  if ((event.ctrlKey && key === 'v') || (event.shiftKey && event.key === 'Insert')) {
    event.preventDefault()
    pasteIntoSession()
    return false
  }
  if (event.ctrlKey && key === 'c' && term.hasSelection()) {
    event.preventDefault()
    copySelection()
    return false
  }
  return true
}

// Right-click, as in Windows Terminal: copy when text is selected, paste when not.
function onRightClick(event) {
  event.preventDefault()
  if (term.hasSelection()) copySelection()
  else pasteIntoSession()
}

async function wake() {
  if (running.value) {
    term.focus()
    return
  }
  problem.value = ''
  fitNow()
  // The console asks this screen where its cursor is the moment it starts, before Castle has said
  // the session is running; the answer must still reach it or the CLI never begins.
  starting = true
  try {
    show(await terminalStart(term.cols, term.rows, props.room))
    ended.value = false
    term.focus()
  } catch (reason) {
    problem.value = String(reason)
  } finally {
    starting = false
  }
}

function startDrag(event) {
  const box = panel.value.parentElement.getBoundingClientRect()
  const move = (e) => {
    const percent = ((e.clientX - box.left) / box.width) * 100
    width.value = Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, percent)))
  }
  const stop = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', stop)
    try {
      localStorage.setItem(WIDTH_KEY, String(width.value))
    } catch {
      // Not kept; the width resets next time.
    }
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', stop)
  event.preventDefault()
}

onMounted(async () => {
  term = new Terminal({
    allowTransparency: true,
    allowProposedApi: true,
    cursorBlink: true,
    fontFamily: "'Cascadia Mono', Consolas, monospace",
    fontSize: 14,
    lineHeight: 1.15,
    scrollback: 10000,
    theme: {
      background: 'rgba(0, 0, 0, 0)',
      foreground: cssVar('--text'),
      cursor: cssVar('--accent'),
      selectionBackground: 'rgba(143, 184, 255, 0.35)',
    },
  })
  fit = new FitAddon()
  term.loadAddon(fit)
  term.loadAddon(new Unicode11Addon())
  term.unicode.activeVersion = '11'
  term.open(surface.value)
  term.attachCustomKeyEventHandler(clipboardKeys)
  surface.value.addEventListener('contextmenu', onRightClick)
  term.onData((data) => {
    if (running.value || starting) terminalWrite(data, props.room).catch(() => {})
  })

  // Listen before anything starts: the console asks this screen a question before it runs.
  const onOutput = props.room === 'story' ? onStoryOutput : onTerminalOutput
  const onExit = props.room === 'story' ? onStoryExit : onTerminalExit
  stopListening.push(await onOutput((text) => term.write(text)))
  stopListening.push(
    await onExit(async () => {
      running.value = false
      ended.value = true
      show(await terminalState(props.room))
    }),
  )

  fitNow()
  observer = new ResizeObserver(scheduleFit)
  observer.observe(surface.value)

  if (props.room !== 'story') stopListening.push(await onWakeRequest(wake))
  try {
    stopListening.push(await getCurrentWebview().onDragDropEvent(onDragDrop))
  } catch {
    // No drop events outside Castle's window (a browser preview).
  }

  const state = await terminalState(props.room)
  show(state)
  if (state.snapshot) term.write(state.snapshot)

  // Once per sign-in, and only now that this screen is listening (the main session only).
  if (props.room !== 'story' && !state.running && (await takeAutoWake())) {
    if (state.status.ready) await wake()
    else problem.value = `${props.companion} could not start by itself: ${state.status.message}`
  }
})

// The memory folder may have changed in Settings, or while Castle was hidden: read it again.
async function recheck() {
  try {
    show(await terminalState(props.room))
    problem.value = ''
  } catch {
    // The last status stays.
  }
}
watch(() => props.settingsStamp, recheck)
window.addEventListener('castle-provider-changed', recheck)
onBeforeUnmount(() => window.removeEventListener('castle-provider-changed', recheck))
const whenShown = () => {
  if (document.visibilityState === 'visible' && !running.value) recheck()
}
document.addEventListener('visibilitychange', whenShown)
onBeforeUnmount(() => document.removeEventListener('visibilitychange', whenShown))

// Switching CHAT and LOG, or going wide, keeps the session; the Log is refitted and, when it is
// the one showing, the cursor goes back into it.
watch([view, expanded], () => {
  scheduleFit()
  if (running.value && view.value === 'log') setTimeout(() => term.focus(), 80)
})

onBeforeUnmount(() => {
  observer?.disconnect()
  stopListening.forEach((stop) => stop())
  term?.dispose()
})

// What Chat sends goes into the same session, exactly as if typed in the Log.
function writeToSession(data) {
  if (!running.value) return Promise.resolve()
  return terminalWrite(data, props.room)
}

// Chat's answer buttons read the menu on the Log's screen.
function screenLines(count = 25) {
  if (!term) return []
  const buffer = term.buffer.active
  const end = buffer.baseY + term.rows
  const lines = []
  for (let y = Math.max(0, end - count); y < end; y += 1) {
    lines.push(buffer.getLine(y)?.translateToString(true) ?? '')
  }
  return lines
}

defineExpose({ wake, screenLines, writeToSession })
</script>

<template>
  <section
    ref="panel"
    class="terminal-panel"
    :class="{ wide: expanded }"
    :style="!expanded && width ? { width: width + '%' } : null"
    :aria-label="companion"
  >
    <div class="header">
      <div v-if="$slots.header && !expanded" class="portrait"><slot name="header" /></div>
      <div class="tabs" role="tablist" aria-label="Chat or log">
        <button role="tab" :aria-selected="view === 'chat'" :class="{ active: view === 'chat' }" @click="setView('chat')">CHAT</button>
        <button role="tab" :aria-selected="view === 'log'" :class="{ active: view === 'log' }" @click="setView('log')">LOG</button>
        <span class="spacer"></span>
        <button
          class="widen"
          :title="expanded ? 'Back beside the widgets' : 'Fill the screen'"
          :aria-label="expanded ? 'Back beside the widgets' : 'Fill the screen'"
          @click="expanded = !expanded"
        >{{ expanded ? '⤡' : '⤢' }}</button>
      </div>
    </div>

    <QuickActions v-if="room !== 'story'" :running="running" :write="writeToSession" :settings-stamp="settingsStamp" />

    <div class="body">
    <div v-show="view === 'log'" ref="surface" class="surface"></div>
    <div v-show="view === 'chat'" class="chat-area">
      <ChatView
        ref="chat"
        :running="running"
        :visible="view === 'chat'"
        :write="writeToSession"
        :read-screen="screenLines"
        :companion="companion"
        :room="room"
        :cli="activeProvider || cli"
        @show-log="setView('log')"
      />
    </div>

    <div v-if="dropping" class="drop-hint">{{ view === 'chat' ? `Drop to share with ${companion}` : 'Drop to type the path into the Log' }}</div>

    <div v-if="!running" class="asleep">
      <p class="title">{{ ended ? `${companion}'s session ended.` : `${companion} is resting.` }}</p>
      <p class="message">{{ problem || status.message }}</p>
      <div class="buttons">
        <button class="primary" :disabled="!status.ready" @click="wake">Wake {{ companion === 'Your companion' ? 'your companion' : companion }}</button>
        <button v-if="status.fix === 'setup'" @click="emit('setup')">Open setup</button>
        <button v-if="status.fix === 'settings' || status.fix === 'install'" @click="emit('settings')">Settings</button>
      </div>
    </div>
    </div>

    <div v-if="!expanded" class="handle" title="Drag to widen" @pointerdown="startDrag"></div>
  </section>
</template>

<style scoped>
.terminal-panel {
  position: absolute;
  top: var(--panel-top, 24px);
  bottom: var(--panel-bottom, 24px);
  left: var(--panel-left, 24px);
  right: var(--panel-right, auto);
  width: var(--panel-width, 40%);
  height: var(--panel-height, auto);
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 6px 14px 16px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
  box-sizing: border-box;
}

.terminal-panel.wide {
  top: 16px;
  bottom: 16px;
  left: 16px;
  right: auto;
  width: calc(100% - 32px);
  height: auto;
}

.header {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: none;
}

.tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-right: 10px;
  border-bottom: 1px solid var(--line);
}

.tabs button {
  padding: 6px 14px;
  border: 0;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  background: none;
  color: var(--text-dim);
  font-size: 12px;
  letter-spacing: 0.14em;
}

.tabs button.active {
  color: var(--text);
  border-bottom-color: var(--accent);
}

.tabs .spacer {
  flex: 1;
}

.tabs .widen {
  padding: 4px 8px;
  font-size: 16px;
  letter-spacing: 0;
}

.chat-area {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.chat-soon {
  margin: auto;
  font-size: 13px;
  color: var(--text-dim);
}

.body {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
}

.surface {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.surface :deep(.xterm-viewport) {
  background-color: transparent !important;
  scrollbar-width: thin;
  scrollbar-color: var(--line) transparent;
}

.surface :deep(.xterm-viewport::-webkit-scrollbar) {
  width: 8px;
}

.surface :deep(.xterm-viewport::-webkit-scrollbar-track) {
  background: transparent;
}

.surface :deep(.xterm-viewport::-webkit-scrollbar-thumb) {
  background: var(--line);
  border-radius: 4px;
}

.drop-hint {
  position: absolute;
  inset: 6px 16px 6px 6px;
  z-index: 6;
  display: grid;
  place-items: center;
  border: 2px dashed var(--accent);
  border-radius: 14px;
  background: color-mix(in srgb, var(--accent) 14%, rgba(4, 10, 30, 0.6));
  color: var(--text);
  font-size: 15px;
  pointer-events: none;
}

.asleep {
  position: absolute;
  inset: 0 10px 0 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  text-align: center;
  border-radius: 14px;
  background: var(--panel-background);
}

.title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.message {
  margin: 0 0 8px;
  max-width: 420px;
  font-size: 13px;
  color: var(--text-dim);
}

.buttons {
  display: flex;
  gap: 8px;
}

button {
  padding: 8px 16px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 14px;
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

button:hover:not(:disabled) {
  filter: brightness(1.12);
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.handle {
  position: absolute;
  top: 12px;
  right: -5px;
  bottom: 12px;
  width: 10px;
  cursor: ew-resize;
}

.handle:hover {
  background: linear-gradient(to right, transparent 4px, var(--accent) 4px, var(--accent) 6px, transparent 6px);
}
</style>
