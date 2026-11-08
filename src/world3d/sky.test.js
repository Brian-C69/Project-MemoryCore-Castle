import { describe, expect, it } from 'vitest'
import { arcDirection, moonPhase, skyAt } from './sky.js'

const at = (h, m = 0, s = 0) => new Date(2026, 10, 9, h, m, s)
const brightness = (hex) => [1, 3, 5].reduce((sum, i) => sum + parseInt(hex.slice(i, i + 2), 16), 0)
const HEX = /^#[0-9a-f]{6}$/

describe('the sky from the real clock', () => {
  it('is night with full stars and no sun at midnight', () => {
    const sky = skyAt(at(0))
    expect(sky.night).toBe(true)
    expect(sky.stars).toBe(1)
    expect(sky.sun.height).toBe(0)
    expect(sky.glow).toBe(1)
  })

  it('is day at noon: no stars, the sun overhead, the strongest light', () => {
    const noon = skyAt(at(12, 15))
    const midnight = skyAt(at(0))
    expect(noon.night).toBe(false)
    expect(noon.stars).toBe(0)
    expect(noon.sun.height).toBeGreaterThan(0.95)
    expect(noon.strength).toBeGreaterThan(midnight.strength)
    expect(brightness(noon.top)).toBeGreaterThan(brightness(midnight.top))
    expect(noon.moon.show).toBe(0)
  })

  it('gives valid colours at every minute of the day', () => {
    for (let minute = 0; minute < 24 * 60; minute += 7) {
      const sky = skyAt(at(Math.floor(minute / 60), minute % 60))
      for (const key of ['top', 'horizon', 'light']) expect(sky[key]).toMatch(HEX)
      expect(sky.moon.colour).toMatch(HEX)
      for (const key of ['stars', 'glow', 'ambient']) {
        expect(sky[key]).toBeGreaterThanOrEqual(0)
        expect(sky[key]).toBeLessThanOrEqual(1)
      }
    }
  })

  it('changes smoothly - one minute never jumps far', () => {
    for (let minute = 0; minute < 24 * 60; minute++) {
      const a = skyAt(at(Math.floor(minute / 60), minute % 60))
      const next = minute + 1
      const b = skyAt(at(Math.floor(next / 60) % 24, next % 60))
      expect(Math.abs(brightness(a.top) - brightness(b.top))).toBeLessThan(12)
      expect(Math.abs(a.strength - b.strength)).toBeLessThan(0.02)
    }
  })

  it('wraps midnight cleanly', () => {
    const late = skyAt(at(23, 59, 59))
    const early = skyAt(at(0, 0, 0))
    expect(late.top).toBe(early.top)
    expect(Math.abs(late.strength - early.strength)).toBeLessThan(0.001)
  })

  it('warms the dawn and dusk horizons', () => {
    const dawn = skyAt(at(6))
    const dusk = skyAt(at(18, 15))
    for (const sky of [dawn, dusk]) {
      const r = parseInt(sky.horizon.slice(1, 3), 16)
      const b = parseInt(sky.horizon.slice(5, 7), 16)
      expect(r).toBeGreaterThan(b)
    }
  })

  it('raises the moon through the night, highest around midnight, warm near the horizon', () => {
    const rising = skyAt(at(19))
    const high = skyAt(at(0, 15))
    expect(high.moon.height).toBeGreaterThan(rising.moon.height)
    expect(high.moon.show).toBe(1)
    expect(brightness(high.moon.colour)).toBeGreaterThan(brightness(rising.moon.colour))
  })
})

describe('the moon phase', () => {
  it('is full on a known full moon and new on a known new moon', () => {
    expect(moonPhase(new Date(Date.UTC(2024, 0, 25, 17, 54)))).toBeCloseTo(0.5, 1)
    const p = moonPhase(new Date(Date.UTC(2024, 0, 11, 11, 57)))
    expect(Math.min(p, 1 - p)).toBeLessThan(0.03)
  })

  it('always lies in [0, 1)', () => {
    for (let day = -400; day < 400; day += 13) {
      const p = moonPhase(new Date(Date.UTC(2026, 0, 1) + day * 86400000))
      expect(p).toBeGreaterThanOrEqual(0)
      expect(p).toBeLessThan(1)
    }
  })

  it('reports the lit fraction from the phase', () => {
    const full = skyAt(new Date(Date.UTC(2024, 0, 25, 17, 54)))
    expect(full.moon.lit).toBeGreaterThan(0.97)
  })
})

describe('arc directions', () => {
  it('are unit vectors in front of the home view', () => {
    for (const around of [0, 0.25, 0.5, 1]) {
      const [x, y, z] = arcDirection(around, 0.5)
      expect(Math.hypot(x, y, z)).toBeCloseTo(1, 6)
      expect(z).toBeLessThan(0)
      expect(y).toBeGreaterThan(0)
    }
  })
})
