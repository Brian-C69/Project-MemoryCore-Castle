// The isles of the WORLD, built from geometry alone (no model files): floating rock islands with
// grass caps, a few trees and glowing crystals, bridges of light between them, a sea of clouds
// far below and drifting motes. Everything is placed by a seeded random, so the same seed always
// builds the same world.
import * as THREE from 'three'
import { seeded, hashSeed } from './random.js'
import { arc, neighbours } from './walk.js'

// at = the centre of the isle's grass top; radius = the grass radius, in metres.
export const LAYOUT = [
  { id: 'heart', at: [0, 0, 0], radius: 9 },
  { id: 'east', at: [30, 3, -8], radius: 6.5 },
  { id: 'north', at: [6, 5, -34], radius: 7 },
  { id: 'west', at: [-30, 2, -12], radius: 6 },
  { id: 'south', at: [-6, -2, 28], radius: 5.5 },
]
export const BRIDGES = [
  ['heart', 'east'],
  ['heart', 'north'],
  ['heart', 'west'],
  ['heart', 'south'],
  ['east', 'north'],
]
export const CENTRE = 'heart'

const GRASS = ['#6fbf73', '#78c47a', '#66b88a', '#7cc47f']
const ROCK = '#6b6080'
const CRYSTAL_COLOURS = ['#8fe6ff', '#c7a6ff']
const SEA_Y = -30

// ---------- the plan (pure: numbers only, no GPU) ----------

// Where things stand on each isle, and the points of every bridge. Trees and crystals keep clear
// of the lanes that lead to the bridges and of the middle, where the crystal rests.
export function planIsles({ layout = LAYOUT, bridges = BRIDGES, seed = 7 } = {}) {
  const rand = seeded(seed)
  const byId = new Map(layout.map((spec) => [spec.id, spec]))
  const near = neighbours(bridges)
  const toward = (a, b) => Math.atan2(b.at[0] - a.at[0], b.at[2] - a.at[2])

  const isles = new Map()
  for (const spec of layout) {
    const lanes = [...(near.get(spec.id) ?? [])].map((id) => toward(spec, byId.get(id)))
    const clear = (angle) => lanes.every((lane) => Math.abs(Math.atan2(Math.sin(angle - lane), Math.cos(angle - lane))) > 0.5)
    const spot = (minR, maxR) => {
      for (let tries = 0; tries < 24; tries++) {
        const angle = rand() * Math.PI * 2
        if (!clear(angle)) continue
        const r = spec.radius * (minR + rand() * (maxR - minR))
        return { x: spec.at[0] + Math.sin(angle) * r, z: spec.at[2] + Math.cos(angle) * r, turn: rand() * Math.PI * 2 }
      }
      return null
    }
    const trees = []
    const treeCount = spec.id === CENTRE ? 3 : 2 + Math.floor(rand() * 2)
    for (let n = 0; n < treeCount; n++) {
      const at = spot(0.45, 0.78)
      if (at) trees.push({ ...at, scale: 0.85 + rand() * 0.6 })
    }
    const crystals = []
    const crystalCount = 1 + Math.floor(rand() * 3)
    for (let n = 0; n < crystalCount; n++) {
      const at = spot(0.35, 0.8)
      if (at) crystals.push({ ...at, height: 0.8 + rand() * 1.1, colour: CRYSTAL_COLOURS[Math.floor(rand() * CRYSTAL_COLOURS.length)] })
    }
    isles.set(spec.id, {
      spec,
      grass: GRASS[Math.floor(rand() * GRASS.length)],
      depth: spec.radius * (1.4 + rand() * 0.5),
      trees,
      crystals,
    })
  }

  const stand = (id) => [...byId.get(id).at]
  const edge = (a, b) => {
    const A = byId.get(a)
    const angle = toward(A, byId.get(b))
    const r = A.radius * 0.92
    return [A.at[0] + Math.sin(angle) * r, A.at[1], A.at[2] + Math.cos(angle) * r]
  }
  const bridge = (a, b) => {
    const from = edge(a, b)
    const to = edge(b, a)
    const length = Math.hypot(to[0] - from[0], to[2] - from[2])
    return arc(from, to, 0.6 + length * 0.04, Math.max(8, Math.round(length / 1.5)))
  }
  return { layout, bridges, isles, stand, edge, bridge }
}

// ---------- the build ----------

