// Deep Space - recipe by Ashran Helmi · ashranhelmi.com
// (Space Playbook Ch.1 "Nebula & Starfield"), written again in our own words for three 0.172.
//
// A nebula and its stars, drawn entirely by code - no pictures, no pieces, nothing to download.
// Four layers:
//   nebula   coloured gas around a tilted band, dust lanes and thin bright threads - painted ONCE
//            into a cube map and used as scene.background, so it costs nothing per frame
//   fog      soft clouds nearer than the sky; they slide against it while the camera orbits
//   stars    thousands of far points at a fixed pixel size (sharp at any zoom)
//   glitter  a few hundred four-point sparkles; each is dark most of the time and flashes briefly
import * as THREE from 'three'

// ---------- the pure parts (tested without a GPU) ----------

// [deep, middle, bright] in sRGB; the shader mixes them along a stretched hue noise.
export const PALETTES = {
  kociverse: { name: 'Kociverse', colours: ['#2a46d6', '#7a3cc8', '#e86ab8'], fog: '#8a6ad8', glitter: '#fff2e0' },
  ember: { name: 'Ember', colours: ['#6e140e', '#c4471a', '#f0a040'], fog: '#d0703a', glitter: '#fff0d0' },
  teal: { name: 'Teal', colours: ['#0c4652', '#1d8a8a', '#7fd0c0'], fog: '#3aa8a0', glitter: '#e8fff8' },
}

// The soft shoulder: bright gas rolls off instead of clipping to a flat blob.
export const toneCurve = (c) => (c * 1.7) / (1 + c * 1.9)

// A sparkle's brightness at time t: dark most of the time, a brief flash (the power keeps it rare).
export const sparkle = (t, speed, offset) => Math.pow(Math.max(0, Math.sin(t * speed + offset)), 14)

// The drag's glide: slows down and STOPS (a glide that creeps forever is snapped to zero).
export const GLIDE_STOP = 1e-4
export function glideStep(velocity, dt, friction = 2.5) {
  const next = velocity * Math.exp(-friction * dt)
  return Math.abs(next) < GLIDE_STOP ? 0 : next
}

// A small seeded random, so the same stars come back every time.
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

// The band the gas and the stars gather around: a great circle tilted across the sky.
export const BAND_AXIS = new THREE.Vector3(0.32, 1, 0.22).normalize()
const bandWeight = (dir, width) => Math.exp(-Math.pow(dir.dot(BAND_AXIS) / width, 2))

// n directions, more of them near the band (base = the share spread evenly over the whole sky).
export function bandDirections(n, rand, { width = 0.25, base = 0.3 } = {}) {
  const out = []
  const dir = new THREE.Vector3()
  while (out.length < n) {
    const z = rand() * 2 - 1
    const a = rand() * Math.PI * 2
    const r = Math.sqrt(1 - z * z)
    dir.set(r * Math.cos(a), z, r * Math.sin(a))
    if (rand() < base + (1 - base) * bandWeight(dir, width)) out.push(dir.clone())
  }
  return out
}

// ---------- the nebula, painted once ----------

const PAINT_VERTEX = `
varying vec3 vDir;
void main() {
  vDir = position;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`

