// The WORLD's stage: renderer, sky dome, light and fog, and a loop capped at 30 frames a second
// that can be paused while the window or the room is hidden. Everything else hangs its update on
// `onUpdate(fn)`; fn(dt, t, sky) runs once per drawn frame.
import * as THREE from 'three'
import { createSkyDome, skyAt, arcDirection } from './sky.js'

const FRAME_MS = 1000 / 30
const SKY_EVERY_S = 30

export function createWorld(canvas, { clock: now = () => new Date() } = {}) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, powerPreference: 'high-performance' })
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5))
  renderer.outputColorSpace = THREE.SRGBColorSpace
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  renderer.toneMappingExposure = 1.05

  const scene = new THREE.Scene()
  const camera = new THREE.PerspectiveCamera(45, 16 / 9, 0.2, 900)
  camera.position.set(0, 24, 80)
  camera.lookAt(0, 0, 0)

  const dome = createSkyDome()
  scene.add(dome.mesh)

  const hemi = new THREE.HemisphereLight('#ffffff', '#4a4070', 0.6)
  const sun = new THREE.DirectionalLight('#ffffff', 1.2)
  scene.add(hemi, sun, sun.target)
  scene.fog = new THREE.Fog('#9a8ac0', 80, 380)

  let sky = skyAt(now())
  function applySky() {
    sky = skyAt(now())
    dome.apply(sky)
    sun.color.set(sky.light)
    sun.intensity = sky.strength * 1.6
    hemi.color.set(sky.light)
    hemi.intensity = sky.ambient * 1.25
    scene.fog.color.set(sky.horizon)
    // by day the light comes from the sun; by night from the moon
    const body = sky.night ? sky.moon : sky.sun
    const [x, y, z] = arcDirection(body.around, Math.max(0.15, body.height))
    sun.position.set(x * 200, y * 200, z * 200)
  }
  applySky()

  const updates = []
  let running = true
  let disposed = false
  let last = 0
  let skyClock = 0
  let handle = 0
  const clock = new THREE.Clock()

  function resize() {
    const w = canvas.clientWidth || canvas.parentElement?.clientWidth || 1280
    const h = canvas.clientHeight || canvas.parentElement?.clientHeight || 720
    renderer.setSize(w, h, false)
    camera.aspect = w / Math.max(1, h)
    camera.updateProjectionMatrix()
  }
  const observer = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(resize) : null
  observer?.observe(canvas)
  resize()

  function frame(time) {
    if (!running || disposed) return
    handle = requestAnimationFrame(frame)
    if (time - last < FRAME_MS) return
    last = time
    const dt = Math.min(clock.getDelta(), 0.1)
    const t = clock.elapsedTime
    skyClock += dt
    if (skyClock > SKY_EVERY_S) {
      skyClock = 0
      applySky()
    }
    for (const update of updates) update(dt, t, sky)
    dome.tick(t, camera.position)
    renderer.render(scene, camera)
  }
  handle = requestAnimationFrame(frame)

  return {
    THREE,
    renderer,
    scene,
    camera,
    get sky() {
      return sky
    },
    get running() {
      return running
    },
    onUpdate(fn) {
      updates.push(fn)
      return () => {
        const i = updates.indexOf(fn)
        if (i >= 0) updates.splice(i, 1)
      }
    },
    refreshSky: applySky,
    setRunning(on) {
      on = !!on
      if (on === running || disposed) return
      running = on
      cancelAnimationFrame(handle)
      if (on) {
        clock.getDelta() // the time spent paused is not one long frame
        applySky()
        handle = requestAnimationFrame(frame)
      }
    },
    // Frees everything the world holds - geometries, materials, textures and the GPU context.
    dispose() {
      if (disposed) return
      disposed = true
      running = false
      cancelAnimationFrame(handle)
      observer?.disconnect()
      updates.length = 0
      scene.traverse((o) => {
        o.geometry?.dispose?.()
        const mats = Array.isArray(o.material) ? o.material : o.material ? [o.material] : []
        for (const m of mats) {
          for (const value of Object.values(m)) if (value?.isTexture) value.dispose()
          if (m.uniforms) for (const u of Object.values(m.uniforms)) if (u?.value?.isTexture) u.value.dispose()
          m.dispose?.()
        }
        if (o.isLight) o.dispose?.()
      })
      scene.clear()
      renderer.dispose()
      renderer.forceContextLoss?.()
    },
  }
}
