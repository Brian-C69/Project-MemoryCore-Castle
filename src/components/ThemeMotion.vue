<script setup>
// A theme's moving wallpaper on HOME (theme.json `motion`, read by themes/motion.js): the surf's
// rolling bands of light, a shore's washing foam, a river's current with sun patches and ripples, or a
// sky whose brightest stars twinkle. One canvas over the wallpaper and under everything else, lined up
// with the wallpaper's own cover box. Gentle on the machine: at most 30 frames a second, a fixed number
// of points, a capped pixel ratio, and no frames at all while HOME or the window is hidden, unless the
// member turned moving wallpapers on (off to begin with), or with reduced motion.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { coverRect } from '../themes/quotaglow.js'
import {
  along,
  bandSpan,
  brightestPoints,
  foamSpecks,
  newPatch,
  newRipple,
  newStreak,
  patchAlpha,
  pointOnWater,
  readMotion,
  rippleState,
  shoreSwing,
  stepPatches,
  stepRipples,
  stepStreaks,
  stepSurf,
  streakAlpha,
  surfBrightness,
  surfParticles,
  toScreen,
  twinkleStars,
  twinkleState,
} from '../themes/motion.js'

const props = defineProps({
  theme: { type: Object, default: null },
  visible: { type: Boolean, default: true },
  enabled: { type: Boolean, default: false },
})

const FRAME_MS = 1000 / 30
const MAX_RATIO = 1.5
const MAX_RIPPLES = 10
const GRID_WIDTH = 320 // the picture is read this wide to find its stars

const layer = ref(null)
const canvas = ref(null)
const awake = ref(document.visibilityState === 'visible')
const calm = ref(window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false)
const motion = computed(() => readMotion(props.theme?.motion))
const running = computed(() => Boolean(motion.value && props.visible && props.enabled && awake.value && !calm.value))

let image = null // the wallpaper's natural size
let rect = null // the wallpaper's cover box inside the canvas, in CSS pixels
let ratio = 1
let frame = 0
let last = 0
let seconds = 0
let particles = []
let streaks = []
let patches = []
let ripples = []
let specks = []
let stars = []
let nextFixed = 0
let nextRandom = 0
let fixedIndex = 0
let glow = null
let sparkle = null

// A soft round glow in the theme's colour, drawn once and stamped for every point.
function makeGlow(color) {
  const size = 32
  const c = document.createElement('canvas')
  c.width = c.height = size
  const g = c.getContext('2d')
  const gradient = g.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2)
  gradient.addColorStop(0, '#ffffff')
  gradient.addColorStop(0.25, color)
  gradient.addColorStop(1, 'rgba(0, 0, 0, 0)')
  g.fillStyle = gradient
  g.fillRect(0, 0, size, size)
  return c
}

// A star's sparkle: a bright core and four soft rays.
function makeSparkle(color) {
  const size = 64
  const c = document.createElement('canvas')
  c.width = c.height = size
  const g = c.getContext('2d')
  const mid = size / 2
  const core = g.createRadialGradient(mid, mid, 0, mid, mid, mid * 0.45)
  core.addColorStop(0, '#ffffff')
  core.addColorStop(0.3, color)
  core.addColorStop(1, 'rgba(0, 0, 0, 0)')
  g.fillStyle = core
  g.fillRect(0, 0, size, size)
  for (const [w, h] of [[size, 2.2], [2.2, size]]) {
    const ray = g.createRadialGradient(mid, mid, 0, mid, mid, mid)
    ray.addColorStop(0, 'rgba(255, 255, 255, 0.9)')
    ray.addColorStop(1, 'rgba(255, 255, 255, 0)')
    g.fillStyle = ray
    g.fillRect(mid - w / 2, mid - h / 2, w, h)
  }
  return c
}

function reset() {
  const m = motion.value
  seconds = 0
  particles = []
  streaks = []
  patches = []
  ripples = []
  specks = []
  stars = []
  nextFixed = 0.5
  nextRandom = 1.5
  glow = null
  sparkle = null
  if (!m) return
  glow = makeGlow(m.color)
  if (m.kind === 'surf') particles = surfParticles(m)
  if (m.kind === 'river') {
    streaks = Array.from({ length: m.count }, () => newStreak(m)).filter(Boolean)
    for (const s of streaks) s.age = Math.random() * s.life // not all at once
    patches = Array.from({ length: m.patches }, () => newPatch(m)).filter(Boolean)
    for (const p of patches) p.age = Math.random() * p.life
  }
  if (m.kind === 'shore') specks = foamSpecks(m)
  if (m.kind === 'twinkle') sparkle = makeSparkle(m.color)
}

