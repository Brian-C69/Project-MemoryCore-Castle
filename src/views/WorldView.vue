<script setup>
// WORLD: the companion's world in 3D (floating isles, the crystal) or 2D (a slideshow of what the
// companion made, with music), here and on the member's other screens.
import { computed, defineAsyncComponent, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import Slideshow from '../world2d/Slideshow.vue'
import MusicPlayer from '../world2d/MusicPlayer.vue'
import { worldSignals } from '../world3d/signals.js'
import { getSettings, saveSettings, studioLibrary, worldClose, worldOpen, worldScreens } from '../api/castle.js'

// three.js is large: it loads only when the 3D world is first shown
const World3D = defineAsyncComponent(() => import('../world3d/World3D.vue'))

const props = defineProps({
  active: { type: Boolean, default: true },
  wallpaper: { type: String, default: '' },
})

const settings = ref(null)
const lib = ref({ images: [], songs: [] })
const screens = ref([])
const problem = ref('')

const mode = computed(() => settings.value?.world?.mode ?? '3d')
const pictures = computed(() => {
  const made = lib.value.images.map((m) => convertFileSrc(m.path))
  return props.wallpaper ? [props.wallpaper, ...made] : made
})
const tracks = computed(() => lib.value.songs.map((m) => ({ title: m.name.replace(/\.[^.]+$/, '').replace(/^\d+-/, '').replace(/-/g, ' '), url: convertFileSrc(m.path) })))
const others = computed(() => screens.value.filter((s) => !s.isMain))

async function load() {
  try {
    settings.value = (await getSettings()).settings
    lib.value = await studioLibrary()
    screens.value = await worldScreens()
  } catch {
    // outside Castle's window
  }
}

async function saveWorld(patch) {
  const next = { ...settings.value, world: { ...settings.value.world, ...patch } }
  try {
    settings.value = (await saveSettings(next)).settings
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function toggleScreen(screen) {
  problem.value = ''
  try {
    if (screen.open) await worldClose(screen.index)
    else await worldOpen(screen.index, mode.value)
  } catch (reason) {
    problem.value = String(reason)
  }
  screens.value = await worldScreens()
}

// an open world on another screen follows the mode
watch(mode, async (m) => {
  for (const s of screens.value.filter((x) => x.open)) await worldOpen(s.index, m).catch(() => {})
})

onMounted(load)
watch(() => props.active, (on) => on && load())
</script>

<template>
  <div class="world-room">
    <div class="stage">
      <World3D v-if="mode === '3d'" :signals="worldSignals" :active="active" />
      <Slideshow v-else :pictures="pictures" :interval="12" />
    </div>

    <div class="controls widget">
      <div class="row">
        <button :class="{ primary: mode === '3d' }" @click="saveWorld({ mode: '3d' })">3D isles</button>
        <button :class="{ primary: mode === '2d' }" @click="saveWorld({ mode: '2d' })">2D slideshow</button>
      </div>
      <MusicPlayer
        v-if="mode === '2d' && settings"
        :tracks="tracks"
        :enabled="settings.world.music && active"
        :volume="settings.world.volume"
        @update:enabled="saveWorld({ music: $event })"
        @update:volume="saveWorld({ volume: $event })"
      />
      <p v-if="mode === '2d' && !tracks.length" class="hint">Songs your companion makes play here.</p>
      <div v-if="others.length" class="row screens">
        <span class="hint">Other screens:</span>
        <button v-for="s in others" :key="s.index" :class="{ primary: s.open }" @click="toggleScreen(s)">
          {{ s.open ? `Close on ${s.name}` : `Show on ${s.name}` }}
        </button>
      </div>
      <p v-if="problem" class="problem">{{ problem }}</p>
    </div>
  </div>
</template>

<style scoped>
.world-room {
  position: absolute;
  inset: 0;
}
.stage {
  position: absolute;
  inset: 0;
}
.controls {
  position: absolute;
  left: 50%;
  bottom: 22px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: center;
  padding: 10px 16px;
  transform: translateX(-50%);
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}
.problem {
  margin: 0;
  font-size: 12px;
  color: var(--warn);
}
button {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
</style>
