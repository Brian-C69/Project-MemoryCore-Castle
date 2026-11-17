// APPEARANCE's look (the standard picture, today's try-on, the wardrobe and the hairstyles) and how
// the next picture would be drawn, kept for the whole run. Castle reads them at start-up behind the
// start-up screen, with a small copy of each picture the closet shows, so the room opens full; the
// room then reads them again quietly and swaps in anything new.
import { ref } from 'vue'
import { appearanceLook, studioPowers } from '../api/castle.js'
import { ensureThumbs } from './useThumbs.js'

export const appearanceNow = ref(null)
// how the next picture would be drawn: "ready", "builtin" or "prompt" (see studio.rs)
export const appearanceImage = ref('')

// The pictures a closet card shows: a turnaround sheet, or the reference it was drawn from.
export function closetPictures(look) {
  const items = [...(look?.wardrobe ?? []), ...(look?.hairstyles ?? [])]
  return items.map((i) => i.sheet || i.reference).filter(Boolean)
}

export async function prepareAppearance(progress) {
  try {
    appearanceNow.value = await appearanceLook()
    appearanceImage.value = (await studioPowers()).image
    await ensureThumbs(closetPictures(appearanceNow.value), progress)
  } catch {
    // outside Castle's window, or no memory folder yet: the room says so when it opens
  }
}
