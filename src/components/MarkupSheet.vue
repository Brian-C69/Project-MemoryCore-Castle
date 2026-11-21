<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { COLORS, TOOLS, changed, cropOf, drawOp, finish, keptArea, paint, pushOp, rectOf, redo, sizes, startHistory, toolForKey, undo } from './markup.js'
import { isMac, modKey } from '../platform.js'

// The markup sheet: the member draws on a picture they put in a Chat (arrow, box, pen, words, blur,
// crop) and keeps a marked copy. The picture itself is never changed: `save` gets the marks and the
// finished PNG, and the sheet closes once it is saved.
const props = defineProps({
  name: { type: String, default: '' },
  // the original picture, as a data URL
  src: { type: String, required: true },
  // the marks the shown copy was made with: they stay editable until the message is sent
  ops: { type: Array, default: () => [] },
  // (ops, toDataUrl) -> Promise; throws with the reason when the copy could not be saved
  save: { type: Function, required: true },
})
const emit = defineEmits(['close'])

const canvas = ref(null)
const stage = ref(null)
const textBox = ref(null)
const history = shallowRef(startHistory(props.ops))
const tool = ref('arrow')
const color = ref(COLORS[0].value)
const asking = ref(false)
const saving = ref(false)
const problem = ref('')
const loaded = ref(false)
// the words being typed for a Text mark: { p (picture point), left, top (on screen), value }
const typing = ref(null)

// pixels are read back for every blur, so the canvases say so
const context = (c) => c.getContext('2d', { willReadFrequently: true })
let picture = null
let width = 0
let height = 0
// the picture with every finished mark, so a drag repaints only its own mark
let finished = null
let draft = null

const undoKey = isMac ? '⌘Z' : 'Ctrl+Z'
const redoKey = isMac ? '⌘⇧Z' : 'Ctrl+Shift+Z'
const canUndo = computed(() => history.value.ops.length > 0)
const canRedo = computed(() => history.value.undone.length > 0)
const hasMarks = computed(() => history.value.ops.length > 0)

function repaintFinished() {
  finished ??= document.createElement('canvas')
  finished.width = width
  finished.height = height
  paint(context(finished), picture, history.value.ops, width, height)
}

function show() {
  const g = canvas.value && context(canvas.value)
  if (!g || !finished) return
  g.clearRect(0, 0, width, height)
  g.drawImage(finished, 0, 0)
  if (draft && draft.t !== 'crop') drawOp(g, draft.t === 'box' || draft.t === 'blur' ? { ...draft, r: rectOf(draft.a, draft.b) } : draft, width, height)
  const crop = draft?.t === 'crop' ? rectOf(draft.a, draft.b) : cropOf(history.value.ops)
  if (crop) {
    g.save()
    g.fillStyle = 'rgba(0, 0, 0, 0.55)'
    g.beginPath()
    g.rect(0, 0, width, height)
    g.rect(crop.x, crop.y, crop.w, crop.h)
    g.fill('evenodd')
    g.setLineDash([sizes(width, height).line * 3, sizes(width, height).line * 2])
    g.strokeStyle = '#fff'
    g.lineWidth = Math.max(1, sizes(width, height).line / 2)
    g.strokeRect(crop.x, crop.y, crop.w, crop.h)
    g.restore()
  }
}

function setHistory(next) {
  if (next === history.value) return
  history.value = next
  repaintFinished()
  show()
}

const point = (event) => {
  const r = canvas.value.getBoundingClientRect()
  return { x: ((event.clientX - r.left) * width) / r.width, y: ((event.clientY - r.top) * height) / r.height }
}

function onDown(event) {
  if (!loaded.value || saving.value || event.button !== 0) return
  // the canvas keeps the press: no text selection, and the words box keeps its focus
  event.preventDefault()
  if (typing.value) return commitText()
  const p = point(event)
  if (tool.value === 'text') return startText(event, p)
  canvas.value.setPointerCapture(event.pointerId)
  draft = tool.value === 'pen' ? { t: 'pen', c: color.value, pts: [p] } : { t: tool.value, c: color.value, a: p, b: p }
  show()
}

