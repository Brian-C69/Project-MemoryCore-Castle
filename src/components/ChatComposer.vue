<script setup>
import { computed, nextTick, ref, watch } from 'vue'
import { chatAttach, chatMarkupSave, clipboardPaste } from '../api/castle.js'
import MarkupSheet from './MarkupSheet.vue'
import { applyMarked, keepMarks, markupSource, saveMarkedCopy } from './markupsave.js'
import { isMac } from '../platform.js'

// Chat's box: the member's words, the pictures they drop, paste or type as paths (shown above the
// box, removable, opened in the markup sheet to be marked up), Enter to send, Shift+Enter for a new line.
const props = defineProps({
  running: { type: Boolean, default: false },
  placeholder: { type: String, default: '' },
  companion: { type: String, default: 'Your companion' },
  // path -> data URL, shared with the conversation
  media: { type: Object, required: true },
  loadPath: { type: Function, required: true },
  // (text) -> Promise<boolean>: true when the words went in, and the box empties
  send: { type: Function, required: true },
})

const draft = ref('')
const box = ref(null)
// the picture the markup sheet is open for: { shown, original, ops, previous }
const marking = ref(null)
// marked copy path -> { original, ops }: a copy's marks stay editable until the message is sent
const marks = ref({})

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

// a picture sent or taken out of the message forgets its marks
watch(draftPictures, (paths) => (marks.value = keepMarks(marks.value, paths)))

function openMarkup(path) {
  const source = markupSource(marks.value, path)
  if (!props.media[source.original]) return
  marking.value = { shown: path, ...source }
}

// "Use marked copy": the copy is saved beside the original and takes its place in the message.
async function useMarked(ops, toDataUrl) {
  const { shown, original, previous } = marking.value
  const picture = ops.length ? toDataUrl() : null
  const saved = await saveMarkedCopy({ original, previous, ops, picture, save: chatMarkupSave })
  if (saved !== original) props.media[saved] = picture
  const next = applyMarked({ draft: draft.value, marks: marks.value }, { shown, original, ops, saved })
  marks.value = next.marks
  draft.value = next.draft
}

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

defineExpose({ addPaths, focus: () => box.value?.focus(), hasText: () => !!draft.value.trim() })
</script>

<template>
  <div v-if="draftPictures.length" class="attached" aria-label="Pictures in your message">
    <div v-for="path in draftPictures" :key="path" class="attached-item">
      <button class="attached-thumb" :title="`${fileName(path)} - click to mark it up`" @click="openMarkup(path)">
        <img v-if="media[path]" :src="media[path]" :alt="fileName(path)" />
        <span v-else class="thumb-wait">{{ fileName(path) }}</span>
      </button>
      <span v-if="marks[path]" class="attached-badge">marked</span>
      <button
        v-if="media[markupSource(marks, path).original]"
        class="attached-markup"
        title="Mark up"
        aria-label="Mark up this picture"
        @click="openMarkup(path)"
      >&#9998;</button>
      <button class="attached-remove" aria-label="Remove this picture" @click="removeDraftPicture(path)">&times;</button>
    </div>
  </div>
  <MarkupSheet
    v-if="marking"
    :name="fileName(marking.original)"
    :src="media[marking.original]"
    :ops="marking.ops"
    :save="useMarked"
    @close="marking = null"
  />

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
  cursor: pointer;
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

.attached-markup {
  position: absolute;
  top: -6px;
  right: 18px;
  width: 20px;
  height: 20px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: rgba(20, 16, 58, 0.95);
  color: var(--accent);
  font-size: 11px;
  line-height: 18px;
}

.attached-badge {
  position: absolute;
  left: 4px;
  bottom: 4px;
  padding: 1px 6px;
  border-radius: 999px;
  background: rgba(6, 4, 20, 0.85);
  color: #7fe0b0;
  font-size: 9.5px;
  pointer-events: none;
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
