// A centrepiece world (Crystal Isle, or one a member creates): one great base isle, the
// companion's piece standing at its heart and glowing, islets circling slowly round it, sparkles
// rising, the sea of clouds below. The companion piece wears the Crystal's outward shape
// (position, heading, walking, resting, play, setSpeaking, setResting, update) so the camera rig
// films it the same way.
import * as THREE from 'three'
import { cloudSea, motes, softDot } from './effects.js'
import { orbitAt, surfaceY } from './glb.js'

const approach = (from, to, rate, dt) => from + (to - from) * Math.min(1, rate * dt)

// The companion's piece, lit from within: its materials take an emissive glow in the world's
// colour, a halo sprite and a point light sit at its heart.
export class GlowingPiece {
  constructor(model, { colour = '#8fe6ff', height = 9 } = {}) {
    this.group = new THREE.Group()
    this.model = model
    this.height = height
    this.group.add(model)
    this.colour = new THREE.Color(colour)
    this.materials = []
    model.traverse((node) => {
      if (!node.isMesh) return
      node.material = [node.material].flat().map((m) => {
        const glowing = m.clone()
        if ('emissive' in glowing) {
          glowing.emissive = this.colour.clone()
          glowing.emissiveIntensity = 0.6
          if (!glowing.emissiveMap && glowing.map) glowing.emissiveMap = glowing.map
        }
        this.materials.push(glowing)
        return glowing
      })
      if (node.material.length === 1) node.material = node.material[0]
    })
    this.haloMaterial = new THREE.SpriteMaterial({ map: softDot(), color: this.colour, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending })
    this.halo = new THREE.Sprite(this.haloMaterial)
    this.halo.position.y = height * 0.55
    this.light = new THREE.PointLight(this.colour, 2, height * 6, 1.6)
    this.light.position.y = height * 0.6
    this.group.add(this.halo, this.light)
    this.speaking = false
    this.resting = false
    this.waveLeft = 0
    this.glow = 0.6
    this.phase = 0
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
    return false
  }

  play(name) {
    if (name === 'wave') this.waveLeft = 2
  }
  setSpeaking(on) {
    this.speaking = !!on
  }
  setResting(on) {
    this.resting = !!on
  }

  update(dt, t) {
    const resting = this.resting && !this.speaking
    this.glow = approach(this.glow, this.speaking ? 1.6 : resting ? 0.2 : 0.65, 3, dt)
    let pulse = 0
    if (this.waveLeft > 0) {
      this.waveLeft = Math.max(0, this.waveLeft - dt)
      pulse = Math.sin((1 - this.waveLeft / 2) * Math.PI) * 0.8
    }
    const flicker = this.speaking ? 0.25 * Math.sin(t * 11) + 0.15 * Math.sin(t * 17.3) : 0.08 * Math.sin(t * 1.3)
    const glow = Math.max(0.05, this.glow + flicker + pulse)
    for (const m of this.materials) if ('emissiveIntensity' in m) m.emissiveIntensity = glow
    this.haloMaterial.opacity = Math.min(0.85, 0.2 + glow * 0.35)
    this.halo.scale.setScalar(this.height * (0.9 + glow * 0.35))
    this.light.intensity = 1 + glow * 4
    // the piece turns very slowly and breathes a little
    this.phase += dt
    this.model.rotation.y += dt * (resting ? 0.01 : 0.04)
    this.model.position.y = Math.sin(this.phase * 0.5) * this.height * 0.015
  }

  dispose() {
    for (const m of this.materials) m.dispose()
    this.haloMaterial.dispose()
    this.light.dispose?.()
    this.group.removeFromParent()
  }
}

// Sparkles rising from the heart of the isle and fading as they climb.
function sparkles({ count = 120, radius = 6, rise = 26, colour = '#bff2ff' } = {}) {
  const positions = new Float32Array(count * 3)
  const seeds = Array.from({ length: count }, (_, i) => ({ a: (i * 2.399) % (Math.PI * 2), r: radius * Math.sqrt(((i * 0.618) % 1) || 0.1), off: (i * 0.37) % 1 }))
  const geometry = new THREE.BufferGeometry()
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3))
  const material = new THREE.PointsMaterial({ color: colour, size: 0.5, map: softDot(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending })
  const points = new THREE.Points(geometry, material)
  function update(t, glow = 1) {
    material.opacity = 0.35 + glow * 0.5
    seeds.forEach((s, i) => {
      const k = (t * 0.05 + s.off) % 1
      const a = s.a + t * 0.1
      positions[i * 3] = Math.sin(a) * s.r * (1 + k * 0.6)
      positions[i * 3 + 1] = k * rise
      positions[i * 3 + 2] = Math.cos(a) * s.r * (1 + k * 0.6)
    })
    geometry.attributes.position.needsUpdate = true
  }
  return { points, update }
}

// pieces: { base, companion, islets: [{ object, orbit }], props: [{ object, at }] } - every
// object already sized and sitting on y = 0 (loadPiece). Returns { subject, dispose }.
export function buildCentrepiece(world, pieces, { glow = '#8fe6ff', seed = 7 } = {}) {
  const root = new THREE.Group()
  root.name = 'centrepiece'
  world.scene.add(root)

  // the base's top surface sits at y = 0, so the camera rig's heights stay simple
  const base = pieces.base
  root.add(base)
  const top = surfaceY(base, 0, 0)
  const box = new THREE.Box3().setFromObject(base)
  base.position.y = -(top ?? box.max.y)
  const size = box.getSize(new THREE.Vector3())
  const span = Math.max(size.x, size.z)

  let subject
  if (pieces.companion) {
    const height = new THREE.Box3().setFromObject(pieces.companion).getSize(new THREE.Vector3()).y
    subject = new GlowingPiece(pieces.companion, { colour: glow, height })
    root.add(subject.group)
    subject.position.set(0, 0, 0)
  } else {
    subject = { position: new THREE.Vector3(), heading: 0, walking: false, resting: false, update() {}, setSpeaking() {}, setResting() {}, play() {} }
  }

  const islets = pieces.islets ?? []
  for (const islet of islets) root.add(islet.object)
  for (const prop of pieces.props ?? []) {
    prop.object.position.set(...(prop.at ?? [0, 0, 0]))
    root.add(prop.object)
  }

  const sea = cloudSea(-span * 0.9 - 20)
  root.add(sea.mesh)
  const dust = motes({ seed, spread: span * 3, low: -span * 0.3, high: span * 0.5, centreZ: 0, size: 0.5 })
  root.add(dust.points)
  const rising = sparkles({ radius: span * 0.12, rise: span * 0.7, colour: glow })
  root.add(rising.points)

  const remove = world.onUpdate((dt, t, sky) => {
    const g = sky?.glow ?? 1
    subject.update(dt, t)
    islets.forEach((islet, i) => {
      const at = orbitAt(islet.orbit, t)
      islet.object.position.set(at.x, at.y + Math.sin(t * 0.3 + i * 2) * 0.6, at.z)
      islet.object.rotation.y = at.heading * 0.25
    })
    sea.update(t, sky)
    dust.update(t, g)
    rising.update(t, 0.5 + g * 0.5)
  })

  return {
    subject,
    root,
    span,
    dispose() {
      remove()
      subject.dispose?.()
      root.removeFromParent()
    },
  }
}
