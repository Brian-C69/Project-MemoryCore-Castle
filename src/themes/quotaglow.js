// A theme may mark a shining thing painted in its wallpaper (`quotaGlow`), and HOME lays a light
// over it that follows the weekly limit left on the member's CLI. The numbers are fractions (0..1):
// the thing's centre and size and an optional shine below it are fractions of the wallpaper; the
// outline's points are fractions of the thing's own box. This file holds the maths: where the
// wallpaper lands on screen and where the light goes.

const KEYWORDS = { left: 0, top: 0, center: 0.5, right: 1, bottom: 1 }

// "center", "center top", "30% 70%" -> [x, y] as 0..1 (how CSS places a background).
export function parsePosition(position = 'center') {
  const parts = String(position).trim().toLowerCase().split(/\s+/).filter(Boolean)
  const read = (p) => (p in KEYWORDS ? KEYWORDS[p] : p.endsWith('%') && !Number.isNaN(parseFloat(p)) ? parseFloat(p) / 100 : null)
  if (parts.length === 0) return [0.5, 0.5]
  if (parts.length === 1) {
    const v = read(parts[0])
    if (v === null) return [0.5, 0.5]
    // a lone "top" / "bottom" moves y, anything else moves x
    return parts[0] === 'top' || parts[0] === 'bottom' ? [0.5, v] : [v, 0.5]
  }
  let [a, b] = parts
  // "top left" means left top
  if (a === 'top' || a === 'bottom' || b === 'left' || b === 'right') [a, b] = [b, a]
  return [read(a) ?? 0.5, read(b) ?? 0.5]
}

// Where a `background-size: cover` picture lands inside a box: {x, y, width, height}.
export function coverRect(box, image, position = 'center') {
  if (!box?.width || !box?.height || !image?.width || !image?.height) return null
  const scale = Math.max(box.width / image.width, box.height / image.height)
  const width = image.width * scale
  const height = image.height * scale
  const [px, py] = parsePosition(position)
  return { x: (box.width - width) * px, y: (box.height - height) * py, width, height }
}

const fraction = (v) => typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= 1

// A theme's quotaGlow, checked: null when it cannot be used.
export function readGlow(glow) {
  if (!glow || typeof glow !== 'object') return null
  const { x, y, width, height } = glow
  if (![x, y, width, height].every(fraction) || width === 0 || height === 0) return null
  const outline = Array.isArray(glow.outline) && glow.outline.length >= 3 && glow.outline.every((p) => Array.isArray(p) && p.length === 2 && p.every(fraction)) ? glow.outline : null
  const shine = glow.shine && fraction(glow.shine.x) && fraction(glow.shine.y) ? { x: glow.shine.x, y: glow.shine.y } : null
  return { x, y, width, height, outline, shine }
}

// The thing and its shine in screen pixels, given where the wallpaper landed.
export function glowBoxes(glow, wall) {
  if (!glow || !wall) return null
  const width = glow.width * wall.width
  const height = glow.height * wall.height
  const thing = { x: wall.x + glow.x * wall.width - width / 2, y: wall.y + glow.y * wall.height - height / 2, width, height }
  const shine = glow.shine ? { x: wall.x + glow.shine.x * wall.width, y: wall.y + glow.shine.y * wall.height } : null
  return { thing, shine }
}

// The outline as a CSS clip-path inside the thing's box (an ellipse when there is none).
export function clipPath(outline) {
  if (!outline) return 'ellipse(50% 50% at 50% 50%)'
  return `polygon(${outline.map(([x, y]) => `${(x * 100).toFixed(1)}% ${(y * 100).toFixed(1)}%`).join(', ')})`
}
