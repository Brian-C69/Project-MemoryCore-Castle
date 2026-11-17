// STUDIO's shelves and powers, kept for the whole run. Castle reads them once at the start (App.vue),
// in the background, so opening STUDIO shows its gallery at once; the room then reads them again
// quietly and swaps in anything new. Nothing here blocks: a read runs off the page's thread, and a
// read already under way is shared rather than started twice.
import { ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { studioLibrary, studioPowers, studioThumbs, voiceStudioStatus } from '../api/castle.js'

export const EMPTY_LIBRARY = { images: [], videos: [], songs: [], voices: [], saved: [], backups: [] }

export const studioLib = ref({ ...EMPTY_LIBRARY })
export const studioPowersNow = ref({})
export const studioVoiceUp = ref(false)
// true once the Picture power's real state (it asks Codex itself, which can take a moment) has been
// read at least once - until then its pill and button read "Checking..." rather than "missing"
export const studioPowersLoaded = ref(false)

// A picture's small copy (made once by the core and kept), by the picture's path. The gallery shows
// these; a picture without one yet shows as it is. They are made and warmed in the background at the
// start, so opening STUDIO does not wait for a gallery of full-size drawings to be read.
export const studioThumbUrls = ref({})
let thumbing = false
let thumbAgain = false

// Loads and decodes a small copy now, so it is ready when the gallery is drawn. Resolves either way.
function warm(url) {
  const image = new Image()
  image.decoding = 'async'
  image.src = url
  return image.decode().catch(() => {})
}

let thumbJob = Promise.resolve()
let onThumbProgress = null

async function buildThumbsNow() {
  thumbing = true
  try {
    do {
      thumbAgain = false
      const paths = [...studioLib.value.images, ...studioLib.value.backups].map((m) => m.path)
      // the core makes a few at a time and says how many are left, so the window is never held
      for (let round = 0; round < 40; round += 1) {
        const { thumbs, pending } = await studioThumbs(paths)
        const next = { ...studioThumbUrls.value }
        const warming = []
        for (const t of thumbs) {
          if (!next[t.path]) {
            next[t.path] = convertFileSrc(t.thumb)
            warming.push(warm(next[t.path]))
          }
        }
        studioThumbUrls.value = next
        await Promise.all(warming)
        onThumbProgress?.(thumbs.length, paths.length)
        if (!pending) break
      }
    } while (thumbAgain)
  } catch {
    // outside Castle's window, or a copy could not be made: the pictures show as they are
  } finally {
    thumbing = false
  }
}

function buildThumbs() {
  if (thumbing) {
    thumbAgain = true // the running job goes round again for what changed
    return thumbJob
  }
  thumbJob = buildThumbsNow()
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
      studioLib.value = await studioLibrary()
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
