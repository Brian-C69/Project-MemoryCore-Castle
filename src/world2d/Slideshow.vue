<script setup>
// The 2D WORLD: the member's pictures one after another, crossfading, each drifting in a slow
// Ken Burns pan and zoom. Prev / Pause / Next show when the mouse moves and fade away again.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { kenBurns, nextIndex, shuffle, transformCss } from './slideshow.js'

const props = defineProps({
  // picture URLs
  pictures: { type: Array, default: () => [] },
  // seconds each picture stays
  interval: { type: Number, default: 12 },
  // show them in a shuffled order (a new order each time the room opens)
  shuffled: { type: Boolean, default: true },
})

const FADE_S = 1.6
const CONTROLS_HIDE_MS = 2500

const seed = Date.now() % 100000
const order = computed(() => (props.shuffled ? shuffle(props.pictures, seed) : [...props.pictures]))
const index = ref(0)
const count = ref(0) // every slide shown gets a new number, so its animation starts fresh
const paused = ref(false)
const controls = ref(false)
const reduceMotion = typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

let timer = 0
let hideTimer = 0

const seconds = computed(() => Math.max(3, Number(props.interval) || 12))

const slide = computed(() => {
  const url = order.value[index.value]
  if (!url) return null
  const move = kenBurns(count.value, seed)
  return {
    key: count.value,
    url,
    style: {
      '--from': reduceMotion ? 'none' : transformCss(move.from),
      '--to': reduceMotion ? 'none' : transformCss(move.to),
      '--duration': `${seconds.value + FADE_S * 2}s`,
    },
  }
})

function schedule() {
  clearTimeout(timer)
  if (paused.value || order.value.length < 2) return
  timer = setTimeout(() => go(1), seconds.value * 1000)
}

function preload(i) {
  const url = order.value[i]
  if (url && typeof Image !== 'undefined') new Image().src = url
}

function go(step) {
  if (order.value.length < 1) return
  index.value = nextIndex(index.value, order.value.length, step)
  count.value++
  preload(nextIndex(index.value, order.value.length, 1))
  schedule()
}

function togglePause() {
  paused.value = !paused.value
  schedule()
}

function wake() {
  controls.value = true
  clearTimeout(hideTimer)
  hideTimer = setTimeout(() => (controls.value = false), CONTROLS_HIDE_MS)
}

// a new list of pictures (not just a new array holding the same ones) starts over
watch(() => order.value.join('|'), () => {
  index.value = 0
  count.value++
  preload(1)
  schedule()
})
watch(seconds, schedule)

onMounted(() => {
  preload(1)
  schedule()
})
onBeforeUnmount(() => {
  clearTimeout(timer)
  clearTimeout(hideTimer)
})
</script>

<template>
  <div class="slideshow" :class="{ paused, 'show-controls': controls }" @mousemove="wake" @mouseleave="controls = false">
    <TransitionGroup name="crossfade" tag="div" class="frames">
      <div v-if="slide" :key="slide.key" class="frame">
        <img :src="slide.url" alt="" draggable="false" :style="slide.style" />
      </div>
    </TransitionGroup>

    <p v-if="!order.length" class="empty">No pictures yet.</p>

    <div v-if="order.length > 1" class="controls" @mousemove.stop="wake">
      <button aria-label="Previous picture" @click="go(-1)">&#8249;</button>
      <button :aria-label="paused ? 'Play' : 'Pause'" class="pause" @click="togglePause">
        <span v-if="paused">&#9654;</span>
        <span v-else>&#10074;&#10074;</span>
      </button>
      <button aria-label="Next picture" @click="go(1)">&#8250;</button>
    </div>
  </div>
</template>

<style scoped>
.slideshow {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #05060f;
}
.frames,
.frame {
  position: absolute;
  inset: 0;
}
.frame img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transform: var(--from);
  animation: ken-burns var(--duration) linear forwards;
  will-change: transform;
  user-select: none;
}
.paused .frame img {
  animation-play-state: paused;
}
@keyframes ken-burns {
  from {
    transform: var(--from);
  }
  to {
    transform: var(--to);
  }
}

.crossfade-enter-active,
.crossfade-leave-active {
  transition: opacity 1.6s ease;
}
.crossfade-enter-from,
.crossfade-leave-to {
  opacity: 0;
}

.empty {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  margin: 0;
  color: var(--text-dim, #b9b4d8);
}

.controls {
  position: absolute;
  left: 50%;
  bottom: 28px;
  transform: translateX(-50%);
  display: flex;
  gap: 10px;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.4s ease;
}
.show-controls .controls,
.paused .controls {
  opacity: 1;
  pointer-events: auto;
}
.controls button {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.25);
  background: rgba(10, 10, 24, 0.55);
  color: #fff;
  font-size: 1.4rem;
  line-height: 1;
  cursor: pointer;
  backdrop-filter: blur(6px);
}
.controls button.pause {
  font-size: 0.85rem;
  letter-spacing: -2px;
}
.controls button:hover {
  border-color: var(--accent, #b9a2ff);
}
</style>
