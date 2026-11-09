<script setup>
// Deep Space - recipe by Ashran Helmi · ashranhelmi.com. The scenery alone: a nebula painted once,
// fog, stars and glitter, and a camera orbiting at a distance. Its own small renderer (no sky dome,
// no crystal), capped at 30 frames a second and resting while the screen is hidden or covered.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as THREE from 'three'
import { SpaceOrbit, createSpaceBackdrop } from './space.js'

const props = defineProps({
  // false while covered or hidden: the GPU rests
  active: { type: Boolean, default: true },
  palette: { type: String, default: 'kociverse' },
  brightness: { type: Number, default: 0.55 },
})
const emit = defineEmits(['unsupported'])

const canvas = ref(null)
const problem = ref('')
const FRAME_MS = 1000 / 30
let world = null

function createSpaceWorld(el) {
  const renderer = new THREE.WebGLRenderer({ canvas: el, antialias: true, powerPreference: 'high-performance' })
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5))
  renderer.outputColorSpace = THREE.SRGBColorSpace
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  const scene = new THREE.Scene()
  const camera = new THREE.PerspectiveCamera(50, 16 / 9, 0.5, 1200)
  const updates = []
  const disposers = []
  const clock = new THREE.Clock()
  let running = true
  let disposed = false
  let last = 0
  let handle = 0

  function resize() {
    const w = el.clientWidth || el.parentElement?.clientWidth || 1280
    const h = el.clientHeight || el.parentElement?.clientHeight || 720
    renderer.setSize(w, h, false)
    camera.aspect = w / Math.max(1, h)
    camera.updateProjectionMatrix()
  }
  const observer = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(resize) : null
  observer?.observe(el)
  resize()

  function frame(time) {
    if (!running || disposed) return
    handle = requestAnimationFrame(frame)
    if (time - last < FRAME_MS) return
    last = time
    const dt = Math.min(clock.getDelta(), 0.1)
    for (const update of updates) update(dt, clock.elapsedTime)
    renderer.render(scene, camera)
  }
  handle = requestAnimationFrame(frame)

  return {
    renderer,
    scene,
    camera,
    onUpdate: (fn) => updates.push(fn),
    onDispose: (fn) => disposers.push(fn),
    setRunning(on) {
      if (!!on === running || disposed) return
      running = !!on
      cancelAnimationFrame(handle)
      if (running) {
        clock.getDelta() // the time spent resting is not one long frame
        handle = requestAnimationFrame(frame)
      }
    },
    dispose() {
      if (disposed) return
      disposed = true
      running = false
      cancelAnimationFrame(handle)
      observer?.disconnect()
      for (const fn of disposers) fn()
      scene.traverse((o) => {
        o.geometry?.dispose?.()
        const mats = Array.isArray(o.material) ? o.material : o.material ? [o.material] : []
        for (const m of mats) m.dispose?.()
      })
      scene.clear()
      renderer.dispose()
      renderer.forceContextLoss?.()
    },
  }
}

function syncRunning() {
  world?.setRunning(props.active && document.visibilityState !== 'hidden')
}

let backdrop = null
onMounted(() => {
  try {
    world = createSpaceWorld(canvas.value)
    backdrop = createSpaceBackdrop(world, { palette: props.palette, brightness: props.brightness })
    new SpaceOrbit(world, { canvas: canvas.value })
  } catch (reason) {
    problem.value = 'This computer could not start the 3D world.'
    emit('unsupported', String(reason?.message ?? reason))
    return
  }
  document.addEventListener('visibilitychange', syncRunning)
  syncRunning()
})

watch(() => props.active, syncRunning)
watch(() => [props.palette, props.brightness], ([palette, brightness]) => backdrop?.set({ palette, brightness }))

onBeforeUnmount(() => {
  document.removeEventListener('visibilitychange', syncRunning)
  world?.dispose()
  world = null
})
</script>

<template>
  <div class="space">
    <canvas ref="canvas" />
    <p v-if="problem" class="problem">{{ problem }}</p>
  </div>
</template>

<style scoped>
.space {
  position: absolute;
  inset: 0;
  background: #02030a;
}
canvas {
  width: 100%;
  height: 100%;
  display: block;
  touch-action: none;
}
.problem {
  position: absolute;
  left: 50%;
  top: 18px;
  margin: 0;
  padding: 6px 14px;
  border-radius: 10px;
  background: rgba(4, 8, 23, 0.6);
  color: #d8d4f2;
  font-size: 12px;
  transform: translateX(-50%);
}
</style>
