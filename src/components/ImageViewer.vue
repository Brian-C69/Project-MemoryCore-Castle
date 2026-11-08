<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { chatMediaOpen } from '../api/castle.js'

const props = defineProps({
  // [{ path, name, url }] - every picture in this chat, in order
  pictures: { type: Array, required: true },
  start: { type: Number, default: 0 },
  // Chat's pictures can be opened in Windows; other rooms show them here only.
  canOpen: { type: Boolean, default: true },
})
const emit = defineEmits(['close'])

const index = ref(props.start)
const scale = ref(1)
const offset = ref({ x: 0, y: 0 })
const problem = ref('')
let dragFrom = null

const picture = computed(() => props.pictures[index.value])

function reset() {
  scale.value = 1
  offset.value = { x: 0, y: 0 }
  problem.value = ''
}

function go(step) {
  if (props.pictures.length < 2) return
  index.value = (index.value + step + props.pictures.length) % props.pictures.length
  reset()
}

function zoomAt(factor) {
  scale.value = Math.min(6, Math.max(1, scale.value * factor))
  if (scale.value === 1) offset.value = { x: 0, y: 0 }
}

function onWheel(event) {
  event.preventDefault()
  zoomAt(event.deltaY < 0 ? 1.2 : 1 / 1.2)
}

function onClickImage() {
  if (dragFrom?.moved) return
  scale.value > 1 ? reset() : zoomAt(2)
}

function startDrag(event) {
  if (scale.value === 1) return
  dragFrom = { x: event.clientX - offset.value.x, y: event.clientY - offset.value.y, moved: false }
  const move = (e) => {
    dragFrom.moved = true
    offset.value = { x: e.clientX - dragFrom.x, y: e.clientY - dragFrom.y }
  }
  const stop = () => {
    window.removeEventListener('pointermove', move)
    window.removeEventListener('pointerup', stop)
    setTimeout(() => (dragFrom = null), 0)
  }
  window.addEventListener('pointermove', move)
  window.addEventListener('pointerup', stop)
}

async function openOutside(folder) {
  problem.value = ''
  try {
    await chatMediaOpen(picture.value.path, folder)
  } catch (reason) {
    problem.value = String(reason)
  }
}

// The viewer holds the keys while it is open, so Esc never reaches the terminal.
function onKey(event) {
  const handled = { Escape: () => emit('close'), ArrowLeft: () => go(-1), ArrowRight: () => go(1) }[event.key]
  if (handled) {
    event.preventDefault()
    event.stopImmediatePropagation()
    handled()
  }
}

watch(() => props.start, (value) => {
  index.value = value
  reset()
})

onMounted(() => window.addEventListener('keydown', onKey, true))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey, true))
</script>

<template>
  <Teleport to="body">
    <div class="viewer" role="dialog" aria-modal="true" :aria-label="picture?.name" @click.self="emit('close')">
      <button class="close" aria-label="Close" @click="emit('close')">&times;</button>
      <button v-if="pictures.length > 1" class="nav prev" aria-label="Previous picture" @click="go(-1)">&#8249;</button>
      <button v-if="pictures.length > 1" class="nav next" aria-label="Next picture" @click="go(1)">&#8250;</button>

      <div class="stage" @wheel="onWheel" @click.self="emit('close')">
        <img
          v-if="picture?.url"
          :src="picture.url"
          :alt="picture.name"
          :class="{ zoomed: scale > 1 }"
          :style="{ transform: `translate(${offset.x}px, ${offset.y}px) scale(${scale})` }"
          draggable="false"
          @click="onClickImage"
          @pointerdown="startDrag"
        />
        <p v-else class="loading">Opening the picture&hellip;</p>
      </div>

      <footer class="bar">
        <span class="name">{{ picture?.name }}</span>
        <span v-if="pictures.length > 1" class="count">{{ index + 1 }} / {{ pictures.length }}</span>
        <span v-if="problem" class="problem">{{ problem }}</span>
        <span class="spacer"></span>
        <template v-if="canOpen">
          <button @click="openOutside(true)">Open folder</button>
          <button @click="openOutside(false)">Open in Photos</button>
        </template>
      </footer>
    </div>
  </Teleport>
</template>

<style scoped>
.viewer {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  flex-direction: column;
  background: rgba(6, 4, 20, 0.92);
  backdrop-filter: blur(6px);
  color: #f3efff;
}

.stage {
  display: grid;
  place-items: center;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  padding: 48px 72px 0;
}

img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 6px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  cursor: zoom-in;
  transition: transform 0.12s ease-out;
  user-select: none;
}

img.zoomed {
  cursor: grab;
  transition: none;
}

.loading {
  color: #b6add9;
}

button {
  border: 1px solid rgba(200, 180, 255, 0.38);
  border-radius: 8px;
  background: rgba(20, 16, 58, 0.7);
  color: inherit;
  font: inherit;
  cursor: pointer;
}

button:hover {
  background: rgba(200, 180, 255, 0.18);
}

button:focus-visible {
  outline: 2px solid #b48cff;
  outline-offset: 2px;
}

.close {
  position: absolute;
  top: 14px;
  right: 18px;
  width: 40px;
  height: 40px;
  font-size: 24px;
  z-index: 1;
}

.nav {
  position: absolute;
  top: 50%;
  width: 48px;
  height: 64px;
  font-size: 36px;
  transform: translateY(-50%);
  z-index: 1;
}

.prev {
  left: 14px;
}

.next {
  right: 14px;
}

.bar {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 24px 18px;
  font-size: 13px;
}

.bar button {
  padding: 7px 14px;
  font-size: 13px;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.count,
.problem {
  color: #b6add9;
}

.problem {
  color: #ff9aa8;
}

.spacer {
  flex: 1;
}
</style>
