// Shared touches for every 3D world: the soft glowing dot, the sea of clouds far below and the
// motes drifting through the air. Each returns its object(s) and an update to hang on the world's
// loop.
import * as THREE from 'three'
import { seeded } from './random.js'

let dotTexture
export function softDot() {
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

// The sea of clouds at height `y`, coloured by the sky.
export function cloudSea(y = -30, radius = 700) {
  const uniforms = {
    time: { value: 0 },
    light: { value: new THREE.Color('#f4f0ff') },
    shade: { value: new THREE.Color('#9a90c0') },
    haze: { value: new THREE.Color('#9a8ac0') },
  }
  const mesh = new THREE.Mesh(
    new THREE.CircleGeometry(radius, 48),
    new THREE.ShaderMaterial({ uniforms, vertexShader: SEA_VERTEX, fragmentShader: SEA_FRAGMENT, fog: false }),
  )
  mesh.rotation.x = -Math.PI / 2
  mesh.position.y = y
  const light = new THREE.Color()
  const horizon = new THREE.Color()
  const WHITE = new THREE.Color('#ffffff')
  function update(t, sky) {
    uniforms.time.value = t
    if (!sky) return
    horizon.set(sky.horizon)
    light.copy(horizon).lerp(WHITE, 0.35 + sky.strength * 0.25)
    uniforms.light.value.copy(light)
    uniforms.shade.value.set(sky.top).lerp(horizon, 0.45)
    uniforms.haze.value.copy(horizon)
  }
  return { mesh, update }
}

// Motes drifting in a box `spread` wide between heights low..high: fireflies by night, faint
// dust by day.
export function motes({ count = 220, spread = 90, low = -6, high = 16, centreZ = -6, seed = 7, size = 0.35 } = {}) {
  const rand = seeded(`${seed}:motes`)
  const positions = new Float32Array(count * 3)
  const base = new Float32Array(count * 3)
  for (let i = 0; i < count; i++) {
    base[i * 3] = (rand() - 0.5) * spread
    base[i * 3 + 1] = low + rand() * (high - low)
    base[i * 3 + 2] = (rand() - 0.5) * spread + centreZ
  }
  positions.set(base)
  const geometry = new THREE.BufferGeometry()
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3))
  const material = new THREE.PointsMaterial({ color: '#fff3c8', size, map: softDot(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending })
  const points = new THREE.Points(geometry, material)
  const drift = spread / 75
  function update(t, glow = 1) {
    material.opacity = 0.15 + glow * 0.75
    for (let i = 0; i < count; i++) {
      positions[i * 3] = base[i * 3] + Math.sin(t * 0.21 + i) * drift
      positions[i * 3 + 1] = base[i * 3 + 1] + Math.sin(t * 0.37 + i * 1.7) * drift * 0.5
      positions[i * 3 + 2] = base[i * 3 + 2] + Math.cos(t * 0.19 + i * 0.6) * drift
    }
    geometry.attributes.position.needsUpdate = true
  }
  return { points, update }
}
