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

function warm(url) {
  const image = new Image()
  image.decoding = 'async'
  image.src = url
}

async function buildThumbs() {
  if (thumbing) {
    thumbAgain = true
    return
  }
  thumbing = true
  try {
    do {
      thumbAgain = false
      const paths = [...studioLib.value.images, ...studioLib.value.backups].map((m) => m.path)
      // the core makes a few at a time and says how many are left, so the window is never held
      for (let round = 0; round < 40; round += 1) {
        const { thumbs, pending } = await studioThumbs(paths)
        const next = { ...studioThumbUrls.value }
        for (const t of thumbs) {
          if (!next[t.path]) {
            next[t.path] = convertFileSrc(t.thumb)
            warm(next[t.path])
          }
        }
        studioThumbUrls.value = next
        if (!pending) break
      }
    } while (thumbAgain)
  } catch {
    // outside Castle's window, or a copy could not be made: the pictures show as they are
  } finally {
    thumbing = false
  }
}

let reading = null

export function refreshStudio() {
  if (reading) return reading
  reading = (async () => {
    try {
      // the gallery first: it is what the room shows, and it does not wait on Codex's check
      studioLib.value = await studioLibrary()
      buildThumbs() // in the background: nothing below waits on it
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
