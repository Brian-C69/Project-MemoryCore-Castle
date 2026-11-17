// A theme's living water: a `motion` block in theme.json, its positions in fractions of the picture
// (0..1 across, 0..1 down), so the effect stays on the water at any window size once the
// wallpaper's cover box (quotaglow.js coverRect) maps it to the screen.
//   { "kind": "surf", "crests": [[[x, y], ...], ...], "color": "#5fd8ff", "count": 160 }
//     glowing points ride each breaking wave and a brighter wash runs along it now and then
//   { "kind": "river", "water": [[x, y], ...], "flow": [dx, dy], "ripples": [[x, y], ...], "color": "#e8f7c8", "count": 70 }
//     light streaks drift down the water, sun patches swell and fade on it, and rings spread from the given points
//   { "kind": "shore", "line": [[x, y], ...], "toward": [dx, dy], "reach": 0.03, "color": "#fff6e8" }
//     the foam at the waterline washes up the sand along `toward` and draws back on a slow sea rhythm
//   { "kind": "twinkle", "area": [[x, y], ...], "count": 36, "color": "#ffffff" }
//     the brightest stars inside `area`, found in the picture itself, flare and fade in turn
// Everything here is plain math, apart from the page, so it can be tested.

const MAX_COUNT = { surf: 240, river: 160, twinkle: 80 }
const DEFAULT_COUNT = { surf: 160, river: 90, twinkle: 36 }

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
    return { kind: 'river', water, flow, ripples, color: colour(motion.color, '#e8f7c8'), count: count(motion.count, 'river'), patches: 10 }
  }
  if (motion.kind === 'shore') {
    const line = points(motion.line)
    if (!line || line.length < 2) return null
    const [dx, dy] = Array.isArray(motion.toward) ? motion.toward.map(Number) : [0, -1]
    const length = Math.hypot(dx, dy)
    const toward = Number.isFinite(length) && length > 0 ? { x: dx / length, y: dy / length } : { x: 0, y: -1 }
    const reach = Number(motion.reach)
    return { kind: 'shore', line, toward, reach: Number.isFinite(reach) && reach > 0 ? Math.min(reach, 0.1) : 0.03, color: colour(motion.color, '#fff6e8') }
  }
  if (motion.kind === 'twinkle') {
    const area = points(motion.area)
    if (!area || area.length < 3) return null
    return { kind: 'twinkle', area, count: count(motion.count, 'twinkle'), color: colour(motion.color, '#ffffff') }
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
// Where the rolling band of light is on a crest (t, from just before the crest to past its end).
export function washAt(crest, seconds) {
  return ((seconds / 6 + crest * 0.37) % 1.4) - 0.2
}

// The part of a crest the band lights now: { from, to } in t, or null while it is off the crest.
export function bandSpan(crest, seconds, width = 0.16) {
  const at = washAt(crest, seconds)
  const from = Math.max(0, at - width / 2)
  const to = Math.min(1, at + width / 2)
  return to > from ? { from, to, at } : null
}

// How bright a point is: a faint shimmer between bands, bright while the band passes it.
export function surfBrightness(p, seconds) {
  const shimmer = 0.18 + 0.17 * Math.sin(seconds * 2.2 + p.phase)
  const wash = Math.exp(-((p.t - washAt(p.crest, seconds)) ** 2) / 0.004)
  return Math.min(1, shimmer + 0.85 * wash)
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

// A sun patch: a soft light that drifts with the current, swells and fades.
export function newPatch(motion, random = Math.random) {
  const at = pointOnWater(motion.water, random)
  return at ? { ...at, age: 0, life: 4 + random() * 4, size: 0.02 + random() * 0.03 } : null
}

export function stepPatches(patches, motion, dt, random = Math.random) {
  for (let i = 0; i < patches.length; i++) {
    const p = patches[i]
    const near = 0.35 + p.y
    p.x += motion.flow.x * 0.006 * near * dt
    p.y += motion.flow.y * 0.006 * near * dt
    p.age += dt
    if (p.age > p.life || !inside(p, motion.water)) patches[i] = newPatch(motion, random) ?? p
  }
}

// A patch's glow over its life: up and down like a breath.
export function patchAlpha(p) {
  return Math.max(0, Math.sin(Math.PI * Math.min(1, p.age / p.life)))
}

// ---------- shore ----------

// How far the foam has washed up the sand, 0 (the waterline) to 1 (its reach): in quickly, back slowly.
export function shoreSwing(seconds, period = 8) {
  const f = (seconds % period) / period
  return f < 0.35 ? Math.sin((f / 0.35) * (Math.PI / 2)) : Math.cos(((f - 0.35) / 0.65) * (Math.PI / 2))
}

// The foam's specks along the waterline, each with its own sparkle.
export function foamSpecks(motion, random = Math.random, number = 70) {
  return Array.from({ length: number }, () => ({ t: random(), phase: random() * Math.PI * 2, lag: random() * 0.25, size: 0.5 + random() }))
}

// ---------- twinkle ----------

// The brightest stars in a grey-level grid (values 0..1, `width` x `height`, row by row): points
// brighter than all eight neighbours, inside `area` (picture fractions), strongest first, never
// closer than `gap` (a fraction of the width) to a brighter one.
export function brightestPoints(grid, width, height, area, number, gap = 0.02) {
  const found = []
  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      const v = grid[y * width + x]
      if (v < 0.55) continue
      let peak = true
      for (let dy = -1; dy <= 1 && peak; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          if ((dx || dy) && grid[(y + dy) * width + x + dx] > v) {
            peak = false
            break
          }
        }
      }
      const at = { x: (x + 0.5) / width, y: (y + 0.5) / height }
      if (peak && inside(at, area)) found.push({ ...at, b: v })
    }
  }
  found.sort((a, b) => b.b - a.b)
  const kept = []
  for (const p of found) {
    if (kept.length >= number) break
    if (kept.every((k) => Math.hypot(k.x - p.x, (k.y - p.y) * (height / width)) >= gap)) kept.push(p)
  }
  return kept
}

// Each star's own rhythm, so they flare in turn.
export function twinkleStars(points, random = Math.random) {
  return points.map((p) => ({ ...p, phase: random() * Math.PI * 2, period: 2.5 + random() * 3.5 }))
}

// How bright a star's flare is now (0..1): mostly resting, a short bright flare once a period.
export function twinkleState(star, seconds) {
  const wave = 0.5 + 0.5 * Math.sin((seconds / star.period) * Math.PI * 2 + star.phase)
  return wave ** 6
}
