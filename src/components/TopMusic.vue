<script setup>
// The music switch and its volume, in the top bar beside Settings: the music plays on every tab, so
// its controls are on every tab. It saves only the music (never the rest of the settings) and tells
// the player at once, so a flip or a slide is heard immediately.
import { onMounted, ref } from 'vue'
import { getSettings, worldMusic } from '../api/castle.js'

const ready = ref(false)
const on = ref(false)
const volume = ref(0.5)

const tell = (patch) => window.dispatchEvent(new CustomEvent('castle-music', { detail: patch }))
async function save(patch) {
  try {
    await worldMusic(patch)
  } catch {
    // outside Castle: nothing to save
  }
}

function flip() {
  on.value = !on.value
  tell({ music: on.value })
  save({ music: on.value })
}
function slide(event) {
  volume.value = Number(event.target.value)
  tell({ volume: volume.value })
}
function release(event) {
  save({ volume: Number(event.target.value) })
}

onMounted(async () => {
  try {
    const world = (await getSettings()).settings.world
    on.value = !!world.music
    volume.value = world.volume ?? 0.5
    ready.value = true
  } catch {
    // outside Castle: no music controls
  }
})
</script>

<template>
  <div v-if="ready" class="top-music" :class="{ on }">
    <span class="note" aria-hidden="true">&#9835;</span>
    <button class="flip" :class="{ on }" role="switch" :aria-checked="on" aria-label="Music" :title="on ? 'Music on: click to turn it off' : 'Music off: click to turn it on'" @click="flip">
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