let dotTexture
function softDot() {
  if (dotTexture) return dotTexture
  const c = document.createElement('canvas')
  c.width = c.height = 64
  const g = c.getContext('2d')
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32)
  grad.addColorStop(0, 'rgba(255,255,255,1)')
  grad.addColorStop(0.35, 'rgba(255,255,255,0.45)')
  grad.addColorStop(1, 'rgba(255,255,255,0)')
  g.fillStyle = grad
  g.fillRect(0, 0, 64, 64)
  dotTexture = new THREE.CanvasTexture(c)
  dotTexture.colorSpace = THREE.SRGBColorSpace
  return dotTexture
}

// A rock that hangs under the grass: a downward cone, its points nudged by a hash of their
// position (so the seam's twin vertices move together and the rock never cracks open).
function rockGeometry(radius, depth, seed) {
  const geometry = new THREE.ConeGeometry(radius, depth, 11, 5)
  geometry.rotateX(Math.PI)
  geometry.translate(0, -depth / 2, 0)
  const pos = geometry.attributes.position
  for (let i = 0; i < pos.count; i++) {
    const x = pos.getX(i)
    const y = pos.getY(i)
    const z = pos.getZ(i)
    const r = seeded(hashSeed(`${seed}:${x.toFixed(3)},${y.toFixed(3)},${z.toFixed(3)}`))
    const top = y > -0.01
    const spread = top ? 0.04 : 0.22
    const k = 1 + (r() - 0.5) * 2 * spread
    pos.setXYZ(i, x * k, top ? y : y + (r() - 0.5) * depth * 0.12, z * k)
  }
  geometry.computeVertexNormals()
  return geometry
}

// A flat ribbon along points: the deck of a bridge of light.
function ribbonGeometry(points, halfWidth) {
  const vertices = []
  const indices = []
  for (let i = 0; i < points.length; i++) {
    const a = points[Math.max(0, i - 1)]
    const b = points[Math.min(points.length - 1, i + 1)]
    const dx = b[0] - a[0]
    const dz = b[2] - a[2]
    const len = Math.hypot(dx, dz) || 1
    const sx = (dz / len) * halfWidth
    const sz = (-dx / len) * halfWidth
    const [x, y, z] = points[i]
    vertices.push(x + sx, y, z + sz, x - sx, y, z - sz)
    if (i > 0) {
      const n = i * 2
      indices.push(n - 2, n - 1, n, n - 1, n + 1, n)
    }
  }
  const geometry = new THREE.BufferGeometry()
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3))
  geometry.setIndex(indices)
  return geometry
}

const SEA_VERTEX = `
varying vec3 vWorld;
void main() {
  vec4 w = modelMatrix * vec4(position, 1.0);
  vWorld = w.xyz;
  gl_Position = projectionMatrix * viewMatrix * w;
}`
const SEA_FRAGMENT = `
uniform float time;
uniform vec3 light;
uniform vec3 shade;
uniform vec3 haze;
varying vec3 vWorld;
float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
  vec2 i = floor(p); vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}
float fbm(vec2 p) {
  float v = 0.0; float a = 0.5;
  for (int i = 0; i < 4; i++) { v += a * noise(p); p *= 2.03; a *= 0.5; }
  return v;
}
void main() {
  vec2 p = vWorld.xz * 0.018 + vec2(time * 0.012, time * 0.007);
  float c = fbm(p + fbm(p * 0.7 - time * 0.004));
  vec3 col = mix(shade, light, smoothstep(0.3, 0.8, c));
  float far = smoothstep(120.0, 480.0, length(vWorld.xz - cameraPosition.xz));
  gl_FragColor = vec4(mix(col, haze, far), 1.0);
  #include <colorspace_fragment>
}`

