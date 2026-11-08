// The sky over the WORLD, from the real clock: colours, the sun and the moon (with its real
// phase), how bright the light is, whether the stars show and how much the bridges glow.
// `skyAt` and `moonPhase` are pure and tested without a GPU; `createSkyDome` paints them.
import * as THREE from 'three'

// [hour, top, horizon, light colour, light strength, ambient, stars 0..1, glow 0..1]
const KEYS = [
  [0, '#070b1f', '#1a2246', '#8ea8ff', 0.3, 0.32, 1, 1],
  [4.75, '#0a0f28', '#232a52', '#8ea8ff', 0.3, 0.32, 1, 1],
  [6, '#2d3570', '#e3a184', '#ffbf94', 0.6, 0.45, 0.35, 0.6],
  [7.25, '#5f8ad0', '#f2d2ae', '#ffe6c8', 1.05, 0.6, 0, 0.15],
  [12, '#4f8fe0', '#c4e0f6', '#fff8ee', 1.45, 0.75, 0, 0.1],
  [16.75, '#5d88d2', '#efd6b8', '#ffe8cc', 1.2, 0.66, 0, 0.15],
  [18.25, '#4c3f8e', '#ef9c7c', '#ffa27a', 0.78, 0.5, 0.15, 0.55],
  [19.5, '#211d58', '#6c4e8e', '#b48cff', 0.48, 0.4, 0.6, 0.9],
  [21, '#0c1030', '#2a2858', '#98a8ff', 0.34, 0.34, 1, 1],
  [24, '#070b1f', '#1a2246', '#8ea8ff', 0.3, 0.32, 1, 1],
]

const SUNRISE = 6
const SUNSET = 18.5
const SYNODIC_DAYS = 29.530588853
const KNOWN_NEW_MOON = Date.UTC(2000, 0, 6, 18, 14)
const MOON_LOW = '#ffd2a0'
const MOON_HIGH = '#eef2ff'

const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16))
const toHex = (rgb) => '#' + rgb.map((v) => Math.round(v).toString(16).padStart(2, '0')).join('')
const mix = (a, b, t) => a + (b - a) * t
export const mixColour = (a, b, t) => {
  const B = hex(b)
  return toHex(hex(a).map((v, i) => mix(v, B[i], t)))
}
const clamp01 = (v) => Math.min(1, Math.max(0, v))

// The moon's age in the cycle: 0 = new, 0.5 = full, back toward 1 = new again.
export function moonPhase(date = new Date()) {
  const days = (date.getTime() - KNOWN_NEW_MOON) / 86400000
  const p = (days / SYNODIC_DAYS) % 1
  return p < 0 ? p + 1 : p
}

// The sky at a moment:
// { hour, top, horizon, light, strength, ambient, stars, glow, night,
//   sun: { height, around }, moon: { height, around, show, colour, phase, lit } }
// height = 0 at the horizon .. 1 overhead; around = 0 rising .. 1 setting.
export function skyAt(date = new Date()) {
  const hour = date.getHours() + date.getMinutes() / 60 + date.getSeconds() / 3600
  let i = KEYS.findIndex((k, n) => n < KEYS.length - 1 && hour >= k[0] && hour < KEYS[n + 1][0])
  if (i < 0) i = KEYS.length - 2
  const [h0, top0, hor0, light0, s0, a0, st0, g0] = KEYS[i]
  const [h1, top1, hor1, light1, s1, a1, st1, g1] = KEYS[i + 1]
  const t = (hour - h0) / (h1 - h0)

  const night = !(hour >= SUNRISE && hour < SUNSET)
  const dayLength = SUNSET - SUNRISE
  const sunAround = clamp01((hour - SUNRISE) / dayLength)
  const moonAround = clamp01(((hour - SUNSET + 24) % 24) / (24 - dayLength))
  const sunHeight = night ? 0 : Math.sin(Math.PI * sunAround)
  const moonHeight = night ? Math.sin(Math.PI * moonAround) : 0
  const phase = moonPhase(date)

  return {
    hour,
    top: mixColour(top0, top1, t),
    horizon: mixColour(hor0, hor1, t),
    light: mixColour(light0, light1, t),
    strength: mix(s0, s1, t),
    ambient: mix(a0, a1, t),
    stars: mix(st0, st1, t),
    glow: mix(g0, g1, t),
    night,
    sun: { height: sunHeight, around: sunAround },
    moon: {
      height: moonHeight,
      around: moonAround,
      // it fades in as it clears the horizon, and out again as it sets
      show: night ? clamp01(moonHeight * 5) : 0,
      colour: mixColour(MOON_LOW, MOON_HIGH, clamp01(moonHeight * 1.6)),
      phase,
      lit: (1 - Math.cos(2 * Math.PI * phase)) / 2,
    },
  }
}

