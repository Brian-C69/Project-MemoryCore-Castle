<script setup>
// The music switch and its volume, in the top bar beside Settings: the music plays on every tab, so
// its controls are on every tab. It saves only the music (never the rest of the settings) and tells
// the player at once, so a flip or a slide is heard immediately. It reads the saved choice again
// every few seconds, like the player does, so it never shows an old state; with the WORLD room off
// the choice is still saved, and the music waits for the room to be on again.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { getSettings, worldMusic } from '../api/castle.js'
import { musicFrom } from './music.js'

const POLL_MS = 4000
const ready = ref(false)
const on = ref(false)
const volume = ref(0.5)
const roomOn = ref(true)
let timer = 0
let liveUntil = 0 // just changed here: the saved copy may still be old

const title = computed(() => {
  if (!roomOn.value) return on.value ? 'Music on, but quiet while the WORLD room is off: click to turn it off' : 'Music off (the WORLD room is off too): click to turn it on'
  return on.value ? 'Music on: click to turn it off' : 'Music off: click to turn it on'
})

async function read() {
  try {
    const saved = musicFrom((await getSettings()).settings)
    roomOn.value = saved.roomOn
    if (Date.now() > liveUntil) {
      on.value = saved.music
      volume.value = saved.volume
    }
    ready.value = true
  } catch {
    // outside Castle: no music controls
  }
}

const tell = (patch) => window.dispatchEvent(new CustomEvent('castle-music', { detail: patch }))
async function save(patch) {
  liveUntil = Date.now() + 5000
  try {
    await worldMusic(patch)
  } catch {
    // not saved: show what is really saved again
    liveUntil = 0
    read()
  }
}

function flip() {
  on.value = !on.value
  tell({ music: on.value })
  save({ music: on.value })
}
function slide(event) {
  volume.value = Number(event.target.value)
  liveUntil = Date.now() + 5000
  tell({ volume: volume.value })
}
function release(event) {
  save({ volume: Number(event.target.value) })
}

onMounted(async () => {
  await read()
  timer = setInterval(read, POLL_MS)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <div v-if="ready" class="top-music" :class="{ on, quiet: !roomOn }">
    <span class="note" aria-hidden="true">&#9835;</span>
    <button class="flip" :class="{ on }" role="switch" :aria-checked="on" aria-label="Music" :title="title" @click="flip">
      <span class="knob"></span>
    </button>
    <input type="range" min="0" max="1" step="0.05" :value="volume" aria-label="Music volume" title="Music volume" @input="slide" @change="release" />
  </div>
</template>

<style scoped>
.top-music {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-right: 8px;
  color: var(--text-dim);
}
.top-music.on .note {
  color: var(--accent);
}
.top-music.quiet .note {
  color: var(--text-dim);
  opacity: 0.6;
}
.note {
  font-size: 15px;
}
.flip {
  position: relative;
  flex: none;
  width: 40px;
  height: 22px;
  padding: 0;
  border-radius: 11px;
  border: 1px solid var(--line, rgba(255, 255, 255, 0.2));
  background: rgba(255, 255, 255, 0.12);
  cursor: pointer;
  transition: background 0.2s ease;
}
.flip .knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  transition: transform 0.2s ease;
}
.flip.on {
  background: var(--accent, #9b7dff);
}
.flip.on .knob {
  transform: translateX(18px);
}
input[type='range'] {
  width: 84px;
  accent-color: var(--accent, #9b7dff);
}
.flip:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