// Builds the isles into world.scene. Returns the plan's helpers for walking:
// { ids, centre, bridges, stand(id), bridge(a, b), plan }
export function buildIsles(world, { layout = LAYOUT, bridges = BRIDGES, seed = 7 } = {}) {
  const plan = planIsles({ layout, bridges, seed })
  const root = new THREE.Group()
  root.name = 'isles'
  world.scene.add(root)

  const rockMaterial = new THREE.MeshStandardMaterial({ color: ROCK, roughness: 0.95, flatShading: true })
  const grassMaterials = new Map()
  const grassMaterial = (colour) => {
    if (!grassMaterials.has(colour)) grassMaterials.set(colour, new THREE.MeshStandardMaterial({ color: colour, roughness: 0.9, flatShading: true }))
    return grassMaterials.get(colour)
  }
  const trunkGeometry = new THREE.CylinderGeometry(0.12, 0.2, 1.3, 6)
  trunkGeometry.translate(0, 0.65, 0)
  const crownGeometry = new THREE.IcosahedronGeometry(0.95, 0)
  const trunkMaterial = new THREE.MeshStandardMaterial({ color: '#7a5a48', roughness: 1, flatShading: true })
  const crownMaterial = new THREE.MeshStandardMaterial({ color: '#4f9a6a', roughness: 0.85, flatShading: true })
  const crownLight = new THREE.MeshStandardMaterial({ color: '#8fcf86', roughness: 0.85, flatShading: true })
  const crystalGeometry = new THREE.OctahedronGeometry(0.32, 0)
  const crystalMaterials = new Map(
    CRYSTAL_COLOURS.map((c) => [c, new THREE.MeshStandardMaterial({ color: c, emissive: c, emissiveIntensity: 0.8, roughness: 0.2, metalness: 0.1, flatShading: true })]),
  )
  const pebbleGeometry = new THREE.IcosahedronGeometry(1, 0)
  const pebbles = []

  for (const [id, isle] of plan.isles) {
    const { spec } = isle
    const [x, y, z] = spec.at
    const group = new THREE.Group()
    group.name = `isle-${id}`
    group.position.set(x, y, z)

    const rock = new THREE.Mesh(rockGeometry(spec.radius, isle.depth, `${seed}:${id}`), rockMaterial)
    rock.position.y = -0.35
    const grass = new THREE.Mesh(new THREE.CylinderGeometry(spec.radius * 1.04, spec.radius * 0.98, 0.7, 28), grassMaterial(isle.grass))
    grass.position.y = -0.35
    group.add(rock, grass)

    // two little stones drifting under each isle
    const r = seeded(`${seed}:${id}:pebbles`)
    for (let n = 0; n < 2; n++) {
      const pebble = new THREE.Mesh(pebbleGeometry, rockMaterial)
      const a = r() * Math.PI * 2
      pebble.scale.setScalar(0.5 + r() * 0.9)
      pebble.position.set(Math.sin(a) * spec.radius * 1.25, -isle.depth * (0.35 + r() * 0.3), Math.cos(a) * spec.radius * 1.25)
      pebble.userData.bob = r() * Math.PI * 2
      pebble.userData.y = pebble.position.y
      pebbles.push(pebble)
      group.add(pebble)
    }
    root.add(group)

    for (const tree of isle.trees) {
      const t = new THREE.Group()
      const trunk = new THREE.Mesh(trunkGeometry, trunkMaterial)
      const crown = new THREE.Mesh(crownGeometry, tree.scale > 1.1 ? crownMaterial : crownLight)
      crown.position.y = 1.75
      crown.scale.set(1, 1.15, 1)
      t.add(trunk, crown)
      t.position.set(tree.x, y, tree.z)
      t.rotation.y = tree.turn
      t.scale.setScalar(tree.scale)
      root.add(t)
    }
    for (const crystal of isle.crystals) {
      const c = new THREE.Mesh(crystalGeometry, crystalMaterials.get(crystal.colour))
      c.scale.set(1, crystal.height * 2.2, 1)
      c.position.set(crystal.x, y + crystal.height * 0.55, crystal.z)
      c.rotation.set(0.12, crystal.turn, 0.08)
      root.add(c)
    }
  }

  // bridges of light: a faint deck, two bright rails and a spark that travels across
  const deckMaterial = new THREE.MeshBasicMaterial({ color: '#8fe6ff', transparent: true, opacity: 0.25, blending: THREE.AdditiveBlending, depthWrite: false, side: THREE.DoubleSide })
  const railMaterial = new THREE.MeshBasicMaterial({ color: '#bff2ff', transparent: true, opacity: 0.85, blending: THREE.AdditiveBlending, depthWrite: false })
  const sparks = []
  for (const [a, b] of bridges) {
    const points = plan.bridge(a, b)
    root.add(new THREE.Mesh(ribbonGeometry(points, 0.9), deckMaterial))
    for (const side of [-1, 1]) {
      const rail = points.map(([px, py, pz], i) => {
        const p = points[Math.max(0, i - 1)]
        const q = points[Math.min(points.length - 1, i + 1)]
        const dx = q[0] - p[0]
        const dz = q[2] - p[2]
        const len = Math.hypot(dx, dz) || 1
        return new THREE.Vector3(px + (dz / len) * 0.9 * side, py + 0.5, pz - (dx / len) * 0.9 * side)
      })
      const curve = new THREE.CatmullRomCurve3(rail)
      root.add(new THREE.Mesh(new THREE.TubeGeometry(curve, points.length * 2, 0.045, 5, false), railMaterial))
    }
    const spark = new THREE.Sprite(new THREE.SpriteMaterial({ map: softDot(), color: '#d8f6ff', transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }))
    spark.scale.setScalar(1.4)
    spark.userData = { curve: new THREE.CatmullRomCurve3(points.map((p) => new THREE.Vector3(p[0], p[1] + 0.25, p[2]))), offset: sparks.length * 0.37 }
    root.add(spark)
    sparks.push(spark)
  }

  // the sea of clouds far below
  const seaUniforms = {
    time: { value: 0 },
    light: { value: new THREE.Color('#f4f0ff') },
    shade: { value: new THREE.Color('#9a90c0') },
    haze: { value: new THREE.Color('#9a8ac0') },
  }
  const sea = new THREE.Mesh(
    new THREE.CircleGeometry(700, 48),
    new THREE.ShaderMaterial({ uniforms: seaUniforms, vertexShader: SEA_VERTEX, fragmentShader: SEA_FRAGMENT, fog: false }),
  )
  sea.rotation.x = -Math.PI / 2
  sea.position.y = SEA_Y
  root.add(sea)

  // motes drifting between the isles: fireflies by night, faint dust by day
  const moteRand = seeded(`${seed}:motes`)
  const MOTES = 220
  const motePositions = new Float32Array(MOTES * 3)
  const moteBase = new Float32Array(MOTES * 3)
  for (let i = 0; i < MOTES; i++) {
    moteBase[i * 3] = (moteRand() - 0.5) * 90
    moteBase[i * 3 + 1] = -6 + moteRand() * 22
    moteBase[i * 3 + 2] = (moteRand() - 0.5) * 90 - 6
  }
  motePositions.set(moteBase)
  const moteGeometry = new THREE.BufferGeometry()
  moteGeometry.setAttribute('position', new THREE.BufferAttribute(motePositions, 3))
  const moteMaterial = new THREE.PointsMaterial({ color: '#fff3c8', size: 0.35, map: softDot(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending })
  root.add(new THREE.Points(moteGeometry, moteMaterial))

  const lightColour = new THREE.Color()
  const WHITE = new THREE.Color('#ffffff')
  const horizon = new THREE.Color()
  const remove = world.onUpdate((dt, t, sky) => {
    const glow = sky?.glow ?? 1
    deckMaterial.opacity = 0.12 + glow * 0.2 + Math.sin(t * 1.3) * 0.03
    railMaterial.opacity = 0.45 + glow * 0.45
    for (const m of crystalMaterials.values()) m.emissiveIntensity = 0.35 + glow * 1.4 + Math.sin(t * 1.7) * 0.1
    for (const spark of sparks) {
      const k = (t * 0.08 + spark.userData.offset) % 2
      spark.position.copy(spark.userData.curve.getPointAt(k < 1 ? k : 2 - k))
      spark.material.opacity = 0.35 + glow * 0.65
    }
    seaUniforms.time.value = t
    if (sky) {
      horizon.set(sky.horizon)
      lightColour.copy(horizon).lerp(WHITE, 0.35 + sky.strength * 0.25)
      seaUniforms.light.value.copy(lightColour)
      seaUniforms.shade.value.set(sky.top).lerp(horizon, 0.45)
      seaUniforms.haze.value.copy(horizon)
    }
    moteMaterial.opacity = 0.15 + glow * 0.75
    for (let i = 0; i < MOTES; i++) {
      motePositions[i * 3] = moteBase[i * 3] + Math.sin(t * 0.21 + i) * 1.2
      motePositions[i * 3 + 1] = moteBase[i * 3 + 1] + Math.sin(t * 0.37 + i * 1.7) * 0.6
      motePositions[i * 3 + 2] = moteBase[i * 3 + 2] + Math.cos(t * 0.19 + i * 0.6) * 1.2
    }
    moteGeometry.attributes.position.needsUpdate = true
    for (const p of pebbles) p.position.y = p.userData.y + Math.sin(t * 0.6 + p.userData.bob) * 0.35
  })

  return {
    ids: layout.map((spec) => spec.id),
    centre: layout.some((spec) => spec.id === CENTRE) ? CENTRE : layout[0].id,
    bridges,
    stand: plan.stand,
    bridge: plan.bridge,
    plan,
    root,
    dispose: remove,
  }
}
