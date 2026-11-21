<script setup>
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { chatAttach, clipboardPaste } from '../api/castle.js'
import ImageViewer from './ImageViewer.vue'
import { isMac, shortcutLabel } from '../platform.js'
import { micShortcutText } from './voicehold.js'

// Chat's box: the member's words, the pictures they drop, paste or type as paths (shown above the
// box, removable, opened in a viewer), Enter to send, Shift+Enter for a new line.
const props = defineProps({
  running: { type: Boolean, default: false },
  placeholder: { type: String, default: '' },
  companion: { type: String, default: 'Your companion' },
  // path -> data URL, shared with the conversation
  media: { type: Object, required: true },
  loadPath: { type: Function, required: true },
  // (text) -> Promise<boolean>: true when the words went in, and the box empties
  send: { type: Function, required: true },
  // The mic, while Claude Code's voice is on: { ready, hint, shortcut, hold: voiceMic(...) }; null for none
  mic: { type: Object, default: null },
  // Chat is the tab showing: switching away stops the mic
  visible: { type: Boolean, default: true },
})

const draft = ref('')
const box = ref(null)
const draftViewer = ref(null)

const fileName = (path) => path.split(/[\\/]/).pop()

// Picture paths in the box as the member writes: dropped, pasted or typed. On a Mac an absolute
// POSIX path (/Users/…/shot.png) counts too; a URL's path (https://…/a.png) does not.
const PICTURE_PATH = isMac
  ? /[A-Za-z]:[\\/][^"'<>|*?\r\n]*?\.(?:png|jpe?g|webp)|(?<![\w:/\\.~-])\/[^"'<>|*?\r\n:]*?\.(?:png|jpe?g|webp|gif)/gi
  : /[A-Za-z]:[\\/][^"'<>|*?\r\n]*?\.(?:png|jpe?g|webp)/gi
const draftPictures = computed(() => [...new Set(draft.value.match(PICTURE_PATH) ?? [])])

// A path the member typed or pasted as text is the member's too: the core checks it is a picture, then it shows.
watch(draftPictures, async (paths) => {
  const fresh = paths.filter((p) => !props.media[p])
  if (!fresh.length) return
  try {
    await chatAttach(fresh)
  } catch {
    // Shown by name only.
  }
  fresh.forEach(props.loadPath)
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

async function submit() {
  const text = draft.value.replace(/\s+$/, '')
  if (!text || !props.running) return
  if (await props.send(text)) {
    draft.value = ''
    await nextTick()
    grow()
  }
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
    submit()
  }
}

function grow() {
  const el = box.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 160)}px`
}

// ---------- The mic: click to talk, click again to stop ----------

// '' | 'listening' (on) | 'writing' (stopped, Claude Code writes the words down)
const listening = ref('')
const micNote = ref('')
// the mic that was started, so the stop reaches it even if the button has gone meanwhile
let recording = null

function micStart() {
  if (!props.mic?.ready || !props.running || !props.visible || listening.value) return
  micNote.value = ''
  recording = props.mic.hold
  // the 2-minute cap stops it as usual: the words come to the box
  if (recording.start({ onCap: micStop })) listening.value = 'listening'
}

/** Stopped: the words Claude Code wrote down come into the box, to look over before Send. */
async function micStop() {
  if (listening.value !== 'listening') return
  listening.value = 'writing'
  let heard
  try {
    heard = await recording.stop()
  } catch {
    heard = { text: '', note: 'Your words could not be read - open Log to see them.' }
  }
  listening.value = ''
  micNote.value = heard.note
  if (!heard.text) return
  const before = draft.value.replace(/\s+$/, '')
  draft.value = (before ? `${before} ` : '') + heard.text
  await nextTick()
  grow()
  const el = box.value
  el?.focus()
  el?.setSelectionRange(draft.value.length, draft.value.length)
}

/** Esc: Claude Code's prompt is emptied and nothing comes to the box. */
async function micCancel() {
  if (listening.value !== 'listening') return
  listening.value = 'writing'
  let heard
  try {
    heard = await recording.cancel()
  } catch {
    heard = { text: '', note: 'Cancelled, but the Log could not be read - open Log to empty its prompt.' }
  }
  listening.value = ''
  micNote.value = heard.note
}

const micToggle = () => (listening.value === 'listening' ? micStop() : micStart())

// Leaving Castle's window, Castle hidden, the Chat tab switched away, the session ending or voice
// turned off while the mic is on: it stops as usual.
const whenHidden = () => document.visibilityState === 'hidden' && micStop()
window.addEventListener('blur', micStop)
document.addEventListener('visibilitychange', whenHidden)
watch(() => !!props.mic && props.running && props.visible, (on) => on || micStop())
onBeforeUnmount(() => {
  window.removeEventListener('blur', micStop)
  document.removeEventListener('visibilitychange', whenHidden)
  micStop()
})

// the mic's shortcut in this computer's words (⌘⇧M), as chosen in Settings > Chat > Mic
const micKeys = computed(() => shortcutLabel(micShortcutText(props.mic?.shortcut)))
const micStatus = computed(() => {
  if (listening.value === 'listening') return `🎙 Listening… click or ${micKeys.value} to stop · Esc to cancel`
  if (listening.value === 'writing') return 'Writing your words down…'
  return micNote.value
})
const micTitle = computed(() => (listening.value === 'listening' ? `Stop (${micKeys.value}) · Esc to cancel` : `${props.mic?.hint ?? ''} (${micKeys.value})`))

defineExpose({
  addPaths,
  focus: () => box.value?.focus(),
  hasText: () => !!draft.value.trim(),
  micToggle,
  micCancel,
  micOn: () => listening.value === 'listening',
})
</script>

<template>
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

  <p v-if="micStatus" class="mic-status" :class="{ live: listening === 'listening' }" aria-live="polite">{{ micStatus }}</p>

  <form class="composer" @submit.prevent="submit">
    <textarea
      ref="box"
      v-model="draft"
      rows="1"
      :placeholder="placeholder"
      :disabled="!running"
      :aria-label="`Message to ${companion}`"
      @keydown="onKey"
      @input="grow"
      @paste="onPaste"
    ></textarea>
    <button
      v-if="mic"
      type="button"
      class="mic"
      :class="{ on: listening === 'listening' }"
      :disabled="!running || !mic.ready || listening === 'writing'"
      :title="micTitle"
      :aria-label="listening === 'listening' ? 'Stop the mic' : 'Talk'"
      :aria-keyshortcuts="isMac ? 'Meta+Shift+M' : 'Control+Shift+M'"
      :aria-pressed="listening === 'listening'"
      @click="micToggle"
    >&#127897;</button>
    <button type="submit" class="send" :disabled="!running || !draft.trim()" aria-label="Send">&#10148;</button>
  </form>
</template>

<style scoped>
button {
  font: inherit;
  cursor: pointer;
}

.thumb-wait {
  font-size: 11px;
  color: var(--text-dim);
}

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

textarea::-webkit-scrollbar {
  display: none;
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

.mic {
  width: 38px;
  height: 38px;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.22);
  color: var(--text);
  font-size: 16px;
}

.mic.on {
  border-color: #e5484d;
  background: color-mix(in srgb, #e5484d 38%, transparent);
  box-shadow: 0 0 0 4px color-mix(in srgb, #e5484d 24%, transparent);
  animation: mic-on 1.4s ease-in-out infinite;
}

@keyframes mic-on {
  50% {
    box-shadow: 0 0 0 7px color-mix(in srgb, #e5484d 10%, transparent);
  }
}

@media (prefers-reduced-motion: reduce) {
  .mic.on {
    animation: none;
  }
}

.mic:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.mic-status {
  margin: 6px 2px 0;
  font-size: 12px;
  color: var(--text-dim);
}

.mic-status.live {
  color: #ff6b6f;
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
</style>