function onMove(event) {
  if (!draft) return
  const p = point(event)
  if (draft.t === 'pen') draft.pts.push(p)
  else draft.b = p
  show()
}

function onUp() {
  if (!draft) return
  const kept = finish(draft, width, height)
  draft = null
  if (kept) setHistory(pushOp(history.value, kept))
  else show()
}

async function startText(event, p) {
  const box = stage.value.getBoundingClientRect()
  const scale = canvas.value.getBoundingClientRect().width / width
  typing.value = {
    p,
    left: event.clientX - box.left + stage.value.scrollLeft,
    top: event.clientY - box.top + stage.value.scrollTop,
    size: Math.max(12, sizes(width, height).font * scale),
    value: '',
  }
  await nextTick()
  textBox.value?.focus()
}

function commitText(keep = true) {
  const words = typing.value?.value.trim()
  const p = typing.value?.p
  typing.value = null
  if (keep && words) setHistory(pushOp(history.value, { t: 'text', c: color.value, p, s: words }))
}

function leave() {
  if (!changed(history.value) || asking.value) return emit('close')
  asking.value = true
}

function toDataUrl() {
  const all = document.createElement('canvas')
  all.width = width
  all.height = height
  paint(context(all), picture, history.value.ops, width, height)
  const area = keptArea(history.value.ops, width, height)
  if (area.w === width && area.h === height) return all.toDataURL('image/png')
  const out = document.createElement('canvas')
  out.width = area.w
  out.height = area.h
  out.getContext('2d').drawImage(all, area.x, area.y, area.w, area.h, 0, 0, area.w, area.h)
  return out.toDataURL('image/png')
}

async function use() {
  if (saving.value) return
  commitText()
  saving.value = true
  problem.value = ''
  try {
    await props.save(history.value.ops, toDataUrl)
    emit('close')
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  } finally {
    saving.value = false
  }
}

// The sheet holds the keys while it is open, so none reach the terminal or the composer behind it.
function onKey(event) {
  event.stopImmediatePropagation()
  if (event.target === textBox.value) {
    if (event.key === 'Enter') {
      event.preventDefault()
      commitText()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      commitText(false)
    }
    return
  }
  const key = event.key.toLowerCase()
  if (modKey(event) && (key === 'z' || key === 'y')) {
    event.preventDefault()
    setHistory(key === 'y' || event.shiftKey ? redo(history.value) : undo(history.value))
  } else if (event.key === 'Escape') {
    event.preventDefault()
    leave()
  } else if (!event.metaKey && !event.ctrlKey && !event.altKey && toolForKey(key) && event.target?.tagName !== 'INPUT') {
    tool.value = toolForKey(key)
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKey, true)
  const image = new Image()
  image.onload = () => {
    picture = image
    width = image.naturalWidth
    height = image.naturalHeight
    canvas.value.width = width
    canvas.value.height = height
    repaintFinished()
    loaded.value = true
    show()
  }
  image.onerror = () => (problem.value = 'The picture could not be opened.')
  image.src = props.src
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey, true))
</script>

