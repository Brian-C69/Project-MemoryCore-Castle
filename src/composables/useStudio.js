// STUDIO's shelves and powers, kept for the whole run. Castle reads them once at the start (App.vue),
// behind the start-up screen, so opening STUDIO shows its gallery at once; the room then reads them
// again quietly and swaps in anything new. Nothing here blocks: a read runs off the page's thread, and
// a read already under way is shared rather than started twice. The pictures' small copies come from
// useThumbs.js.
import { ref } from 'vue'
import { studioLibrary, studioPowers, voiceStudioStatus } from '../api/castle.js'
import { ensureThumbs, thumbUrls } from './useThumbs.js'

export const EMPTY_LIBRARY = { images: [], videos: [], songs: [], voices: [], saved: [], backups: [] }

export const studioLib = ref({ ...EMPTY_LIBRARY })
export const studioPowersNow = ref({})
export const studioVoiceUp = ref(false)
// true once the Picture power's real state (it asks Codex itself, which can take a moment) has been
// read at least once - until then its pill and button read "Checking..." rather than "missing"
export const studioPowersLoaded = ref(false)
// a picture's small copy, by the picture's path (shared by every room)
export const studioThumbUrls = thumbUrls

let thumbJob = Promise.resolve()
let onThumbProgress = null

function buildThumbs() {
  const paths = [...studioLib.value.images, ...studioLib.value.backups].map((m) => m.path)
  thumbJob = ensureThumbs(paths, (done, total) => onThumbProgress?.(done, total))
  return thumbJob
}

// Everything STUDIO needs to open full: the shelves, the powers and a small copy of every picture,
// made and decoded. Castle's start-up screen waits on this (and shows its progress), so the room is
// simply there when it opens; a gallery too large to finish in time carries on in the background.
export async function prepareStudio(progress) {
  onThumbProgress = progress ?? null
  try {
    await refreshStudio()
    await thumbJob
  } finally {
    onThumbProgress = null
  }
}

let reading = null

export function refreshStudio() {
  if (reading) return reading
  reading = (async () => {
    try {
      // the gallery first: it is what the room shows, and it does not wait on Codex's check
      studioLib.value = await studioLibrary(true)
      buildThumbs() // the powers below do not wait on it; prepareStudio() does
      studioVoiceUp.value = await voiceStudioStatus().catch(() => false)
      studioPowersNow.value = await studioPowers()
      studioPowersLoaded.value = true
    } catch {
      // outside Castle's window
    } finally {
      reading = null
    }
  })()
  return reading
}
