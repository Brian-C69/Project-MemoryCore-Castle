// The companion's avatar in the WORLD: a glowing crystal (the companion has no 3D body). It moves
// like a character - `play('idle' | 'walk' | 'wave')` - only faked by motion: a gentle bob, a lean
// with a quicker bob, a spinning pulse. It brightens while speaking and dims while resting.
import * as THREE from 'three'

const HOVER = 1.35 // metres above the ground the crystal floats
const WAVE_S = 1.8

// Each look the crystal eases toward: bob height and speed, lean forward, spin, glow.
const POSES = {
  idle: { bob: 0.12, bobSpeed: 1.3, lean: 0, spin: 0.45 },
  walk: { bob: 0.2, bobSpeed: 3.2, lean: 0.28, spin: 1.1 },
}

let haloTexture
function halo() {
  if (!haloTexture) {
    const c = document.createElement('canvas')
    c.width = c.height = 128
    const g = c.getContext('2d')
    const grad = g.createRadialGradient(64, 64, 0, 64, 64, 64)
    grad.addColorStop(0, 'rgba(255,255,255,0.9)')
    grad.addColorStop(0.25, 'rgba(255,255,255,0.35)')
    grad.addColorStop(1, 'rgba(255,255,255,0)')
    g.fillStyle = grad
    g.fillRect(0, 0, 128, 128)
    haloTexture = new THREE.CanvasTexture(c)
    haloTexture.colorSpace = THREE.SRGBColorSpace
  }
  return haloTexture
}

const approach = (value, target, rate, dt) => value + (target - value) * Math.min(1, rate * dt)

export class Crystal {
  constructor({ colour = '#9fe8ff', size = 1 } = {}) {
    this.colour = new THREE.Color(colour)
    // group = the feet (on the ground, turned to the heading); body = the floating crystal
    this.group = new THREE.Group()
    this.group.name = 'crystal'
    this.body = new THREE.Group()
    this.body.position.y = HOVER
    this.group.add(this.body)

    this.coreMaterial = new THREE.MeshPhysicalMaterial({
      color: this.colour.clone().lerp(new THREE.Color('#ffffff'), 0.4),
      emissive: this.colour,
      emissiveIntensity: 0.6,
      transmission: 0.85,
      thickness: 0.9,
      ior: 1.6,
      roughness: 0.06,
      metalness: 0,
      clearcoat: 1,
      clearcoatRoughness: 0.05,
      flatShading: true,
    })
    this.core = new THREE.Mesh(new THREE.IcosahedronGeometry(0.55 * size, 0), this.coreMaterial)
    this.core.scale.set(1, 1.35, 1)

    this.heartMaterial = new THREE.MeshBasicMaterial({ color: this.colour.clone().lerp(new THREE.Color('#ffffff'), 0.6), transparent: true, opacity: 0.9, blending: THREE.AdditiveBlending, depthWrite: false })
    this.heart = new THREE.Mesh(new THREE.IcosahedronGeometry(0.2 * size, 1), this.heartMaterial)

    this.haloMaterial = new THREE.SpriteMaterial({ map: halo(), color: this.colour, transparent: true, opacity: 0.55, depthWrite: false, blending: THREE.AdditiveBlending })
    this.halo = new THREE.Sprite(this.haloMaterial)
    this.halo.scale.setScalar(3 * size)

    this.light = new THREE.PointLight(this.colour, 3, 10, 1.6)

    // three small shards circling the crystal
    this.shards = new THREE.Group()
    const shardGeometry = new THREE.OctahedronGeometry(0.09 * size, 0)
    for (let i = 0; i < 3; i++) {
      const shard = new THREE.Mesh(shardGeometry, this.heartMaterial)
      const a = (i / 3) * Math.PI * 2
      shard.position.set(Math.sin(a) * 0.95 * size, (i - 1) * 0.18, Math.cos(a) * 0.95 * size)
      shard.scale.set(1, 1.8, 1)
      this.shards.add(shard)
    }
    this.body.add(this.core, this.heart, this.halo, this.light, this.shards)

    this.base = 'idle'
    this.speaking = false
    this.resting = false
    this.waveLeft = 0
    this.now = { ...POSES.idle, glow: 0.6, hover: HOVER }
    this.phase = 0
    this.spinAngle = 0
  }

  get position() {
    return this.group.position
  }
  get heading() {
    return this.group.rotation.y
  }
  set heading(value) {
    this.group.rotation.y = value
  }
  get walking() {
    return this.base === 'walk'
  }
  get waving() {
    return this.waveLeft > 0
  }

  // 'idle' and 'walk' are kept until changed; 'wave' plays once and returns to the last one.
  play(name) {
    if (name === 'wave') this.waveLeft = WAVE_S
    else if (POSES[name]) this.base = name
  }

  setSpeaking(on) {
    this.speaking = !!on
  }

  setResting(on) {
    this.resting = !!on
  }

  update(dt, t) {
    const pose = POSES[this.base]
    const resting = this.resting && !this.speaking
    const target = {
      bob: resting ? 0.05 : pose.bob,
      bobSpeed: resting ? 0.6 : this.speaking ? pose.bobSpeed * 1.4 : pose.bobSpeed,
      lean: pose.lean,
      spin: resting ? 0.12 : this.speaking ? pose.spin * 1.8 : pose.spin,
      glow: this.speaking ? 1.7 : resting ? 0.15 : 0.6,
      hover: resting ? HOVER * 0.7 : HOVER,
    }
    for (const key of Object.keys(target)) this.now[key] = approach(this.now[key], target[key], 3, dt)
    const now = this.now

    // the bob runs on its own phase so changing speed never jumps
    this.phase += dt * now.bobSpeed * Math.PI * 2
    let spin = now.spin
    let pulse = 1
    if (this.waveLeft > 0) {
      this.waveLeft = Math.max(0, this.waveLeft - dt)
      const k = this.waveLeft / WAVE_S // 1 -> 0
      spin += 9 * k * k
      pulse = 1 + Math.sin((1 - k) * Math.PI) * 0.16
    }
    this.spinAngle += spin * dt

    this.body.position.y = now.hover + Math.sin(this.phase) * now.bob
    this.body.rotation.x = now.lean
    this.core.rotation.y = this.spinAngle
    this.core.scale.set(pulse, 1.35 * pulse, pulse)
    this.shards.rotation.y = -this.spinAngle * 0.7
    this.shards.position.y = Math.sin(this.phase * 0.5) * 0.06

    // speaking flickers softly, like a voice
    const flicker = this.speaking ? 0.25 * Math.sin(t * 11) + 0.15 * Math.sin(t * 17.3) : 0
    const glow = Math.max(0.05, now.glow + flicker + (pulse - 1) * 3)
    this.coreMaterial.emissiveIntensity = glow
    this.heartMaterial.opacity = Math.min(1, 0.35 + glow * 0.4)
    this.haloMaterial.opacity = Math.min(0.9, 0.15 + glow * 0.35)
    this.halo.scale.setScalar(2.4 + glow * 0.8)
    this.light.intensity = 0.6 + glow * 2.6
  }

  dispose() {
    this.group.traverse((o) => {
      o.geometry?.dispose?.()
      o.material?.dispose?.()
    })
    this.light.dispose?.()
    this.group.removeFromParent()
  }
}
