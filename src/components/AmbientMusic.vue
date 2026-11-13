<script setup>
// Castle's own music, on every tab: it plays from the moment Castle opens while MUSIC (WORLD room)
// is on, and keeps playing when Castle is minimised or hidden in the tray. A world window on another
// screen plays its own music (its kind's: Castle Theme for 2D worlds, Floating Isles for 3D ones), so
// this player steps aside exactly while such a window is open - never two tunes at once - and takes
// over again the moment the last one closes. It is also quiet when the WORLD room is switched off.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import MusicPlayer from '../world2d/MusicPlayer.vue'
import { getSettings, studioLibrary, worldScreens } from '../api/castle.js'
import { ambienceFor, songTitle } from '../worlds/ambience.js'
import { shouldPlay } from './ambientmusic.js'

const POLL_MS = 4000
// Much less often than settings: a song dropped into the library while Castle is already running
// should eventually show up, but re-reading the whole library every 4s would be wasteful.
const SONGS_POLL_MS = 60000
const music = ref(false)
const volume = ref(0.5)
const screensOn = ref(false)
const roomOn = ref(true)
const songs = ref([])
let timer = 0
let songsTimer = 0
let liveUntil = 0 // the World tab's controls just changed things: the saved copy may still be old

const tracks = computed(() => [...ambienceFor('2d'), ...songs.value])
const enabled = computed(() => shouldPlay({ music: music.value, roomOn: roomOn.value, screensOn: screensOn.value }))

async function read() {
  try {
    const view = await getSettings()
    const s = view.settings ?? view
    const fresh = Date.now() > liveUntil
    if (fresh) music.value = !!s.world?.music
    if (fresh) volume.value = s.world?.volume ?? 0.5
    roomOn.value = s.rooms?.world !== false
    // a window that is really open (a screen saved as on but unplugged has none, so this player stays)
    screensOn.value = roomOn.value && (await worldScreens()).some((screen) => screen.open && !screen.isMain)
  } catch {
    // outside Castle: no music
  }
}

async function readSongs() {
  try {
    const lib = await studioLibrary()
    songs.value = (lib.songs ?? []).map((m) => ({ title: songTitle(m.name), url: convertFileSrc(m.path) }))
  } catch {
    // no memory folder yet: Castle's own music only
  }
}

// The WORLD room's MUSIC controls: follow them at once, without waiting for the saved copy.
function live(event) {
  const patch = event.detail ?? {}
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
  // a song dropped into the library while Castle keeps running should eventually join the mix
  songsTimer = setInterval(readSongs, SONGS_POLL_MS)
})
onBeforeUnmount(() => {
  clearInterval(timer)
  clearInterval(songsTimer)
  window.removeEventListener('castle-music', live)
})
</script>

<template>
  <MusicPlayer v-show="false" :tracks="tracks" :enabled="enabled" :volume="volume" />
</template>
