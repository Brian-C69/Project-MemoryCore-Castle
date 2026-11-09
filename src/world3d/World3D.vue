<script setup>
// A 3D WORLD under the real sky. The built-in isles are drawn from geometry, with the companion's
// crystal resting on the middle isle and now and then crossing a bridge of light. A world made of
// model pieces is either an archipelago (the pieces stand in for the isles, the crystal still
// walks) or a centrepiece (the companion's own piece glows at the heart, islets circle it). The
// companion speaks while busy, rests while it sleeps, and waves when a message is sent.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { createWorld } from './world.js'
import { buildIsles } from './isles.js'
import { buildCentrepiece } from './centrepiece.js'
import { Crystal } from './crystal.js'
import { CameraRig } from './camera.js'
import { loadPiece, pieceProblems } from './glb.js'
import { along, pathLength, pathPoints, route, turnBetween } from './walk.js'

const props = defineProps({
  // { busy, asleep, sentAt } - see worldSignals in ./signals.js
  signals: { type: Object, default: () => ({ busy: false, asleep: false, sentAt: 0 }) },
  // false while the world is covered or hidden, so the GPU rests
  active: { type: Boolean, default: true },
  // the same seed always builds the same isles
  seed: { type: Number, default: 7 },
  // the screen's camera: { camera: 'director'|'follow'|'wide'|'orbit', holdMin, holdMax, glide, shots }
  camera: { type: Object, default: () => ({ camera: 'director' }) },
  // a world made of model pieces (a registry entry with engine 'glb'), or null for the built-in isles
  world: { type: Object, default: null },
  // the weekly limit left on the member's CLI, 0..1 (null = unknown): the crystal shines with it
  energy: { type: Number, default: null },
})
const emit = defineEmits(['ready', 'unsupported'])

const canvas = ref(null)
const problem = ref('')

const WALK_SPEED = 2.4 // metres a second
const PAUSE_MIN_S = 18
const PAUSE_MAX_S = 45

let world = null
let isles = null
let built = null // a centrepiece build
let subject = null // the crystal, or the companion's glowing piece
let rig = null
let trip = null // { points, length, travelled, to }
let here = null
let pauseLeft = 0
let gone = false

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
  subject.play('walk')
}

// The crystal's day: rest a while on an isle, then cross to another. It stays put while the
// companion works or sleeps, but always finishes a crossing it has begun.
function wander(dt) {
  if (trip) {
    trip.travelled += WALK_SPEED * dt
    const step = along(trip.points, trip.travelled)
    subject.position.set(...step.at)
    subject.heading += turnBetween(subject.heading, step.heading) * Math.min(1, dt * 5)
    if (step.done) {
      here = trip.to
      trip = null
      subject.play('idle')
      pauseLeft = PAUSE_MIN_S + Math.random() * (PAUSE_MAX_S - PAUSE_MIN_S)
    }
    return
  }
  if (props.signals?.busy || props.signals?.asleep) return
  pauseLeft -= dt
  if (pauseLeft <= 0) startTrip()
}

function applySignals() {
  if (!subject) return
  subject.setSpeaking(!!props.signals?.busy)
  subject.setResting(!!props.signals?.asleep)
  subject.setEnergy?.(props.energy)
}

// The isles with the walking crystal; `models` / `extras` stand model pieces in for drawn isles.
function buildWalkingIsles(models = new Map(), extras = []) {
  isles = buildIsles(world, { seed: props.seed, models, props: extras })
  const crystal = new Crystal()
  world.scene.add(crystal.group)
  subject = crystal
  here = isles.centre
  crystal.position.set(...isles.stand(here))
  pauseLeft = PAUSE_MIN_S * 0.5
  world.onUpdate((dt, t) => {
    wander(dt)
    crystal.update(dt, t)
  })
}

async function buildModelWorld(entry) {
  const problems = pieceProblems(entry)
  if (problems.length) throw new Error(problems.join('; '))
  const loaded = await Promise.all(entry.pieces.map(async (p) => ({ piece: p, object: await loadPiece(p.url, p.size) })))
  if (gone) return
  const of = (role) => loaded.filter((l) => l.piece.role === role)
  const extras = of('prop').map((l) => ({ object: l.object, at: l.piece.at }))
  if (entry.layout === 'archipelago') {
    buildWalkingIsles(new Map(of('isle').map((l) => [l.piece.isle, l.object])), extras)
    return
  }
  built = buildCentrepiece(
    world,
    {
      base: of('base')[0].object,
      companion: of('companion')[0]?.object ?? null,
      islets: of('islet').map((l) => ({ object: l.object, orbit: l.piece.orbit })),
      props: extras,
    },
    { glow: entry.glow, seed: props.seed },
  )
  subject = built.subject
}

onMounted(async () => {
  try {
    world = createWorld(canvas.value)
  } catch (reason) {
    problem.value = 'This computer could not start the 3D world.'
    emit('unsupported', String(reason?.message ?? reason))
    return
  }
  if (props.world?.engine === 'glb') {
    try {
      await buildModelWorld(props.world)
    } catch (reason) {
      problem.value = `${props.world.title} could not be built (${reason?.message ?? reason}) - showing the built-in isles.`
    }
  }
  if (gone) return
  if (!subject) buildWalkingIsles()
  applySignals()
  rig = new CameraRig(world, subject, { scale: props.world?.camera?.scale ?? 1, framing: props.world?.camera ?? {} })
  rig.setSettings(props.camera)
  document.addEventListener('visibilitychange', syncRunning)
  syncRunning()
  emit('ready')
})

watch(() => [props.signals?.busy, props.signals?.asleep, props.energy], applySignals)
watch(
  () => props.signals?.sentAt,
  (value, old) => {
    if (value && value !== old) subject?.play('wave')
  },
)
watch(() => props.active, syncRunning)
watch(() => props.camera, (value) => rig?.setSettings(value), { deep: true })

onBeforeUnmount(() => {
  gone = true
  document.removeEventListener('visibilitychange', syncRunning)
  rig?.dispose()
  built?.dispose()
  world?.dispose()
  world = isles = built = subject = rig = trip = null
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
  left: 50%;
  bottom: 18px;
  margin: 0;
  padding: 6px 14px;
  border-radius: 10px;
  background: rgba(4, 8, 23, 0.6);
  color: var(--text-dim, #b9b4d8);
  font-size: 0.85rem;
  transform: translateX(-50%);
}
</style>
