// Small copies of pictures, for any room. The core makes each once (src-tauri/src/thumbs.rs), keeps
// it in Castle's settings folder and makes it again only when the picture changes; this keeps their
// addresses for the whole run and loads and decodes each one so it is ready when a room draws it.
// A room asks for the pictures it will show (`ensureThumbs`) at start-up and draws
// `thumbUrls[path] ?? the real picture`, so a picture without a small copy yet shows as it always did.
import { ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { studioThumbs } from '../api/castle.js'

// picture path -> the small copy's address
export const thumbUrls = ref({})

// Loads and decodes a small copy now. Resolves either way.
function warm(url) {
  const image = new Image()
  image.decoding = 'async'
  image.src = url
  return image.decode().catch(() => {})
}

// One run at a time: two rooms asking together wait their turn, so the core is not asked twice for the
// same pictures and the window is never held by two runs at once.
let line = Promise.resolve()

// Makes sure the pictures named have small copies (a few at a time, the core says how many are
// left) and that each is loaded. `progress(done, total)` is told as it goes. Never throws.
export function ensureThumbs(paths, progress) {
  const wanted = [...new Set(paths.filter(Boolean))]
  line = line.then(() => run(wanted, progress)).catch(() => {})
  return line
}

async function run(paths, progress) {
  if (!paths.length) return
  for (let round = 0; round < 40; round += 1) {
    const { thumbs, pending } = await studioThumbs(paths)
    const next = { ...thumbUrls.value }
    const warming = []
    for (const t of thumbs) {
      const url = convertFileSrc(t.thumb)
      // a picture drawn again gets a new small copy, so a different address replaces the old one
      if (next[t.path] !== url) {
        next[t.path] = url
        warming.push(warm(url))
      }
    }
    thumbUrls.value = next
    await Promise.all(warming)
    progress?.(thumbs.length, paths.length)
    if (!pending) return
  }
}