// 3D value noise, fbm with 7 octaves, a two-step domain warp; colours along a stretched hue noise
// (fbm sits near 0.5, so it is stretched with smoothstep(0.4, 0.6) or only the middle colour shows);
// a bright band and a wider soft band; dark dust lanes; thin bright threads; saturation 1.3; the soft
// tone curve; a little dither against banding.
const PAINT_FRAGMENT = `
uniform vec3 deep;
uniform vec3 middle;
uniform vec3 bright;
uniform vec3 axis;
uniform float brightness;
uniform float seed;
varying vec3 vDir;

float hash3(vec3 p) {
  p = fract(p * 0.3183099 + 0.1);
  p *= 17.0;
  return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}
float valueNoise(vec3 x) {
  vec3 i = floor(x);
  vec3 f = fract(x);
  f = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(mix(hash3(i), hash3(i + vec3(1.0, 0.0, 0.0)), f.x), mix(hash3(i + vec3(0.0, 1.0, 0.0)), hash3(i + vec3(1.0, 1.0, 0.0)), f.x), f.y),
    mix(mix(hash3(i + vec3(0.0, 0.0, 1.0)), hash3(i + vec3(1.0, 0.0, 1.0)), f.x), mix(hash3(i + vec3(0.0, 1.0, 1.0)), hash3(i + vec3(1.0, 1.0, 1.0)), f.x), f.y),
    f.z);
}
float fbm(vec3 p) {
  float v = 0.0;
  float amp = 0.5;
  for (int i = 0; i < 7; i++) {
    v += amp * valueNoise(p);
    p = p * 2.03 + vec3(1.7, 9.2, 3.1);
    amp *= 0.5;
  }
  return v;
}
void main() {
  vec3 d = normalize(vDir);
  vec3 p = d * 2.4 + vec3(seed);
  // the two-step warp
  vec3 q = vec3(fbm(p), fbm(p + vec3(5.2, 1.3, 2.8)), fbm(p + vec3(1.7, 9.2, 4.4)));
  vec3 r = vec3(fbm(p + 4.0 * q + vec3(1.7, 9.2, 0.0)), fbm(p + 4.0 * q + vec3(8.3, 2.8, 1.1)), 0.0);
  float n = fbm(p + 4.0 * vec3(r.xy, r.x));

  float lat = dot(d, axis);
  float band = exp(-pow(lat / 0.17, 2.0));
  float soft = exp(-pow(lat / 0.42, 2.0));
  // the sky stays dark away from the band: gas only where the noise runs high
  float dense = smoothstep(0.32, 0.78, n);
  float gas = dense * dense * (0.04 + 1.25 * band + 0.35 * soft);
  // dark dust lanes through the bright band
  float dust = smoothstep(0.42, 0.68, fbm(p * 2.3 + vec3(r.xy * 2.0, 0.0)));
  gas *= 1.0 - 0.8 * dust * band;
  // thin bright threads
  float threadNoise = fbm(p * 3.1 + q * 1.3);
  float threads = pow(1.0 - abs(2.0 * threadNoise - 1.0), 18.0) * (0.08 + band) * dense;

  float hue = smoothstep(0.4, 0.6, fbm(p * 0.7 + q * 1.5));
  vec3 colour = mix(deep, middle, smoothstep(0.0, 0.6, hue));
  colour = mix(colour, bright, smoothstep(0.5, 1.0, hue));

  vec3 c = colour * gas * 1.3 + mix(colour, bright, 0.5) * threads * 0.45 + bright * band * band * 0.06 * dense;
  float luma = dot(c, vec3(0.2126, 0.7152, 0.0722));
  c = max(mix(vec3(luma), c, 1.3), 0.0);
  c *= brightness;
  c = c * 1.7 / (1.0 + c * 1.9);
  c += (hash3(d * 1543.0) - 0.5) / 255.0;
  gl_FragColor = vec4(max(c, 0.0), 1.0);
}`

// ---------- the glitter ----------

const GLITTER_VERTEX = `
attribute float speed;
attribute float phase;
attribute float spark;
uniform float time;
uniform float pixel;
varying float vGlow;
void main() {
  float s = max(0.0, sin(time * speed + phase));
  vGlow = pow(s, 14.0);
  gl_PointSize = spark * pixel * (0.6 + 0.4 * vGlow);
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`

// a four-point star: two thin crossed rays and a soft core
const GLITTER_FRAGMENT = `
uniform vec3 tint;
varying float vGlow;
void main() {
  vec2 c = gl_PointCoord * 2.0 - 1.0;
  float rays = max(exp(-abs(c.x) * 22.0) * (1.0 - abs(c.y)), exp(-abs(c.y) * 22.0) * (1.0 - abs(c.x)));
  float core = exp(-dot(c, c) * 9.0);
  float glow = (rays + core) * vGlow;
  if (glow < 0.004) discard;
  gl_FragColor = vec4(tint * glow, glow);
}`

function fogTexture() {
  const size = 128
  const canvas = document.createElement('canvas')
  canvas.width = canvas.height = size
  const g = canvas.getContext('2d')
  const grad = g.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2)
  grad.addColorStop(0, 'rgba(255,255,255,0.55)')
  grad.addColorStop(0.45, 'rgba(255,255,255,0.18)')
  grad.addColorStop(1, 'rgba(255,255,255,0)')
  g.fillStyle = grad
  g.fillRect(0, 0, size, size)
  const texture = new THREE.CanvasTexture(canvas)
  texture.colorSpace = THREE.SRGBColorSpace
  return texture
}

const linear = (hex) => new THREE.Color(hex)

