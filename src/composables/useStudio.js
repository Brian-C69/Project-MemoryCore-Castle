// STUDIO's shelves and powers, kept for the whole run. Castle reads them once at the start (App.vue),
// in the background, so opening STUDIO shows its gallery at once; the room then reads them again
// quietly and swaps in anything new. Nothing here blocks: a read runs off the page's thread, and a
// read already under way is shared rather than started twice.
import { ref } from 'vue'
import { studioLibrary, studioPowers, voiceStudioStatus } from '../api/castle.js'

export const EMPTY_LIBRARY = { images: [], videos: [], songs: [], voices: [], saved: [], backups: [] }

export const studioLib = ref({ ...EMPTY_LIBRARY })
export const studioPowersNow = ref({})
export const studioVoiceUp = ref(false)
// true once the Picture power's real state (it asks Codex itself, which can take a moment) has been
// read at least once - until then its pill and button read "Checking..." rather than "missing"
export const studioPowersLoaded = ref(false)

let reading = null

export function refreshStudio() {
  if (reading) return reading
  reading = (async () => {
    try {
      // the gallery first: it is what the room shows, and it does not wait on Codex's check
      studioLib.value = await studioLibrary()
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
