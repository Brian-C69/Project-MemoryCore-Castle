import { describe, expect, it } from 'vitest'
import {
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
    expect(river.count).toBe(70)
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
})