// world: { renderer, scene, onUpdate(fn), onDispose?(fn) } (DeepSpace.vue makes one).
// Returns { update(dt, t), set({ palette, brightness }), dispose() }.
export function createSpaceBackdrop(world, { palette = 'kociverse', brightness = 0.55, cubeSize, seed = 7 } = {}) {
  const { renderer, scene } = world
  const small = Math.min(window.screen?.width ?? 1920, window.screen?.height ?? 1080) < 900
  const size = cubeSize ?? (small ? 768 : 1536)
  const reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches ?? false
  let colours = PALETTES[palette] ?? PALETTES.kociverse
  let shine = brightness
  const rand = seeded(seed)

  // the nebula: one sphere painted into a cube map, then thrown away
  const target = new THREE.WebGLCubeRenderTarget(size, { type: THREE.HalfFloatType, generateMipmaps: false })
  const cubeCamera = new THREE.CubeCamera(0.1, 10, target)
  const paintUniforms = {
    deep: { value: new THREE.Color() },
    middle: { value: new THREE.Color() },
    bright: { value: new THREE.Color() },
    axis: { value: BAND_AXIS.clone() },
    brightness: { value: shine },
    seed: { value: seed * 1.37 },
  }
  function paint() {
    const [d, m, b] = colours.colours
    paintUniforms.deep.value.copy(linear(d))
    paintUniforms.middle.value.copy(linear(m))
    paintUniforms.bright.value.copy(linear(b))
    paintUniforms.brightness.value = shine
    const paintScene = new THREE.Scene()
    const sphere = new THREE.Mesh(
      new THREE.SphereGeometry(5, 64, 32),
      new THREE.ShaderMaterial({ uniforms: paintUniforms, vertexShader: PAINT_VERTEX, fragmentShader: PAINT_FRAGMENT, side: THREE.BackSide, depthWrite: false }),
    )
    paintScene.add(sphere)
    const toneMapping = renderer.toneMapping
    renderer.toneMapping = THREE.NoToneMapping
    cubeCamera.update(renderer, paintScene)
    renderer.toneMapping = toneMapping
    sphere.geometry.dispose()
    sphere.material.dispose()
    scene.background = target.texture
  }
  paint()

  // the stars: far, fixed pixel size, a little warm or cool
  const starDirs = bandDirections(3800, rand)
  const starPos = new Float32Array(starDirs.length * 3)
  const starCol = new Float32Array(starDirs.length * 3)
  const warm = new THREE.Color('#ffe2c4')
  const cool = new THREE.Color('#cfdcff')
  const tmp = new THREE.Color()
  starDirs.forEach((d, i) => {
    d.multiplyScalar(520 + rand() * 60).toArray(starPos, i * 3)
    tmp.copy(warm).lerp(cool, rand()).multiplyScalar(0.45 + rand() * 0.55)
    tmp.toArray(starCol, i * 3)
  })
  const starGeo = new THREE.BufferGeometry()
  starGeo.setAttribute('position', new THREE.BufferAttribute(starPos, 3))
  starGeo.setAttribute('color', new THREE.BufferAttribute(starCol, 3))
  const stars = new THREE.Points(
    starGeo,
    new THREE.PointsMaterial({ size: 1.4, sizeAttenuation: false, vertexColors: true, transparent: true, depthWrite: false, fog: false }),
  )
  scene.add(stars)

  // the fog: soft clouds between the sky and the camera, around the band
  const fogMap = fogTexture()
  const fog = []
  for (const d of bandDirections(38, rand, { width: 0.2, base: 0 })) {
    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({ map: fogMap, color: colours.fog, transparent: true, opacity: 0.05 + rand() * 0.07, blending: THREE.AdditiveBlending, depthWrite: false, fog: false }),
    )
    sprite.position.copy(d.multiplyScalar(110 + rand() * 120))
    const s = 70 + rand() * 110
    sprite.scale.set(s, s * (0.6 + rand() * 0.4), 1)
    sprite.material.rotation = rand() * Math.PI * 2
    fog.push(sprite)
    scene.add(sprite)
  }

  // the glitter: 320 sparkles, each with its own speed and phase
  const glitterDirs = bandDirections(320, rand, { width: 0.3, base: 0.35 })
  const gPos = new Float32Array(glitterDirs.length * 3)
  const gSpeed = new Float32Array(glitterDirs.length)
  const gPhase = new Float32Array(glitterDirs.length)
  const gSpark = new Float32Array(glitterDirs.length)
  glitterDirs.forEach((d, i) => {
    d.multiplyScalar(440 + rand() * 40).toArray(gPos, i * 3)
    gSpeed[i] = 0.25 + rand() * 0.55
    gPhase[i] = rand() * Math.PI * 2
    gSpark[i] = 9 + rand() * 13
  })
  const gGeo = new THREE.BufferGeometry()
  gGeo.setAttribute('position', new THREE.BufferAttribute(gPos, 3))
  gGeo.setAttribute('speed', new THREE.BufferAttribute(gSpeed, 1))
  gGeo.setAttribute('phase', new THREE.BufferAttribute(gPhase, 1))
  gGeo.setAttribute('spark', new THREE.BufferAttribute(gSpark, 1))
  const glitterUniforms = {
    time: { value: reducedMotion ? 1.3 : 0 },
    pixel: { value: renderer.getPixelRatio() },
    tint: { value: linear(colours.glitter) },
  }
  const glitter = new THREE.Points(
    gGeo,
    new THREE.ShaderMaterial({ uniforms: glitterUniforms, vertexShader: GLITTER_VERTEX, fragmentShader: GLITTER_FRAGMENT, transparent: true, blending: THREE.AdditiveBlending, depthWrite: false }),
  )
  scene.add(glitter)

  const api = {
    update(dt, t) {
      // reduced motion: the sparkles hold still
      if (!reducedMotion) glitterUniforms.time.value = t
    },
    // a new palette or brightness repaints the sky once
    set({ palette: next, brightness: b } = {}) {
      if (next && PALETTES[next]) colours = PALETTES[next]
      if (typeof b === 'number') shine = b
      for (const sprite of fog) sprite.material.color.set(colours.fog)
      glitterUniforms.tint.value.copy(linear(colours.glitter))
      paint()
    },
    dispose() {
      target.dispose()
      fogMap.dispose()
    },
  }
  world.onUpdate((dt, t) => api.update(dt, t))
  world.onDispose?.(() => api.dispose())
  return api
}

