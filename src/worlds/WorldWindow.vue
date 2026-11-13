<script setup>
// A WORLD window on one of the member's other screens: the whole page is that screen's world.
// Its settings arrive before the page loads (world.rs) and again, without a reload, whenever the
// member changes them in the WORLD room. The lead window (the lowest screen switched on) plays
// the music. Its player lives here, beside the stage rather than inside it, so letting a long-covered
// world go never stops the music (Castle's own player keeps stepping aside while this window is open).
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import WorldStage from './WorldStage.vue'
import MusicPlayer from '../world2d/MusicPlayer.vue'
import { loadMine } from './mine.js'
import { allWorlds } from './registry.js'
import { studioLibrary } from '../api/castle.js'
import { kindFor, tracksFor } from '../components/music.js'

const props = defineProps({ initial: { type: Object, required: true } })

const config = ref(props.initial.config ?? {})
const lead = ref(!!props.initial.lead)
const music = reactive({ enabled: !!props.initial.music, volume: props.initial.volume ?? 0.5 })
const visible = ref(true)
// covered for ten minutes: the world is let go to free the memory, and comes back when seen
const UNLOAD_MS = 10 * 60 * 1000
const unloaded = ref(false)
let unloadTimer = 0
const mine = ref([])
const songs = ref([])
// the player starts once the member's worlds and songs are read, so its first track is not cut off
const loaded = ref(false)
// this window has no session of its own: the crystal simply lives here
const signals = reactive({ busy: false, asleep: false, sentAt: 0 })
const stops = []

// the music of the kind of world on this screen, then the companion's songs
const tracks = computed(() => tracksFor(kindFor(allWorlds(mine.value), config.value.world), songs.value, convertFileSrc))

function take(payload) {
  config.value = payload.config ?? config.value
  lead.value = !!payload.lead
  music.enabled = !!payload.music
  music.volume = payload.volume ?? music.volume
}

function seen(on) {
  visible.value = on
  clearTimeout(unloadTimer)
  if (on) unloaded.value = false
  // the music player is not part of the stage, so the lead window may let its world go too
  else unloadTimer = setTimeout(() => (unloaded.value = true), UNLOAD_MS)
}

async function readSongs() {
  try {
    songs.value = (await studioLibrary()).songs ?? []
  } catch {
    // no memory folder yet: Castle's own music only
  }
}

onMounted(async () => {
  stops.push(await listen('castle-world-config', (event) => take(event.payload)))
  stops.push(await listen('world-visible', (event) => seen(!!event.payload)))
  const [worlds] = await Promise.all([loadMine(), readSongs()])
  mine.value = worlds
  loaded.value = true
})
onBeforeUnmount(() => {
  clearTimeout(unloadTimer)
  stops.forEach((stop) => stop())
})
</script>

<template>
  <div class="world-window">
    <!-- the stage plays no music here (no `lead`): the player below does -->
    <WorldStage v-if="!unloaded" :config="config" :signals="signals" :mine="mine" :active="visible" />
    <MusicPlayer v-if="lead && loaded" v-show="false" :tracks="tracks" :enabled="music.enabled" :volume="music.volume" />
  </div>
</template>

<style scoped>
.world-window {
  position: fixed;
  inset: 0;
  background: #040817;
}
</style>
