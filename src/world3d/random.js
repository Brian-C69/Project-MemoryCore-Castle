// A small seeded random (mulberry32): the same seed always gives the same numbers, so the isles
// keep their shape between visits and the tests stay steady. Strings are hashed into a seed.

export function hashSeed(value) {
  if (typeof value === 'number' && Number.isFinite(value)) return value >>> 0 || 1
  const text = String(value ?? '')
  let h = 2166136261
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0 || 1
}

// A function returning numbers in [0, 1), like Math.random, but repeatable.
export function seeded(seed = 1) {
  let a = hashSeed(seed)
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
