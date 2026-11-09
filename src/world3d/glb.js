// Model pieces for the 3D worlds: one shared loader (meshopt-compressed GLBs, the way the
// prepare helper writes them), each piece sized by its bounding box to the metres its world.json
// asks for, and a downward ray to find where things stand on it. The maths is pure and tested.
import * as THREE from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { MeshoptDecoder } from 'three/examples/jsm/libs/meshopt_decoder.module.js'

let loader = null
function gltf() {
  if (!loader) {
    loader = new GLTFLoader()
    loader.setMeshoptDecoder(MeshoptDecoder)
  }
  return loader
}

// ---------- pure ----------

// The scale that makes a piece `size` metres across its largest side.
export function fitScale(dims, size) {
  const largest = Math.max(dims.x ?? dims[0] ?? 0, dims.y ?? dims[1] ?? 0, dims.z ?? dims[2] ?? 0)
  return largest > 0 && size > 0 ? size / largest : 1
}

// An islet circling the centre: { radius, height, lap (seconds a turn), phase } at time t.
export function orbitAt({ radius = 40, height = 0, lap = 240, phase = 0 } = {}, t = 0) {
  const angle = phase + (t / Math.max(1, lap)) * Math.PI * 2
  return { x: Math.sin(angle) * radius, y: height, z: Math.cos(angle) * radius, heading: angle + Math.PI / 2 }
}

// What is wrong with a world's pieces, in plain words (empty when it can be built).
export function pieceProblems(world) {
  const pieces = world?.pieces ?? []
  const problems = []
  const count = (role) => pieces.filter((p) => p.role === role).length
  if (!pieces.length) problems.push('it has no pieces')
  if (world?.layout === 'centrepiece') {
    if (count('base') !== 1) problems.push('a centrepiece world needs exactly one base piece')
    if (count('companion') > 1) problems.push('only one piece can be the companion')
  }
  if (world?.layout === 'archipelago') {
    if (!count('isle')) problems.push('an archipelago needs at least one isle piece')
    for (const p of pieces.filter((x) => x.role === 'isle' && !x.isle)) problems.push(`${p.file} does not say which isle it is`)
  }
  for (const p of pieces) if (!(p.size > 0)) problems.push(`${p.file} has no size`)
  return problems
}

// ---------- three.js ----------

// Loads one piece, sized and sitting with its lowest point at y = 0 and its middle over x = z = 0.
export async function loadPiece(url, size) {
  const result = await gltf().loadAsync(url)
  const model = result.scene
  model.traverse((node) => {
    if (node.isMesh) node.frustumCulled = true
  })
  const box = new THREE.Box3().setFromObject(model)
  const dims = box.getSize(new THREE.Vector3())
  model.scale.setScalar(fitScale(dims, size))
  box.setFromObject(model)
  const centre = box.getCenter(new THREE.Vector3())
  model.position.set(-centre.x, -box.min.y, -centre.z)
  const holder = new THREE.Group()
  holder.add(model)
  return holder
}

const ray = new THREE.Raycaster()
const DOWN = new THREE.Vector3(0, -1, 0)
// The highest surface of `object` under (x, z) in world space, or null when the ray misses.
export function surfaceY(object, x, z) {
  object.updateMatrixWorld(true)
  ray.set(new THREE.Vector3(x, 10000, z), DOWN)
  const hit = ray.intersectObject(object, true)[0]
  return hit ? hit.point.y : null
}

// Frees a piece's geometry, materials and textures.
export function disposePiece(object) {
  object?.traverse((node) => {
    if (!node.isMesh) return
    node.geometry?.dispose()
    for (const material of [node.material].flat()) {
      if (!material) continue
      for (const value of Object.values(material)) if (value?.isTexture) value.dispose()
      material.dispose()
    }
  })
}
