// A theme's living water: a `motion` block in theme.json, its positions in fractions of the picture
// (0..1 across, 0..1 down), so the effect stays on the water at any window size once the
// wallpaper's cover box (quotaglow.js coverRect) maps it to the screen.
//   { "kind": "surf", "crests": [[[x, y], ...], ...], "color": "#5fd8ff", "count": 160 }
//     glowing points ride each breaking wave and a brighter wash runs along it now and then
//   { "kind": "river", "water": [[x, y], ...], "flow": [dx, dy], "ripples": [[x, y], ...], "color": "#e8f7c8", "count": 70 }
//     light streaks drift down the water and rings spread from the given points and now and then elsewhere
// Everything here is plain math, apart from the page, so it can be tested.

const MAX_COUNT = { surf: 240, river: 120 }
const DEFAULT_COUNT = { surf: 160, river: 70 }

const isFraction = (v) => typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= 1
const point = (p) => (Array.isArray(p) && p.length === 2 && isFraction(p[0]) && isFraction(p[1]) ? { x: p[0], y: p[1] } : null)
const points = (list) => {
  if (!Array.isArray(list)) return null
  const out = list.map(point)
  return out.every(Boolean) ? out : null
}
const colour = (c, fallback) => (typeof c === 'string' && /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(c) ? c : fallback)
const count = (n, kind) => {
  const v = Math.round(Number(n))
  return Number.isFinite(v) && v > 0 ? Math.min(v, MAX_COUNT[kind]) : DEFAULT_COUNT[kind]
}

// A theme's motion block made safe, or null (a bad block is ignored, never an error).
export function readMotion(motion) {
  if (!motion || typeof motion !== 'object') return null
  if (motion.kind === 'surf') {
    const crests = Array.isArray(motion.crests) ? motion.crests.map(points).filter((c) => c && c.length >= 2) : []
    if (!crests.length) return null
    return { kind: 'surf', crests, color: colour(motion.color, '#5fd8ff'), count: count(motion.count, 'surf') }
  }
  if (motion.kind === 'river') {
    const water = points(motion.water)
    if (!water || water.length < 3) return null
    const [dx, dy] = Array.isArray(motion.flow) ? motion.flow.map(Number) : [0, 1]
    const length = Math.hypot(dx, dy)
    const flow = Number.isFinite(length) && length > 0 ? { x: dx / length, y: dy / length } : { x: 0, y: 1 }
    const ripples = (Array.isArray(motion.ripples) ? motion.ripples.map(point) : []).filter(Boolean)
    return { kind: 'river', water, flow, ripples, color: colour(motion.color, '#e8f7c8'), count: count(motion.count, 'river') }
  }
  return null
}

// A picture fraction on the screen, through the wallpaper's cover box { x, y, width, height }.
export function toScreen(p, rect) {
  return { x: rect.x + p.x * rect.width, y: rect.y + p.y * rect.height }
}

// Is the point inside the polygon (both in the same units)?
export function inside(p, polygon) {
  let hit = false
  for (let i = 0, j = polygon.length - 1; i < polygon.length; j = i++) {
    const a = polygon[i]
    const b = polygon[j]
    if (a.y > p.y !== b.y > p.y && p.x < ((b.x - a.x) * (p.y - a.y)) / (b.y - a.y) + a.x) hit = !hit
  }
  return hit
}

// The point at fraction t (0..1) of a polyline's length.
export function along(line, t) {
  const lengths = []
  let total = 0
  for (let i = 1; i < line.length; i++) {
    const d = Math.hypot(line[i].x - line[i - 1].x, line[i].y - line[i - 1].y)
    lengths.push(d)
    total += d
  }
  if (!total) return { ...line[0] }
  let left = Math.min(1, Math.max(0, t)) * total
  for (let i = 0; i < lengths.length; i++) {
    if (left <= lengths[i] || i === lengths.length - 1) {
      const f = lengths[i] ? Math.min(1, left / lengths[i]) : 0
      return { x: line[i].x + (line[i + 1].x - line[i].x) * f, y: line[i].y + (line[i + 1].y - line[i].y) * f }
    }
    left -= lengths[i]
  }
  return { ...line[line.length - 1] }
}

