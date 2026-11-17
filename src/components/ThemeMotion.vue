<script setup>
// A theme's living water on HOME (theme.json `motion`, read by themes/motion.js): the surf's glow
// riding its waves, or a river's light streaks and ripple rings. One canvas over the wallpaper and
// under everything else, lined up with the wallpaper's own cover box. Gentle on the machine: at most
// 30 frames a second, a fixed number of points, a capped pixel ratio, and no frames at all while HOME
// or the window is hidden, when the member turns moving wallpapers off, or with reduced motion.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { coverRect } from '../themes/quotaglow.js'
import {
  along,
  newRipple,
  newStreak,
  pointOnWater,
  readMotion,
  rippleState,
  stepRipples,
  stepStreaks,
  stepSurf,
  streakAlpha,
  surfBrightness,
  surfParticles,
  toScreen,
} from '../themes/motion.js'

const props = defineProps({
  theme: { type: Object, default: null },
  visible: { type: Boolean, default: true },
  enabled: { type: Boolean, default: true },
})

const FRAME_MS = 1000 / 30
const MAX_RATIO = 1.5
const MAX_RIPPLES = 10

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
let ripples = []
let nextFixed = 0
let nextRandom = 0
let fixedIndex = 0
let sprite = null

// A soft round glow in the theme's colour, drawn once and stamped for every point.
function makeSprite(color) {
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

function reset() {
  const m = motion.value
  seconds = 0
  particles = []
  streaks = []
  ripples = []
  nextFixed = 0.5
  nextRandom = 1.5
  sprite = null
  if (!m) return
  if (m.kind === 'surf') {
    particles = surfParticles(m)
    sprite = makeSprite(m.color)
  } else {
    streaks = Array.from({ length: m.count }, () => newStreak(m)).filter(Boolean)
    // spread their ages so they do not all appear at once
    for (const s of streaks) s.age = Math.random() * s.life
  }
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

function loadImage() {
  image = null
  rect = null
  const url = motion.value && props.theme?.wallpaperUrl
  if (!url) return
  const img = new Image()
  img.onload = () => {
    if (props.theme?.wallpaperUrl !== url) return
    image = { width: img.naturalWidth, height: img.naturalHeight }
    measure()
  }
  img.src = url
}

function drawSurf(g, m) {
  g.globalCompositeOperation = 'lighter'
  const unit = rect.width / 1000 // a point's size follows the picture's size on screen
  for (const p of particles) {
    const at = along(m.crests[p.crest], p.t)
    const s = toScreen({ x: at.x, y: at.y + p.lift }, rect)
    const r = (2.5 + p.size * 2.5) * unit
    g.globalAlpha = surfBrightness(p, seconds) * 0.85
    g.drawImage(sprite, s.x - r, s.y - r, r * 2, r * 2)
  }
}

function drawRiver(g, m) {
  g.globalCompositeOperation = 'lighter'
  g.strokeStyle = m.color
  g.lineCap = 'round'
  for (const s of streaks) {
    const head = toScreen(s, rect)
    const near = 0.35 + s.y
    const length = rect.height * 0.012 * near * s.speed
    g.globalAlpha = streakAlpha(s) * 0.32
    g.lineWidth = Math.max(0.6, rect.width / 1600 * near)
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

function step(dt) {
  const m = motion.value
  seconds += dt
  if (m.kind === 'surf') return stepSurf(particles, dt)
  stepStreaks(streaks, m, dt)
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
  else drawRiver(g, m)
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