function measure() {
  const el = layer.value
  const c = canvas.value
  const castle = el?.closest('.castle')
  if (!el || !c || !castle || !image) return (rect = null)
  const box = castle.getBoundingClientRect()
  const own = el.getBoundingClientRect()
  ratio = Math.min(window.devicePixelRatio || 1, MAX_RATIO)
  c.width = Math.round(own.width * ratio)
  c.height = Math.round(own.height * ratio)
  const wall = coverRect({ width: box.width, height: box.height }, image, props.theme?.wallpaperPosition)
  rect = wall && { ...wall, x: wall.x + box.left - own.left, y: wall.y + box.top - own.top }
}

// The twinkle's stars, found once in the picture itself (a picture that cannot be read leaves the sky still).
function findStars(img) {
  const m = motion.value
  if (m?.kind !== 'twinkle') return
  try {
    const width = GRID_WIDTH
    const height = Math.max(1, Math.round((img.naturalHeight / img.naturalWidth) * width))
    const c = document.createElement('canvas')
    c.width = width
    c.height = height
    const g = c.getContext('2d', { willReadFrequently: true })
    g.drawImage(img, 0, 0, width, height)
    const data = g.getImageData(0, 0, width, height).data
    const grid = new Float32Array(width * height)
    for (let i = 0; i < grid.length; i++) grid[i] = (0.2126 * data[i * 4] + 0.7152 * data[i * 4 + 1] + 0.0722 * data[i * 4 + 2]) / 255
    stars = twinkleStars(brightestPoints(grid, width, height, m.area, m.count))
  } catch {
    stars = []
  }
}

function loadImage() {
  image = null
  rect = null
  const url = motion.value && props.theme?.wallpaperUrl
  if (!url) return
  const img = new Image()
  img.onload = () => {
    if (props.theme?.wallpaperUrl !== url) return
    image = { width: img.naturalWidth, height: img.naturalHeight }
    findStars(img)
    measure()
  }
  img.src = url
}

// A stretch of a polyline between two fractions of its length, as screen points.
function stretch(line, from, to, steps = 14) {
  return Array.from({ length: steps + 1 }, (_, i) => toScreen(along(line, from + ((to - from) * i) / steps), rect))
}

function strokePath(g, pts) {
  g.beginPath()
  pts.forEach((p, i) => (i ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)))
  g.stroke()
}

function drawSurf(g, m) {
  g.globalCompositeOperation = 'lighter'
  const unit = rect.width / 1000
  g.strokeStyle = m.color
  g.lineCap = 'round'
  g.lineJoin = 'round'
  // the rolling bands: a wide soft stroke and a bright narrow one over the part of each crest they light
  m.crests.forEach((crest, i) => {
    const span = bandSpan(i, seconds)
    if (!span) return
    const pts = stretch(crest, span.from, span.to)
    g.globalAlpha = 0.22
    g.lineWidth = 14 * unit
    strokePath(g, pts)
    g.globalAlpha = 0.45
    g.lineWidth = 4 * unit
    strokePath(g, pts)
  })
  for (const p of particles) {
    const at = along(m.crests[p.crest], p.t)
    const s = toScreen({ x: at.x, y: at.y + p.lift }, rect)
    const b = surfBrightness(p, seconds)
    const r = (2.5 + p.size * 2.5) * unit * (1 + b)
    g.globalAlpha = b * 0.9
    g.drawImage(glow, s.x - r, s.y - r, r * 2, r * 2)
  }
}

function drawRiver(g, m) {
  g.globalCompositeOperation = 'lighter'
  // sun patches: soft light flattened on the water
  for (const p of patches) {
    const c = toScreen(p, rect)
    const r = p.size * rect.width
    g.globalAlpha = patchAlpha(p) * 0.28
    g.drawImage(glow, c.x - r, c.y - r * 0.45, r * 2, r * 0.9)
  }
  g.strokeStyle = m.color
  g.lineCap = 'round'
  for (const s of streaks) {
    const head = toScreen(s, rect)
    const near = 0.35 + s.y
    const length = rect.height * 0.02 * near * s.speed
    g.globalAlpha = streakAlpha(s) * 0.5
    g.lineWidth = Math.max(0.8, (rect.width / 1300) * near)
    g.beginPath()
    g.moveTo(head.x, head.y)
    g.lineTo(head.x - m.flow.x * length, head.y - m.flow.y * length)
    g.stroke()
  }
  for (const r of ripples) {
    const { radius, alpha } = rippleState(r)
    const c = toScreen(r, rect)
    const rx = radius * rect.width
    g.globalAlpha = alpha
    g.lineWidth = Math.max(0.8, rect.width / 1400)
    g.beginPath()
    g.ellipse(c.x, c.y, rx, rx * 0.32, 0, 0, Math.PI * 2) // flattened: the water is seen from above at a slant
    g.stroke()
  }
}

