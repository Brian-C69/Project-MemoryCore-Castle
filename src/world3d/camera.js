// The camera rig: films the director's shots around the crystal - gliding or cutting between
// them, always keeping it in frame. `subject` is anything with { position, heading, walking,
// resting } (the Crystal). `scale` widens every shot for a bigger world (1 = the built-in isles).
import * as THREE from 'three'
import { nextShot } from './director.js'

const CENTRE_Y = 1.35
const GLIDE_S = 1.6

// A screen's camera choice ('director' | 'follow' | 'wide' | 'orbit') as the shot to keep.
export const onlyShot = (camera) => (camera && camera !== 'director' ? camera : null)

export class CameraRig {
  // `framing` lets a world set its own orbit and wide shots: { orbit: { radius, height }, wide: { radius, height } }
  constructor(world, subject, { only = null, onShot, holdMin, holdMax, glide, shots, scale = 1, framing = {}, rand = Math.random } = {}) {
    this.world = world
    this.subject = subject
    this.only = only
    this.onShot = onShot
    this.hold = { holdMin, holdMax, glide, shots }
    this.scale = scale > 0 ? scale : 1
    this.framing = framing ?? {}
    this.rand = rand
    this.shot = null
    this.left = 0
    this.t = 0
    this.blend = 1
    this.fromPos = new THREE.Vector3()
    this.fromLook = new THREE.Vector3()
    this.look = new THREE.Vector3()
    this.wideAngle = rand() * Math.PI * 2
    this._pos = new THREE.Vector3()
    this._look = new THREE.Vector3()
    this._ahead = new THREE.Vector3()
    this._right = new THREE.Vector3()
    this._centre = new THREE.Vector3()
    this.stop = world.onUpdate((dt) => this.update(dt))
  }

  // One shot kept ('follow' | 'wide' | 'close' | 'orbit' | 'crane'), or null for the director.
  setOnly(shot) {
    this.only = shot
    this.left = 0
  }

  // A screen's settings changed while it runs: { camera, holdMin, holdMax, glide, shots }.
  setSettings({ camera, holdMin, holdMax, glide, shots } = {}) {
    this.only = onlyShot(camera)
    this.hold = { holdMin, holdMax, glide, shots }
    this.left = 0
  }

  choose() {
    const next = nextShot({
      previous: this.shot,
      walking: !!this.subject?.walking,
      resting: !!this.subject?.resting,
      only: this.only,
      ...Object.fromEntries(Object.entries(this.hold).filter(([, v]) => v !== undefined)),
      rand: this.rand,
    })
    const first = this.shot === null
    this.shot = next.shot
    this.left = next.seconds
    this.t = 0
    this.wideAngle += Math.PI * (0.4 + this.rand() * 0.8)
    if (next.glide && !first) {
      this.fromPos.copy(this.world.camera.position)
      this.fromLook.copy(this.look)
      this.blend = 0
    } else {
      this.blend = 1
      this.cut = true
    }
    this.onShot?.(this.shot)
  }

  // Where the camera wants to be, and where it looks, for the current shot.
  aim(pos, look) {
    const at = this.subject?.position ?? this._centre.set(0, 0, 0)
    const facing = this.subject?.heading ?? 0
    const ahead = this._ahead.set(Math.sin(facing), 0, Math.cos(facing))
    const right = this._right.set(ahead.z, 0, -ahead.x)
    const k = this.scale
    const centre = this._centre.copy(at).setY(at.y + CENTRE_Y * k)
    const t = this.t
    switch (this.shot) {
      case 'follow':
        pos.copy(at).addScaledVector(ahead, -6.5 * k).addScaledVector(right, 1.4 * k).setY(at.y + 2.8 * k)
        look.copy(centre).addScaledVector(ahead, 2.5 * k)
        break
      case 'close':
        pos.copy(at).addScaledVector(ahead, 4.2 * k).addScaledVector(right, 1.1 * k).setY(at.y + (CENTRE_Y + 0.35) * k)
        look.copy(centre)
        break
      case 'orbit': {
        // the camera keeps one gentle walking pace, so a wider circle simply takes longer
        const r = this.framing.orbit?.radius ?? 9 * k
        const h = this.framing.orbit?.height ?? 3.2 * k
        const a = facing + t * (1.44 / r)
        pos.set(at.x + Math.sin(a) * r, at.y + h, at.z + Math.cos(a) * r)
        look.copy(centre)
        break
      }
      case 'crane':
        pos.copy(at).addScaledVector(right, 7 * k).setY(at.y + (24 - Math.min(11, t * 0.6)) * k)
        look.copy(centre)
        break
      case 'wide':
      default: {
        const a = this.wideAngle + t * 0.015
        const r = this.framing.wide?.radius ?? 62 * k
        const h = this.framing.wide?.height ?? 22 * k
        pos.set(at.x * 0.4 + Math.sin(a) * r, at.y + h, at.z * 0.4 + Math.cos(a) * r)
        look.copy(at).multiplyScalar(0.5)
        look.y = at.y
        break
      }
    }
  }

  update(dt) {
    this.t += dt
    this.left -= dt
    if (!this.shot || this.left <= 0) this.choose()
    // the crystal set off: the director looks for a walking shot early
    const walkingViews = ['follow', 'wide', 'crane']
    const canFollow = !this.hold.shots?.length || walkingViews.some((s) => this.hold.shots.includes(s))
    if (this.subject?.walking && !this.only && canFollow && !walkingViews.includes(this.shot) && this.t > 2) this.choose()
    const pos = this._pos
    const look = this._look
    this.aim(pos, look)
    const camera = this.world.camera
    if (this.cut) {
      this.cut = false
      camera.position.copy(pos)
      this.look.copy(look)
    } else if (this.blend < 1) {
      this.blend = Math.min(1, this.blend + dt / GLIDE_S)
      const k = this.blend * this.blend * (3 - 2 * this.blend)
      camera.position.lerpVectors(this.fromPos, pos, k)
      this.look.lerpVectors(this.fromLook, look, k)
    } else {
      // a gentle follow so walking never jitters
      camera.position.lerp(pos, Math.min(1, dt * 3))
      this.look.lerp(look, Math.min(1, dt * 4))
    }
    camera.lookAt(this.look)
  }

  dispose() {
    this.stop?.()
  }
}
