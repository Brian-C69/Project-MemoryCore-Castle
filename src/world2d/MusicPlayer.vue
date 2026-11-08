<script setup>
// Music for the 2D WORLD: an on/off switch, shuffle, next and a volume slider over one audio
// element. It plays only while switched on, and stops when switched off or closed.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { nextIndex, shuffle } from './slideshow.js'

const props = defineProps({
  // [{ title, url }]
  tracks: { type: Array, default: () => [] },
  enabled: { type: Boolean, default: false },
  // 0..1
  volume: { type: Number, default: 0.5 },
})
const emit = defineEmits(['update:enabled', 'update:volume'])

const order = ref([]) // indexes into tracks, in playing order
const at = ref(0) // position in order
const blocked = ref(false) // the system refused to start sound until the member clicks
let audio = null

const track = computed(() => props.tracks[order.value[at.value]] ?? null)
const clampVolume = (v) => Math.min(1, Math.max(0, Number(v) || 0))
const level = computed(() => Math.round(clampVolume(props.volume) * 100))

function reorder(keep = null) {
  const indexes = props.tracks.map((_, i) => i)
  order.value = shuffle(indexes, Date.now())
  // the track already playing stays first, so a shuffle never cuts it off
  if (keep !== null && order.value.includes(keep)) order.value = [keep, ...order.value.filter((i) => i !== keep)]
  at.value = 0
}

function load() {
  if (!audio) return
  const url = track.value?.url
  if (!url) {
    audio.pause()
    audio.removeAttribute('src')
    return
  }
  if (audio.getAttribute('src') !== url) {
    audio.src = url
    audio.loop = props.tracks.length === 1
  }
}

function play() {
  if (!audio || !props.enabled || !track.value) return
  load()
  audio.volume = clampVolume(props.volume)
  const started = audio.play()
  if (started?.then) {
    started.then(
      () => (blocked.value = false),
      (reason) => {
        if (reason?.name === 'NotAllowedError') blocked.value = true
      },
    )
  }
}

function stop() {
  blocked.value = false
  audio?.pause()
}

function next() {
  if (!props.tracks.length) return
  at.value = nextIndex(at.value, order.value.length, 1)
  // round the whole list once: a fresh order for the next round
  if (at.value === 0 && order.value.length > 2) reorder()
  load()
  if (props.enabled) play()
}

function reshuffle() {
  if (!props.tracks.length) return
  reorder()
  load()
  if (props.enabled) play()
}

function toggle() {
  emit('update:enabled', !props.enabled)
}

function onVolume(event) {
  emit('update:volume', clampVolume(Number(event.target.value) / 100))
}

function onEnded() {
  next()
}

onMounted(() => {
  audio = new Audio()
  audio.preload = 'auto'
  audio.addEventListener('ended', onEnded)
  reorder()
  load()
  if (props.enabled) play()
})

watch(
  () => props.enabled,
  (on) => (on ? play() : stop()),
)
watch(
  () => props.volume,
  (v) => {
    if (audio) audio.volume = clampVolume(v)
  },
)
// a new list of tracks: keep the one playing if it is still there
watch(
  () => props.tracks.map((t) => t?.url).join('\n'),
  () => {
    const url = audio?.getAttribute('src')
    const keep = props.tracks.findIndex((t) => t?.url === url)
    reorder(keep >= 0 ? keep : null)
    load()
    if (props.enabled) play()
  },
)

onBeforeUnmount(() => {
  if (!audio) return
  audio.removeEventListener('ended', onEnded)
  audio.pause()
  audio.removeAttribute('src')
  audio.load()
  audio = null
})
</script>

<template>
  <div class="music" :class="{ on: enabled }">
    <button class="switch" role="switch" :aria-checked="enabled" :aria-label="enabled ? 'Turn music off' : 'Turn music on'" @click="toggle">
      <span class="knob"></span>
    </button>
    <div class="now">
      <span class="label">Music</span>
      <span class="title" :title="track?.title">
        <template v-if="!tracks.length">No music yet</template>
        <template v-else-if="blocked"><button class="link" @click="play">Click to start the music</button></template>
        <template v-else>{{ track?.title || 'Untitled' }}</template>
      </span>
    </div>
    <button class="icon" aria-label="Shuffle" title="Shuffle" :disabled="tracks.length < 2" @click="reshuffle">&#8644;</button>
    <button class="icon" aria-label="Next track" title="Next" :disabled="!tracks.length" @click="next">&#9197;&#xFE0E;</button>
    <label class="volume">
      <span class="sr-only">Volume</span>
      <input type="range" min="0" max="100" step="1" :value="level" :aria-valuetext="`${level}%`" @input="onVolume" />
    </label>
  </div>
</template>

<style scoped>
.music {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 14px;
  border: 1px solid var(--line, rgba(255, 255, 255, 0.14));
  background: rgba(10, 10, 24, 0.55);
  backdrop-filter: blur(8px);
  color: var(--text, #f2efff);
  font-size: 0.85rem;
  min-width: 0;
}
.switch {
  position: relative;
  flex: none;
  width: 38px;
  height: 22px;
  border-radius: 11px;
  border: 1px solid var(--line, rgba(255, 255, 255, 0.2));
  background: rgba(255, 255, 255, 0.12);
  cursor: pointer;
  padding: 0;
  transition: background 0.2s ease;
}
.switch .knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  transition: transform 0.2s ease;
}
.on .switch {
  background: var(--accent, #9b7dff);
}
.on .switch .knob {
  transform: translateX(16px);
}
.now {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}
.label {
  color: var(--text-dim, #b9b4d8);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.title {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.link {
  border: 0;
  padding: 0;
  background: none;
  color: var(--accent, #b9a2ff);
  cursor: pointer;
  font: inherit;
  text-decoration: underline;
}
.icon {
  flex: none;
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  color: inherit;
  font-size: 1rem;
  cursor: pointer;
}
.icon:hover:not(:disabled) {
  border-color: var(--line, rgba(255, 255, 255, 0.2));
}
.icon:disabled {
  opacity: 0.35;
  cursor: default;
}
.volume input {
  width: 90px;
  accent-color: var(--accent, #9b7dff);
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
}
</style>
