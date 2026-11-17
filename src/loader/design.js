// A theme's say over the start-up screen. In theme.json, an optional block:
//
//   "loader": {
//     "image": "loader.webp",          a 16:9 picture in the theme's own folder
//     "brand": "SAKURA NIGHT",         the small title above the line (up to 40 characters)
//     "quips": ["Counting petals."],   funny lines, up to 60 of up to 120 characters
//     "quipsMode": "add",              "add" to Castle's own lines (default) or "replace" them
//     "accent": "#ff9ccf",             the bar and title colour (default: the theme's --accent)
//     "position": "bottom"             where the words sit: "bottom" (default), "center" or "top"
//   }
//
// Everything is cleaned here and nothing else is trusted: a theme is data only. The start-up screen
// shows before the theme is chosen, so the design is remembered from the last start (loader.js).

export const POSITIONS = ['bottom', 'center', 'top']
export const MODES = ['add', 'replace']
export const MAX_BRAND = 40

// A colour a theme may give: #hex, rgb()/rgba() or hsl()/hsla() with plain numbers, nothing else.
// A value that could carry a url(), a second declaration or a comment is refused.
export function safeColour(value) {
  if (typeof value !== 'string') return null
  const v = value.trim()
  if (/^#[0-9a-f]{3,8}$/i.test(v)) return v
  if (/^(rgb|rgba|hsl|hsla)\(\s*[0-9.%\s,/-]+\)$/i.test(v)) return v
  return null
}

function cleanLines(raw) {
  if (!Array.isArray(raw)) return []
  return raw
    .filter((q) => typeof q === 'string')
    .map((q) => q.trim())
    .filter((q) => q && q.length <= 120)
    .slice(0, 60)
}

// The design a theme asks for, cleaned, or null when it asks for nothing. `imageUrl` is the picture
// the theme loader already read from the theme's own folder (never taken from the JSON).
export function loaderDesign(theme) {
  const block = theme?.loader
  if (!block || typeof block !== 'object') return null
  const brand = typeof block.brand === 'string' ? block.brand.trim().slice(0, MAX_BRAND) : ''
  const design = {
    brand,
    quips: cleanLines(block.quips),
    quipsMode: MODES.includes(block.quipsMode) ? block.quipsMode : 'add',
    accent: safeColour(block.accent),
    position: POSITIONS.includes(block.position) ? block.position : 'bottom',
    imageUrl: typeof theme.loaderUrl === 'string' && theme.loaderUrl ? theme.loaderUrl : null,
  }
  const empty = !design.brand && !design.quips.length && !design.accent && design.position === 'bottom' && !design.imageUrl
  return empty ? null : design
}

// The lines to turn through: the build's own, with the theme's added or put in their place.
export function linesFor(base, design) {
  if (!design?.quips?.length) return base
  return design.quipsMode === 'replace' ? design.quips : [...base, ...design.quips]
}

// A remembered design is only picture-sized when it is small enough for the browser's storage.
export const MAX_STORED_IMAGE = 3 * 1024 * 1024

export function forStorage(design, accentNow) {
  const accent = design?.accent ?? safeColour(accentNow ?? '')
  const keepImage = design?.imageUrl && design.imageUrl.length <= MAX_STORED_IMAGE
  return { ...(design ?? { brand: '', quips: [], quipsMode: 'add', position: 'bottom' }), accent, imageUrl: keepImage ? design.imageUrl : null }
}

// Reads a remembered design back, trusting none of it more than a theme's own JSON.
export function fromStorage(raw) {
  try {
    const d = typeof raw === 'string' ? JSON.parse(raw) : raw
    if (!d || typeof d !== 'object') return null
    return {
      brand: typeof d.brand === 'string' ? d.brand.trim().slice(0, MAX_BRAND) : '',
      quips: cleanLines(d.quips),
      quipsMode: MODES.includes(d.quipsMode) ? d.quipsMode : 'add',
      accent: safeColour(d.accent),
      position: POSITIONS.includes(d.position) ? d.position : 'bottom',
      imageUrl: typeof d.imageUrl === 'string' && /^(data:image\/(png|jpeg|webp);base64,|asset:|https?:\/\/asset\.localhost|\/|blob:)/i.test(d.imageUrl) ? d.imageUrl : null,
    }
  } catch {
    return null
  }
}
