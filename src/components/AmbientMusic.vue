<script setup>
// Castle's own music, on every tab: it plays from the moment Castle opens while MUSIC (WORLD room)
// is on, and keeps playing when Castle is minimised or hidden in the tray. It stays quiet only when
// a world window on another screen is already playing the music, or the WORLD room is switched off.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import MusicPlayer from '../world2d/MusicPlayer.vue'
import { getSettings, studioLibrary } from '../api/castle.js'
import { ambienceFor, songTitle } from '../worlds/ambience.js'

const POLL_MS = 4000
const music = ref(false)
const volume = ref(0.5)
const screensOn = ref(false)
const roomOn = ref(true)
const songs = ref([])
let timer = 0
let liveUntil = 0 // the World tab's controls just changed things: the saved copy may still be old

const tracks = computed(() => [...ambienceFor('2d'), ...songs.value])
const enabled = computed(() => music.value && roomOn.value && !screensOn.value)

async function read() {
  try {
    const view = await getSettings()
    const s = view.settings ?? view
    const fresh = Date.now() > liveUntil
    if (fresh) music.value = !!s.world?.music
    if (fresh) volume.value = s.world?.volume ?? 0.5
    screensOn.value = !!s.world?.screens?.some((screen) => screen.on) && s.rooms?.world !== false
    roomOn.value = s.rooms?.world !== false
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