// A direction on the dome for a body `around` its arc at `height`, kept in front of the
// camera's home view (which looks toward -z).
export function arcDirection(around, height) {
  const x = -Math.cos(Math.PI * around) * 0.9
  const y = 0.06 + height * 0.8
  const z = -1
  const len = Math.hypot(x, y, z)
  return [x / len, y / len, z / len]
}

const SKY_VERTEX = `
varying vec3 vDir;
void main() {
  vDir = normalize(position);
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}`

const SKY_FRAGMENT = `
uniform vec3 top;
uniform vec3 horizon;
uniform float stars;
uniform float time;
uniform vec3 moonDir;
uniform vec3 moonColour;
uniform float moonShow;
uniform float moonPhase;
varying vec3 vDir;
float hash(vec3 p) { return fract(sin(dot(p, vec3(12.9898, 78.233, 45.164))) * 43758.5453); }
void main() {
  vec3 dir = normalize(vDir);
  float h = clamp(dir.y * 1.4 + 0.15, 0.0, 1.0);
  vec3 col = mix(horizon, top, pow(h, 0.8));

  // stars: fixed points on the dome, twinkling softly, hidden near the horizon haze
  vec3 cell = floor(dir * 240.0);
  float s = step(0.9972, hash(cell)) * smoothstep(0.04, 0.32, dir.y);
  float tw = 0.55 + 0.45 * sin(time * 1.7 + hash(cell + 3.1) * 40.0);
  col += vec3(s * tw * stars);

  // the moon: a lit sphere showing its real phase, with a soft halo
  float radius = 0.045;
  float d = dot(dir, moonDir);
  vec3 right = normalize(cross(moonDir, vec3(0.0, 1.0, 0.0)));
  vec3 up = cross(right, moonDir);
  vec2 uv = vec2(dot(dir, right), dot(dir, up)) / radius;
  float r2 = dot(uv, uv);
  float disc = (1.0 - smoothstep(0.92, 1.0, sqrt(r2))) * step(0.0, d);
  float angle = moonPhase * 6.2831853;
  vec3 light = vec3(-sin(angle), 0.0, -cos(angle));
  vec3 n = vec3(uv, sqrt(max(0.0, 1.0 - r2)));
  float lit = smoothstep(-0.05, 0.12, dot(n, light));
  vec3 moon = moonColour * (0.06 + 0.94 * lit);
  float halo = exp(-(1.0 - d) * 900.0) * 0.35 + exp(-(1.0 - d) * 120.0) * 0.12;
  col = mix(col, moon, disc * moonShow);
  col += moonColour * halo * moonShow * (1.0 - disc);

  gl_FragColor = vec4(col, 1.0);
  #include <colorspace_fragment>
}`

// The dome: a big inside-out sphere that follows the camera, so it is never clipped.
export function createSkyDome() {
  const uniforms = {
    top: { value: new THREE.Color() },
    horizon: { value: new THREE.Color() },
    stars: { value: 0 },
    time: { value: 0 },
    moonDir: { value: new THREE.Vector3(0, 0.5, -1).normalize() },
    moonColour: { value: new THREE.Color('#eef2ff') },
    moonShow: { value: 0 },
    moonPhase: { value: 0.5 },
  }
  const mesh = new THREE.Mesh(
    new THREE.SphereGeometry(500, 32, 16),
    new THREE.ShaderMaterial({
      uniforms,
      vertexShader: SKY_VERTEX,
      fragmentShader: SKY_FRAGMENT,
      side: THREE.BackSide,
      depthWrite: false,
      fog: false,
    }),
  )
  mesh.frustumCulled = false
  mesh.renderOrder = -1

  return {
    mesh,
    apply(sky) {
      uniforms.top.value.set(sky.top)
      uniforms.horizon.value.set(sky.horizon)
      uniforms.stars.value = sky.stars
      uniforms.moonDir.value.set(...arcDirection(sky.moon.around, sky.moon.height))
      uniforms.moonColour.value.set(sky.moon.colour)
      uniforms.moonShow.value = sky.moon.show
      uniforms.moonPhase.value = sky.moon.phase
    },
    tick(time, cameraPosition) {
      uniforms.time.value = time
      if (cameraPosition) mesh.position.copy(cameraPosition)
    },
  }
}