<template>
  <Teleport to="body">
    <div class="veil" @click.self="leave">
      <div class="sheet" role="dialog" aria-modal="true" :aria-label="`Mark up ${name || 'picture'}`">
        <div class="bar">
          <div class="group" role="toolbar" aria-label="Tools">
            <button
              v-for="t in TOOLS"
              :key="t.id"
              class="tool"
              :aria-pressed="tool === t.id"
              :title="`${t.label} (${t.key.toUpperCase()})`"
              @click="tool = t.id"
            >
              <span aria-hidden="true">{{ t.icon }}</span> {{ t.label }} <kbd>{{ t.key.toUpperCase() }}</kbd>
            </button>
          </div>
          <span class="sep"></span>
          <div class="group" role="group" aria-label="Colour">
            <button
              v-for="c in COLORS"
              :key="c.value"
              class="swatch"
              :style="{ background: c.value }"
              :aria-label="c.name"
              :aria-pressed="color === c.value"
              @click="color = c.value"
            ></button>
          </div>
          <span class="sep"></span>
          <div class="group">
            <button class="tool" :disabled="!canUndo" :title="`Undo (${undoKey})`" @click="setHistory(undo(history))">↶ Undo</button>
            <button class="tool" :disabled="!canRedo" :title="`Redo (${redoKey})`" @click="setHistory(redo(history))">↷ Redo</button>
          </div>
        </div>

        <div ref="stage" class="stage">
          <canvas
            ref="canvas"
            :class="{ hidden: !loaded }"
            :aria-label="name"
            @pointerdown="onDown"
            @pointermove="onMove"
            @pointerup="onUp"
            @pointercancel="onUp"
          ></canvas>
          <p v-if="!loaded && !problem" class="wait">Opening the picture&hellip;</p>
          <input
            v-if="typing"
            ref="textBox"
            v-model="typing.value"
            class="words"
            placeholder="Type, then Enter"
            aria-label="Words for the picture"
            :style="{ left: `${typing.left}px`, top: `${typing.top}px`, color, fontSize: `${typing.size}px` }"
            @blur="commitText()"
          />
        </div>

        <div class="foot">
          <template v-if="asking">
            <span class="note">Leave without the marks?</span>
            <div class="group">
              <button class="btn" @click="asking = false">Keep marking</button>
              <button class="btn" @click="emit('close')">Leave</button>
            </div>
          </template>
          <template v-else>
            <span v-if="problem" class="note problem">{{ problem }}</span>
            <span v-else class="note">The original stays as it is. The marked copy is what gets sent.</span>
            <div class="group">
              <button class="btn" @click="leave">Cancel</button>
              <button class="btn primary" :disabled="!loaded || saving" @click="use">
                {{ saving ? 'Saving…' : hasMarks ? 'Use marked copy' : 'Use original' }}
              </button>
            </div>
          </template>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.veil {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  padding: 16px;
  background: rgba(6, 4, 20, 0.78);
}

.sheet {
  width: min(1100px, 100%);
  max-height: 100%;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: rgba(20, 16, 58, 0.96);
  color: var(--text);
}

button {
  font: inherit;
  cursor: pointer;
}

.bar,
.foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 14px;
  padding: 10px 14px;
}

.bar {
  border-bottom: 1px solid var(--line);
}

.foot {
  justify-content: space-between;
  border-top: 1px solid var(--line);
}

.group {
  display: flex;
  align-items: center;
  gap: 4px;
}

.sep {
  width: 1px;
  height: 22px;
  background: var(--line);
}

.tool {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
}

.tool[aria-pressed='true'] {
  border-color: var(--line);
  background: rgba(200, 180, 255, 0.16);
  color: var(--text);
}

.tool:disabled {
  opacity: 0.35;
  cursor: default;
}

kbd {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 10px;
  opacity: 0.6;
}

.swatch {
  width: 20px;
  height: 20px;
  padding: 0;
  border: 2px solid transparent;
  border-radius: 50%;
}

.swatch[aria-pressed='true'] {
  border-color: var(--text);
}

.stage {
  position: relative;
  display: grid;
  place-items: center;
  min-height: 200px;
  padding: 14px;
  overflow: auto;
  background: repeating-conic-gradient(rgba(0, 0, 0, 0.35) 0 25%, rgba(0, 0, 0, 0.2) 0 50%) 0 0 / 16px 16px;
}

canvas {
  display: block;
  max-width: 100%;
  max-height: calc(100vh - 170px);
  border-radius: 6px;
  cursor: crosshair;
  touch-action: none;
}

canvas.hidden {
  visibility: hidden;
  position: absolute;
}

.wait {
  color: var(--text-dim);
}

.words {
  position: absolute;
  min-width: 140px;
  padding: 2px 4px;
  border: 1px dashed var(--accent);
  background: rgba(0, 0, 0, 0.45);
  font-weight: 600;
  outline: none;
}

.note {
  flex: 1;
  min-width: 200px;
  font-size: 12.5px;
  color: var(--text-dim);
}

.problem {
  color: #ff9a9a;
}

.btn {
  padding: 6px 13px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
}

.btn.primary {
  border-color: var(--accent);
  background: var(--accent);
  color: #160f33;
}

.btn:disabled {
  opacity: 0.5;
  cursor: default;
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
