<script setup>
// Castle's own music, on every tab: it plays from the moment Castle opens while MUSIC (WORLD room)
// is on, and keeps playing when Castle is minimised or hidden in the tray. A world window on another
// screen plays its own music (its kind's: Castle Theme for 2D worlds, Floating Isles for 3D ones), so
// this player steps aside exactly while such a window is open - never two tunes at once - and takes
// over again the moment the last one closes. "Open" means a window that really exists (world.rs makes
// one only for a connected screen that is not Castle's own), never just a screen saved as on; the
// lead world window keeps its player even when its world is let go after being covered a long time.
// It is also quiet when the WORLD room is switched off (the top-bar switch still saves the choice).
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import MusicPlayer from '../world2d/MusicPlayer.vue'
import { getSettings, studioLibrary, worldScreens } from '../api/castle.js'
import { musicFrom, tracksFor, worldWindowOpen } from './music.js'

const POLL_MS = 4000
const music = ref(false)
const volume = ref(0.5)
const screensOn = ref(false)
const roomOn = ref(true)
const stageOwns = ref(false) // the story stage is playing its own adventure's music
const songs = ref([])
let timer = 0
let liveUntil = 0 // the World tab's controls just changed things: the saved copy may still be old

const tracks = computed(() => tracksFor('2d', songs.value, convertFileSrc))
const enabled = computed(() => music.value && roomOn.value && !screensOn.value && !stageOwns.value)

async function read() {
  try {
    const view = await getSettings()
    const saved = musicFrom(view.settings ?? view)
    if (Date.now() > liveUntil) {
      music.value = saved.music
      volume.value = saved.volume
    }
    roomOn.value = saved.roomOn
    // a window that is really open (a screen saved as on but unplugged has none, so this player stays)
    screensOn.value = roomOn.value && worldWindowOpen(await worldScreens())
  } catch {
    // outside Castle: no music
  }
}

async function readSongs() {
  try {
    const lib = await studioLibrary()
    songs.value = lib.songs ?? []
  } catch {
    // no memory folder yet: Castle's own music only
  }
}

// The WORLD room's MUSIC controls: follow them at once, without waiting for the saved copy.
function live(event) {
  const patch = event.detail ?? {}
  if (typeof patch.stage === 'boolean') stageOwns.value = patch.stage
  if (patch.screens) read() // the screens just changed: hand over at once, not at the next check
  if (typeof patch.music !== 'boolean' && typeof patch.volume !== 'number') return
  liveUntil = Date.now() + 5000
  if (typeof patch.music === 'boolean') music.value = patch.music
  if (typeof patch.volume === 'number') volume.value = patch.volume
}

onMounted(async () => {
  window.addEventListener('castle-music', live)
  await Promise.all([read(), readSongs()])
  timer = setInterval(read, POLL_MS)
})
onBeforeUnmount(() => {
  clearInterval(timer)
  window.removeEventListener('castle-music', live)
})
</script>

<template>
  <MusicPlayer v-show="false" :tracks="tracks" :enabled="enabled" :volume="volume" />
</template>
