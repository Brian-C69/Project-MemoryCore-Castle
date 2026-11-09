<script setup>
// The 3D WORLD: floating isles under the real sky, with the companion's crystal resting on the
// middle isle and now and then crossing a bridge of light to another. It speaks while the
// companion is busy, rests while it sleeps, and waves when a message is sent.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { createWorld } from './world.js'
import { buildIsles } from './isles.js'
import { Crystal } from './crystal.js'
import { CameraRig } from './camera.js'
import { along, pathLength, pathPoints, route, turnBetween } from './walk.js'

const props = defineProps({
  // { busy, asleep, sentAt } - see worldSignals in ./signals.js
  signals: { type: Object, default: () => ({ busy: false, asleep: false, sentAt: 0 }) },
  // false while the WORLD room is covered, so the GPU rests
  active: { type: Boolean, default: true },
  // the same seed always builds the same isles
  seed: { type: Number, default: 7 },
  // the screen's camera: { camera: 'director'|'follow'|'wide'|'orbit', holdMin, holdMax, glide, shots }
  camera: { type: Object, default: () => ({ camera: 'director' }) },
})
const emit = defineEmits(['ready', 'unsupported'])

const canvas = ref(null)
const problem = ref('')

const WALK_SPEED = 2.4 // metres a second
const PAUSE_MIN_S = 18
const PAUSE_MAX_S = 45

let world = null
let isles = null
let crystal = null
let rig = null
let trip = null // { points, length, travelled, to }
let here = null
let pauseLeft = 0

function visible() {
  return typeof document === 'undefined' || document.visibilityState !== 'hidden'
}

function syncRunning() {
  world?.setRunning(props.active && visible())
}

function startTrip() {
  const others = isles.ids.filter((id) => id !== here)
  if (!others.length) return
  const to = others[Math.floor(Math.random() * others.length)]
  const steps = route(isles.bridges, here, to)
  if (steps.length < 2) return
  const points = pathPoints(steps, { stand: isles.stand, bridge: isles.bridge })
  trip = { points, length: pathLength(points), travelled: 0, to }
  crystal.play('walk')
}

// The crystal's day: rest a while on an isle, then cross to another. It stays put while the
// companion works or sleeps, but always finishes a crossing it has begun.
function wander(dt) {
  if (trip) {
    trip.travelled += WALK_SPEED * dt
    const step = along(trip.points, trip.travelled)
    crystal.position.set(...step.at)
    crystal.heading += turnBetween(crystal.heading, step.heading) * Math.min(1, dt * 5)
    if (step.done) {
      here = trip.to
      trip = null
      crystal.play('idle')
      pauseLeft = PAUSE_MIN_S + Math.random() * (PAUSE_MAX_S - PAUSE_MIN_S)
    }
    return
  }
  if (props.signals?.busy || props.signals?.asleep) return
  pauseLeft -= dt
  if (pauseLeft <= 0) startTrip()
}

function applySignals() {
  if (!crystal) return
  crystal.setSpeaking(!!props.signals?.busy)
  crystal.setResting(!!props.signals?.asleep)
}

onMounted(() => {
  try {
    world = createWorld(canvas.value)
  } catch (reason) {
    problem.value = 'This computer could not start the 3D world.'
    emit('unsupported', String(reason?.message ?? reason))
    return
  }
  isles = buildIsles(world, { seed: props.seed })
  crystal = new Crystal()
  world.scene.add(crystal.group)
  here = isles.centre
  crystal.position.set(...isles.stand(here))
  pauseLeft = PAUSE_MIN_S * 0.5
  applySignals()
  world.onUpdate((dt, t) => {
    wander(dt)
    crystal.update(dt, t)
  })
  rig = new CameraRig(world, crystal)
  rig.setSettings(props.camera)
  document.addEventListener('visibilitychange', syncRunning)
  syncRunning()
  emit('ready')
})

watch(() => [props.signals?.busy, props.signals?.asleep], applySignals)
watch(
  () => props.signals?.sentAt,
  (value, old) => {
    if (value && value !== old) crystal?.play('wave')
  },
)
watch(() => props.active, syncRunning)
watch(() => props.camera, (value) => rig?.setSettings(value), { deep: true })

onBeforeUnmount(() => {
  document.removeEventListener('visibilitychange', syncRunning)
  rig?.dispose()
  world?.dispose()
  world = isles = crystal = rig = trip = null
})

defineExpose({
  setRunning: (on) => world?.setRunning(on),
})
</script>

<template>
  <div class="world3d">
    <canvas ref="canvas" class="stage" aria-label="The companion's world" role="img"></canvas>
    <p v-if="problem" class="problem">{{ problem }}</p>
  </div>
</template>

<style scoped>
.world3d {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #0b0f26;
}
.stage {
  display: block;
  width: 100%;
  height: 100%;
}
.problem {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  margin: 0;
  color: var(--text-dim, #b9b4d8);
  font-size: 0.95rem;
}
</style>