// ---------- the camera: an orbit around the centre that you can drag ----------

// The camera MOVES around the centre (it does not only turn), so the fog slides against the sky;
// a drag pushes it, the push glides down and stops; the wheel zooms.
export class SpaceOrbit {
  constructor(world, { canvas, mode = 'orbit', onShot } = {}) {
    this.world = world
    this.canvas = canvas
    this.yaw = Math.random() * Math.PI * 2
    this.pitch = 0.12
    this.spin = 0
    this.tilt = 0
    this.setMode(mode)
    this.radius = this.wanted
    this.dragging = null
    onShot?.(this.mode)
    world.onUpdate((dt) => this.update(dt))
    if (canvas) this.listen(canvas)
  }

  setMode(mode) {
    this.mode = mode === 'wide' ? 'wide' : 'orbit'
    this.wanted = this.mode === 'wide' ? 58 : 34
    this.drift = this.mode === 'wide' ? 0.008 : 0.016
  }

  setSettings() {}

  listen(canvas) {
    this.handlers = {
      pointerdown: (e) => {
        this.dragging = { x: e.clientX, y: e.clientY, at: performance.now() }
        canvas.setPointerCapture?.(e.pointerId)
      },
      pointermove: (e) => {
        if (!this.dragging) return
        const now = performance.now()
        const dt = Math.max(0.008, (now - this.dragging.at) / 1000)
        const dx = e.clientX - this.dragging.x
        const dy = e.clientY - this.dragging.y
        this.yaw -= dx * 0.004
        this.pitch = Math.max(-0.7, Math.min(0.7, this.pitch + dy * 0.003))
        this.spin = (-dx * 0.004) / dt
        this.tilt = (dy * 0.003) / dt
        this.dragging = { x: e.clientX, y: e.clientY, at: now }
      },
      pointerup: () => (this.dragging = null),
      pointercancel: () => (this.dragging = null),
      wheel: (e) => {
        e.preventDefault()
        this.wanted = Math.max(14, Math.min(80, this.wanted * (1 + e.deltaY * 0.001)))
      },
    }
    for (const [name, fn] of Object.entries(this.handlers)) canvas.addEventListener(name, fn, { passive: name !== 'wheel' })
  }

  update(dt) {
    if (!this.dragging) {
      this.yaw += (this.drift + this.spin) * dt
      this.pitch = Math.max(-0.7, Math.min(0.7, this.pitch + this.tilt * dt))
      this.spin = glideStep(this.spin, dt)
      this.tilt = glideStep(this.tilt, dt)
    }
    this.radius += (this.wanted - this.radius) * Math.min(1, dt * 3)
    const camera = this.world.camera
    const c = Math.cos(this.pitch)
    camera.position.set(Math.sin(this.yaw) * c * this.radius, Math.sin(this.pitch) * this.radius, Math.cos(this.yaw) * c * this.radius)
    camera.lookAt(0, 0, 0)
  }
}