// A small seeded random (mulberry32), so a test sees the same particles every time.
export function seeded(seed = 1) {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

// ---------- surf ----------

// The surf's points: each on a crest at t, drifting along it, with its own shimmer.
export function surfParticles(motion, random = Math.random) {
  return Array.from({ length: motion.count }, (_, i) => ({
    crest: i % motion.crests.length,
    t: random(),
    speed: 0.004 + random() * 0.01, // crest lengths per second
    phase: random() * Math.PI * 2,
    size: 0.6 + random() * 1.6,
    lift: (random() - 0.5) * 0.006, // a little above or below the crest line, as a fraction of the picture
  }))
}

export function stepSurf(particles, dt) {
  for (const p of particles) {
    p.t += p.speed * dt
    if (p.t > 1) p.t -= 1
  }
}

// How bright a point is: its own shimmer plus the wash that runs along its crest every few seconds.
export function surfBrightness(p, seconds) {
  const shimmer = 0.45 + 0.35 * Math.sin(seconds * 2.2 + p.phase)
  const washAt = ((seconds / 7 + p.crest * 0.37) % 1.4) - 0.2 // runs from just before the crest to past it
  const wash = Math.exp(-((p.t - washAt) ** 2) / 0.006)
  return Math.min(1, shimmer + 0.65 * wash)
}

// ---------- river ----------

const bounds = (polygon) => ({
  minX: Math.min(...polygon.map((p) => p.x)),
  maxX: Math.max(...polygon.map((p) => p.x)),
  minY: Math.min(...polygon.map((p) => p.y)),
  maxY: Math.max(...polygon.map((p) => p.y)),
})

// A random point on the water (null when the polygon is too thin to hit in a few tries).
export function pointOnWater(water, random = Math.random) {
  const b = bounds(water)
  for (let i = 0; i < 30; i++) {
    const p = { x: b.minX + random() * (b.maxX - b.minX), y: b.minY + random() * (b.maxY - b.minY) }
    if (inside(p, water)) return p
  }
  return null
}

// One light streak on the water; nearer the viewer (lower in the picture) it is longer and faster.
export function newStreak(motion, random = Math.random) {
  const at = pointOnWater(motion.water, random)
  return at ? { ...at, age: 0, life: 3 + random() * 4, speed: 0.6 + random() * 0.6 } : null
}

export function stepStreaks(streaks, motion, dt, random = Math.random) {
  for (let i = 0; i < streaks.length; i++) {
    const s = streaks[i]
    const near = 0.35 + s.y // perspective: the far water moves slowly
    s.x += motion.flow.x * 0.02 * s.speed * near * dt
    s.y += motion.flow.y * 0.02 * s.speed * near * dt
    s.age += dt
    if (s.age > s.life || !inside(s, motion.water)) streaks[i] = newStreak(motion, random) ?? s
  }
}

// How visible a streak is through its life: it fades in and out.
export function streakAlpha(s) {
  const f = s.age / s.life
  return Math.max(0, Math.min(1, Math.min(f * 4, (1 - f) * 3)))
}

// A ripple ring: from a fixed point (the boat) or a random one on the water.
export function newRipple(at, random = Math.random) {
  return { x: at.x, y: at.y, age: 0, life: 2.4 + random() * 1.6, reach: 0.025 + random() * 0.03 }
}

export function stepRipples(ripples, dt) {
  for (const r of ripples) r.age += dt
  return ripples.filter((r) => r.age < r.life)
}

// A ring's size (a fraction of the picture's width) and how visible it still is.
export function rippleState(r) {
  const f = Math.min(1, r.age / r.life)
  return { radius: r.reach * (0.15 + 0.85 * Math.sqrt(f)), alpha: (1 - f) * 0.7 }
}
