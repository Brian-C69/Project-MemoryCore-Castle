import { describe, expect, it } from 'vitest'
import {
  bandSpan,
  brightestPoints,
  foamSpecks,
  newPatch,
  patchAlpha,
  shoreSwing,
  stepPatches,
  twinkleStars,
  twinkleState,
  along,
  inside,
  newRipple,
  newStreak,
  pointOnWater,
  readMotion,
  rippleState,
  seeded,
  stepRipples,
  stepStreaks,
  stepSurf,
  surfBrightness,
  surfParticles,
  toScreen,
} from './motion.js'

const river = readMotion({ kind: 'river', water: [[0.4, 0.2], [0.6, 0.2], [0.9, 1], [0.1, 1]], flow: [0, 2], ripples: [[0.5, 0.6]] })

describe('a theme\'s living water', () => {
  it('reads a good block and ignores a bad one', () => {
    const surf = readMotion({ kind: 'surf', crests: [[[0, 0.6], [1, 0.5]], [[0, 2], [1, 1]]], color: '#5FD8FF', count: 9999 })
    expect(surf.crests).toHaveLength(1)
    expect(surf.count).toBe(240)
    expect(surf.color).toBe('#5FD8FF')
    expect(readMotion({ kind: 'surf', crests: [] })).toBeNull()
    expect(readMotion({ kind: 'river', water: [[0, 0], [1, 1]] })).toBeNull()
    expect(readMotion({ kind: 'fireworks' })).toBeNull()
    expect(readMotion(null)).toBeNull()
    expect(river.flow).toEqual({ x: 0, y: 1 })
    expect(river.color).toBe('#e8f7c8')
    expect(river.count).toBe(90)
  })

  it('maps picture fractions through the cover box', () => {
    expect(toScreen({ x: 0.5, y: 0.25 }, { x: -100, y: 0, width: 2000, height: 1000 })).toEqual({ x: 900, y: 250 })
  })

  it('knows what is on the water', () => {
    expect(inside({ x: 0.5, y: 0.6 }, river.water)).toBe(true)
    expect(inside({ x: 0.05, y: 0.3 }, river.water)).toBe(false)
    const random = seeded(7)
    for (let i = 0; i < 50; i++) expect(inside(pointOnWater(river.water, random), river.water)).toBe(true)
  })

  it('walks a crest by its length', () => {
    const line = [{ x: 0, y: 0 }, { x: 0.3, y: 0 }, { x: 0.3, y: 0.1 }]
    expect(along(line, 0)).toEqual({ x: 0, y: 0 })
    expect(along(line, 0.5)).toEqual({ x: 0.2, y: 0 })
    expect(along(line, 1).y).toBeCloseTo(0.1)
  })

  it('lets the surf ride its crests, wrap around and glow', () => {
    const surf = readMotion({ kind: 'surf', crests: [[[0, 0.6], [1, 0.5]]], count: 20 })
    const particles = surfParticles(surf, seeded(3))
    expect(particles).toHaveLength(20)
    for (let i = 0; i < 400; i++) stepSurf(particles, 1)
    for (const p of particles) {
      expect(p.t).toBeGreaterThanOrEqual(0)
      expect(p.t).toBeLessThanOrEqual(1)
      const b = surfBrightness(p, 12.3)
      expect(b).toBeGreaterThanOrEqual(0)
      expect(b).toBeLessThanOrEqual(1)
    }
  })

  it('keeps the streaks on the water as they flow', () => {
    const random = seeded(11)
    const streaks = Array.from({ length: 30 }, () => newStreak(river, random))
    for (let i = 0; i < 200; i++) stepStreaks(streaks, river, 1 / 30, random)
    for (const s of streaks) expect(inside(s, river.water)).toBe(true)
  })

  it('lets a ripple spread, fade and go', () => {
    let ripples = [newRipple({ x: 0.5, y: 0.6 }, seeded(2))]
    const first = rippleState(ripples[0])
    ripples = stepRipples(ripples, 1)
    const later = rippleState(ripples[0])
    expect(later.radius).toBeGreaterThan(first.radius)
    expect(later.alpha).toBeLessThan(first.alpha)
    expect(stepRipples(ripples, 10)).toEqual([])
  })

  it('rolls a band of light along each crest and lights the points it passes', () => {
    let seen = 0
    for (let s = 0; s < 12; s += 0.25) {
      const span = bandSpan(0, s)
      if (!span) continue
      seen += 1
      expect(span.from).toBeGreaterThanOrEqual(0)
      expect(span.to).toBeLessThanOrEqual(1)
      expect(span.to).toBeGreaterThan(span.from)
    }
    expect(seen).toBeGreaterThan(10)
    const point = { crest: 0, t: 0.5, phase: 0 }
    // the band sits at t 0.5 at 4.2 s (seconds / 6 - 0.2 = 0.5)
    expect(surfBrightness(point, 4.2)).toBeGreaterThan(surfBrightness(point, 0.5))
  })

  it('lets sun patches drift on the water and breathe', () => {
    const random = seeded(5)
    const patches = Array.from({ length: 10 }, () => newPatch(river, random))
    for (let i = 0; i < 300; i++) stepPatches(patches, river, 1 / 30, random)
    for (const p of patches) {
      expect(inside(p, river.water)).toBe(true)
      expect(patchAlpha(p)).toBeGreaterThanOrEqual(0)
      expect(patchAlpha(p)).toBeLessThanOrEqual(1)
    }
  })

  it('reads a shore and washes its foam in quickly and back slowly', () => {
    const shore = readMotion({ kind: 'shore', line: [[0.4, 1], [1, 0.5]], toward: [1, -1], reach: 5 })
    expect(shore.reach).toBe(0.1)
    expect(shore.toward.x).toBeCloseTo(Math.SQRT1_2)
    expect(readMotion({ kind: 'shore', line: [[0.4, 1]] })).toBeNull()
    expect(shoreSwing(0)).toBeCloseTo(0)
    expect(shoreSwing(8 * 0.35)).toBeCloseTo(1)
    expect(shoreSwing(8 * 0.999)).toBeLessThan(0.01)
    expect(shoreSwing(8 * 0.2)).toBeGreaterThan(shoreSwing(8 * 0.8))
    expect(foamSpecks(shore, seeded(1), 12)).toHaveLength(12)
  })

  it('finds the brightest stars inside the sky, strongest first and spaced apart', () => {
    const width = 20
    const height = 10
    const grid = new Float32Array(width * height).fill(0.1)
    grid[2 * width + 5] = 0.95 // in the sky
    grid[2 * width + 6] = 0.9 // its neighbour: not a peak
    grid[3 * width + 12] = 0.8 // in the sky
    grid[8 * width + 10] = 1 // below the sky: left out
    const sky = readMotion({ kind: 'twinkle', area: [[0, 0], [1, 0], [1, 0.5], [0, 0.5]] }).area
    const stars = brightestPoints(grid, width, height, sky, 10)
    expect(stars.map((s) => [Math.round(s.x * width - 0.5), Math.round(s.y * height - 0.5)])).toEqual([[5, 2], [12, 3]])
    expect(brightestPoints(grid, width, height, sky, 1)).toHaveLength(1)
    const lit = twinkleStars(stars, seeded(4))
    for (const s of lit) {
      for (let t = 0; t < 10; t += 0.5) {
        const v = twinkleState(s, t)
        expect(v).toBeGreaterThanOrEqual(0)
        expect(v).toBeLessThanOrEqual(1)
      }
    }
  })
})
