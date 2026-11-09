// The camera director: which shot comes next and for how long. Pure and tested; the camera rig
// (camera.js) films whatever it chooses.
//
// Shots: follow (behind the crystal), wide (the isles), close (the crystal up close),
// orbit (circling it), crane (high above, sinking slowly).

export const SHOTS = ['follow', 'wide', 'close', 'orbit', 'crane']
export const WEIGHTS = { follow: 35, wide: 25, close: 15, orbit: 15, crane: 10 }
export const WALKING_WEIGHTS = { follow: 55, wide: 25, crane: 20 }
export const RESTING_WEIGHTS = { wide: 45, orbit: 35, close: 20 }
export const MIN_S = 8
export const MAX_S = 20

// One name from `weights`, never `previous` (unless it is the only one).
export function pickWeighted(weights, previous, rand = Math.random) {
  let entries = Object.entries(weights).filter(([, w]) => w > 0)
  if (entries.length > 1) entries = entries.filter(([name]) => name !== previous)
  if (!entries.length) return previous ?? 'wide'
  const total = entries.reduce((sum, [, w]) => sum + w, 0)
  let roll = rand() * total
  for (const [name, w] of entries) {
    roll -= w
    if (roll < 0) return name
  }
  return entries[entries.length - 1][0]
}

// The member's camera choices: the director picking views, or one view kept.
export const CAMERAS = [
  { id: 'director', title: 'Director - random views' },
  { id: 'follow', title: 'Follow the crystal' },
  { id: 'wide', title: 'Wide only' },
  { id: 'orbit', title: 'Slow orbit' },
]
export const GLIDES = ['glide', 'cut', 'mix']

// `weights` kept to the views the member ticked; when none of them fit (e.g. no walking view is
// ticked), the calm weights kept to the ticked views; when even that is empty, wide.
function allowed(weights, shots) {
  if (!shots?.length) return weights
  const keep = (w) => Object.fromEntries(Object.entries(w).filter(([name]) => shots.includes(name)))
  const kept = keep(weights)
  if (Object.keys(kept).length) return kept
  const calm = keep(WEIGHTS)
  return Object.keys(calm).length ? calm : { wide: 1 }
}

// { shot, seconds, glide } - glide true = a smooth glide from the last shot, false = a cut.
// `only` keeps a single shot (a fixed camera); holdMin/holdMax bound the seconds; `shots` are the
// views the director may use; `glide` is 'glide' (always), 'cut' (never) or 'mix'.
export function nextShot({
  previous = null,
  walking = false,
  resting = false,
  only = null,
  holdMin = MIN_S,
  holdMax = MAX_S,
  shots = null,
  glide = 'mix',
  rand = Math.random,
} = {}) {
  const lo = Math.max(1, Math.min(holdMin, holdMax))
  const hi = Math.max(lo, holdMin, holdMax)
  const seconds = lo + rand() * (hi - lo)
  if (only && SHOTS.includes(only)) return { shot: only, seconds, glide: true }
  const weights = allowed(walking ? WALKING_WEIGHTS : resting ? RESTING_WEIGHTS : WEIGHTS, shots)
  const shot = pickWeighted(weights, previous, rand)
  const smooth = glide === 'glide' ? true : glide === 'cut' ? false : rand() < 0.6
  return { shot, seconds, glide: smooth }
}
