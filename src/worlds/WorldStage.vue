<script setup>
// One screen's world, whatever kind it is: a world made of model pieces, the built-in isles, a
// slideshow, or the member's own pictures. A model world that is still waiting for its pieces
// shows the built-in isles with a small line saying so. Used by the world windows and by the
// WORLD room's preview.
import { computed, defineAsyncComponent, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import Slideshow from '../world2d/Slideshow.vue'
import MusicPlayer from '../world2d/MusicPlayer.vue'
import { cliQuota, studioLibrary } from '../api/castle.js'
import { DEFAULT_WORLD, allWorlds, builtIn2d, findWorld } from './registry.js'
import { readinessOf } from './readiness.js'
import { ambienceFor, songTitle } from './ambience.js'

// three.js is large: it loads only when a 3D world is shown
const World3D = defineAsyncComponent(() => import('../world3d/World3D.vue'))
const DeepSpace = defineAsyncComponent(() => import('../world3d/DeepSpace.vue'))

const props = defineProps({
  // one WorldScreen: { world, camera, holdMin, holdMax, glide, shots, seconds, shuffle }
  config: { type: Object, required: true },
  // { busy, asleep, sentAt }
  signals: { type: Object, default: () => ({ busy: false, asleep: false, sentAt: 0 }) },
  // the member's own created worlds (entries from worlds_mine)
  mine: { type: Array, default: () => [] },
  // false while covered or hidden: the GPU rests
  active: { type: Boolean, default: true },
  // this stage plays the music
  lead: { type: Boolean, default: false },
  music: { type: Object, default: () => ({ enabled: false, volume: 0.5 }) },
})

const lib = ref({ images: [], songs: [] })
const state = ref(null) // readiness of a model world
// the weekly limit left on the member's CLI, 0..1 (null = unknown) - the crystal shines with it;
// a dev page may try a level with ?energy=0.13
const devEnergy = import.meta.env.DEV ? new URLSearchParams(location.search).get('energy') : null
const energy = ref(devEnergy !== null ? Number(devEnergy) : null)
let quotaTimer = 0
async function readQuota() {
  if (devEnergy !== null) return
  try {
    const q = await cliQuota()
    energy.value = q ? q.leftPercent / 100 : null
  } catch {
    energy.value = null
  }
}

const world = computed(() => findWorld(allWorlds(props.mine), props.config.world) ?? findWorld(allWorlds(), DEFAULT_WORLD))
const showing = computed(() => {
  const w = world.value
  if (w.engine === 'glb' && !state.value?.ready) return findWorld(allWorlds(), DEFAULT_WORLD)
  return w
})
const waiting = computed(() => (world.value.engine === 'glb' && state.value && !state.value.ready ? `${world.value.title} is waiting for its pieces (${state.value.have}/${state.value.total}) - the built-in isles meanwhile` : ''))

const pictures = computed(() => {
  const w = showing.value
  if (w.engine === 'slideshow') return w.pictures
  const made = lib.value.images.map((m) => convertFileSrc(m.path))
  return made.length ? made : builtIn2d()[0]?.pictures ?? []
})
const camera = computed(() => ({
  camera: props.config.camera,
  holdMin: props.config.holdMin,
  holdMax: props.config.holdMax,
  glide: props.config.glide,
  shots: props.config.shots,
}))
const tracks = computed(() => [
  ...ambienceFor(showing.value.kind),
  ...lib.value.songs.map((m) => ({ title: songTitle(m.name), url: convertFileSrc(m.path) })),
])

async function check() {
  state.value = world.value.engine === 'glb' ? await readinessOf(world.value) : null
}

onBeforeUnmount(() => clearInterval(quotaTimer))

onMounted(async () => {
  check()
  readQuota()
  quotaTimer = setInterval(readQuota, 5 * 60 * 1000)
  try {
    lib.value = await studioLibrary()
  } catch {
    // outside Castle, or no memory folder yet
  }
})
watch(() => props.config.world, check)
watch(() => props.mine, check, { deep: true })
</script>

<template>
  <div class="stage">
    <DeepSpace v-if="showing.engine === 'space'" :key="showing.id" :active="active" />
    <World3D
      v-else-if="showing.kind === '3d'"
      :key="showing.id"
      :world="showing.engine === 'glb' ? showing : null"
      :signals="signals"
      :active="active"
      :camera="camera"
      :energy="energy"
    />
    <Slideshow v-else :pictures="pictures" :interval="config.seconds || 15" :shuffled="config.shuffle !== false" />
    <p v-if="waiting" class="waiting">{{ waiting }}</p>
    <MusicPlayer v-if="lead" v-show="false" :tracks="tracks" :enabled="music.enabled && active" :volume="music.volume" />
  </div>
</template>

<style scoped>
.stage {
  position: absolute;
  inset: 0;
  background: #040817;
}
.waiting {
  position: absolute;
  left: 50%;
  top: 18px; /* the bottom of a screen may sit under its taskbar */
  margin: 0;
  padding: 6px 14px;
  border-radius: 10px;
  background: rgba(4, 8, 23, 0.6);
  color: #d8d4f2;
  font-size: 12px;
  transform: translateX(-50%);
  pointer-events: none;
}
</style>