function drawShore(g, m) {
  const unit = rect.width / 1000
  const shifted = (swing, scale = 1) => m.line.map((p) => toScreen({ x: p.x + m.toward.x * m.reach * swing * scale, y: p.y + m.toward.y * m.reach * swing * scale }, rect))
  const swing = shoreSwing(seconds)
  g.strokeStyle = m.color
  g.lineCap = 'round'
  g.lineJoin = 'round'
  // the wet sheen the wave leaves behind, then the foam edge itself
  g.globalCompositeOperation = 'lighter'
  g.globalAlpha = 0.1 + 0.12 * swing
  g.lineWidth = 18 * unit
  strokePath(g, shifted(swing, 0.55))
  g.globalCompositeOperation = 'source-over'
  g.globalAlpha = 0.35 + 0.35 * swing
  g.lineWidth = 3 * unit
  strokePath(g, shifted(swing))
  g.globalCompositeOperation = 'lighter'
  for (const s of specks) {
    const reach = shoreSwing(seconds - s.lag * 8)
    const at = along(m.line, s.t)
    const p = toScreen({ x: at.x + m.toward.x * m.reach * reach, y: at.y + m.toward.y * m.reach * reach }, rect)
    const r = (2 + s.size * 2) * unit
    g.globalAlpha = (0.35 + 0.35 * Math.sin(seconds * 3 + s.phase)) * (0.4 + 0.6 * reach)
    g.drawImage(glow, p.x - r, p.y - r, r * 2, r * 2)
  }
}

function drawTwinkle(g) {
  g.globalCompositeOperation = 'lighter'
  const unit = rect.width / 1000
  for (const s of stars) {
    const v = twinkleState(s, seconds)
    if (v < 0.02) continue
    const c = toScreen(s, rect)
    const r = (5 + 16 * v) * unit * (0.6 + s.b * 0.6)
    g.globalAlpha = Math.min(1, 0.25 + 0.75 * v)
    g.drawImage(sparkle, c.x - r, c.y - r, r * 2, r * 2)
  }
}

function step(dt) {
  const m = motion.value
  seconds += dt
  if (m.kind === 'surf') return stepSurf(particles, dt)
  if (m.kind !== 'river') return
  stepStreaks(streaks, m, dt)
  stepPatches(patches, m, dt)
  ripples = stepRipples(ripples, dt)
  if (seconds > nextFixed && m.ripples.length && ripples.length < MAX_RIPPLES) {
    ripples.push(newRipple(m.ripples[fixedIndex++ % m.ripples.length]))
    nextFixed = seconds + 0.9 + Math.random() * 0.8
  }
  if (seconds > nextRandom && ripples.length < MAX_RIPPLES) {
    const at = pointOnWater(m.water)
    if (at) ripples.push(newRipple(at))
    nextRandom = seconds + 2 + Math.random() * 2.5
  }
}

function tick(now) {
  frame = requestAnimationFrame(tick)
  if (now - last < FRAME_MS) return
  const dt = Math.min(0.1, (now - last) / 1000)
  last = now
  const c = canvas.value
  const m = motion.value
  if (!c || !m || !rect) return
  step(dt)
  const g = c.getContext('2d')
  g.setTransform(ratio, 0, 0, ratio, 0, 0)
  g.globalCompositeOperation = 'source-over'
  g.globalAlpha = 1
  g.clearRect(0, 0, c.width / ratio, c.height / ratio)
  if (m.kind === 'surf') drawSurf(g, m)
  else if (m.kind === 'river') drawRiver(g, m)
  else if (m.kind === 'shore') drawShore(g, m)
  else if (m.kind === 'twinkle') drawTwinkle(g)
}

function start() {
  if (frame) return
  last = performance.now()
  frame = requestAnimationFrame(tick)
}

function stop() {
  cancelAnimationFrame(frame)
  frame = 0
  const c = canvas.value
  c?.getContext('2d').clearRect(0, 0, c.width, c.height)
}

const onVisibility = () => (awake.value = document.visibilityState === 'visible')
let reduced
const onReduced = (event) => (calm.value = event.matches)
let observer
onMounted(() => {
  reset()
  loadImage()
  observer = new ResizeObserver(measure)
  const castle = layer.value?.closest('.castle')
  if (castle) observer.observe(castle)
  document.addEventListener('visibilitychange', onVisibility)
  reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)')
  reduced?.addEventListener('change', onReduced)
  if (running.value) start()
})
onBeforeUnmount(() => {
  stop()
  observer?.disconnect()
  document.removeEventListener('visibilitychange', onVisibility)
  reduced?.removeEventListener('change', onReduced)
})
watch(() => [props.theme?.id, props.theme?.wallpaperUrl], () => {
  reset()
  loadImage()
})
watch(running, (now) => (now ? (measure(), start()) : stop()))
</script>

<template>
  <div ref="layer" class="theme-motion" aria-hidden="true">
    <canvas ref="canvas" />
  </div>
</template>

<style scoped>
.theme-motion {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}
canvas {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
