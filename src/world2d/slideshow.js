// The 2D WORLD's slideshow, as pure helpers: a repeatable shuffle, stepping round the pictures,
// and the slow Ken Burns pan and zoom each slide gets. Tested without a browser.
import { seeded } from '../world3d/random.js'

// A shuffled copy of `list` (Fisher-Yates); the same seed always gives the same order.
export function shuffle(list, seed = 1) {
  const out = [...(list ?? [])]
  const rand = seeded(seed)
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1))
    ;[out[i], out[j]] = [out[j], out[i]]
  }
  return out
}

// The index `step` slides on from `index`, wrapping round both ways. -1 when there is nothing.
export function nextIndex(index, length, step = 1) {
  if (!length || length < 1) return -1
  const i = Number.isFinite(index) ? Math.trunc(index) : 0
  return (((i + step) % length) + length) % length
}

export const MIN_ZOOM = 1.06
export const MAX_ZOOM = 1.18

// How far a picture zoomed `scale` times may slide (percent of its size) before an edge shows.
export function maxPan(scale) {
  return ((scale - 1) / (2 * scale)) * 100
}

// The pan and zoom for slide number `n`: { from: { scale, x, y }, to: { scale, x, y } }, x and y
// in percent. Slides take turns zooming in and out, and never slide far enough to show an edge.
export function kenBurns(n, seed = 1) {
  const rand = seeded(`${seed}:${n}`)
  const big = MIN_ZOOM + 0.06 + rand() * (MAX_ZOOM - MIN_ZOOM - 0.06)
  const small = MIN_ZOOM + rand() * 0.03
  const zoomIn = n % 2 === 0
  const point = (scale) => {
    const reach = maxPan(scale) * 0.85
    const angle = rand() * Math.PI * 2
    const r = reach * (0.4 + rand() * 0.6)
    return { scale: round(scale), x: round(Math.cos(angle) * r), y: round(Math.sin(angle) * r) }
  }
  const a = point(small)
  const b = point(big)
  return zoomIn ? { from: a, to: b } : { from: b, to: a }
}

const round = (v) => Math.round(v * 1000) / 1000

// A CSS transform for one end of the move.
export function transformCss({ scale, x, y }) {
  return `scale(${scale}) translate(${x}%, ${y}%)`
}
