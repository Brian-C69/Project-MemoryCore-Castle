<script setup>
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { chatAttach, clipboardPaste } from '../api/castle.js'
import ImageViewer from './ImageViewer.vue'
import { isMac } from '../platform.js'

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
  // The mic, while Claude Code's voice is on: { ready, hint, hold: voiceHold(...) }; null for none
  mic: { type: Object, default: null },
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

// ---------- The mic: hold to talk ----------

// '' | 'listening' (held) | 'writing' (released, Claude Code writes the words down)
const listening = ref('')
const micNote = ref('')
// the mic that was pressed, so the release reaches it even if the button has gone meanwhile
let holding = null

function micDown(event) {
  if (!props.mic?.ready || !props.running || listening.value || event.button > 0) return
  event.preventDefault()
  micNote.value = ''
  holding = props.mic.hold
  if (holding.start()) listening.value = 'listening'
}

/** Released: the words Claude Code wrote down come into the box, to look over before Send. */
async function micUp() {
  if (listening.value !== 'listening') return
  listening.value = 'writing'
  let heard
  try {
    heard = await holding.stop()
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

// Space held on the focused mic button works like holding the button (its repeats are ignored).
function micKey(event) {
  if (event.key !== ' ') return
  event.preventDefault()
  if (event.type === 'keyup') micUp()
  else if (!event.repeat) micDown(event)
}

// Leaving Castle's window, the session ending or voice turned off while held are a release.
window.addEventListener('blur', micUp)
watch(() => !!props.mic && props.running, (on) => on || micUp())
onBeforeUnmount(() => {
  window.removeEventListener('blur', micUp)
  micUp()
})

const micStatus = computed(() => {
  if (listening.value === 'listening') return '🎙 Listening… (release to stop)'
  if (listening.value === 'writing') return 'Writing your words down…'
  return micNote.value
})

defineExpose({ addPaths, focus: () => box.value?.focus(), hasText: () => !!draft.value.trim() })
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
      :title="mic.hint"
      aria-label="Hold to talk"
      :aria-pressed="listening === 'listening'"
      @pointerdown="micDown"
      @pointerup="micUp"
      @pointerleave="micUp"
      @pointercancel="micUp"
      @keydown="micKey"
      @keyup="micKey"
      @contextmenu.prevent
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
  touch-action: none;
  user-select: none;
}

.mic.on {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 30%, transparent);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
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
  color: var(--accent);
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
