import { describe, expect, it } from 'vitest'
import { BAND_AXIS, GLIDE_STOP, PALETTES, bandDirections, glideStep, seeded, sparkle, toneCurve } from './space.js'

describe('Deep Space (recipe by Ashran Helmi) - the pure parts', () => {
  it('has the three palettes, each with three colours', () => {
    expect(Object.keys(PALETTES)).toEqual(['kociverse', 'ember', 'teal'])
    for (const p of Object.values(PALETTES)) expect(p.colours).toHaveLength(3)
  })

  it('rolls bright gas off instead of clipping', () => {
    expect(toneCurve(0)).toBe(0)
    expect(toneCurve(1)).toBeLessThan(1)
    expect(toneCurve(100)).toBeLessThan(1.7 / 1.9 + 1e-9)
    expect(toneCurve(0.5)).toBeGreaterThan(toneCurve(0.4))
  })

  it('keeps each sparkle dark most of the time', () => {
    let dark = 0
    let sum = 0
    const samples = 10000
    for (let i = 0; i < samples; i++) {
      const v = sparkle(i * 0.037, 0.5, 1.1)
      if (v < 0.05) dark++
      sum += v
    }
    expect(dark / samples).toBeGreaterThan(0.75)
    expect(sum / samples).toBeLessThan(0.12)
    expect(Math.max(...Array.from({ length: 2000 }, (_, i) => sparkle(i * 0.01, 1, 0)))).toBeGreaterThan(0.95)
  })

  it('lets a glide slow down and stop completely', () => {
    let v = 2
    let steps = 0
    while (v !== 0 && steps < 10000) {
      v = glideStep(v, 1 / 30)
      steps++
    }
    expect(v).toBe(0)
    expect(steps).toBeLessThan(10000)
    expect(glideStep(GLIDE_STOP / 2, 0)).toBe(0)
  })

  it('gathers stars around the tilted band, the same every time', () => {
    const a = bandDirections(2000, seeded(3))
    const b = bandDirections(2000, seeded(3))
    expect(a[10].equals(b[10])).toBe(true)
    const near = a.filter((d) => Math.abs(d.dot(BAND_AXIS)) < 0.25).length / a.length
    // an even sky puts about 25% within that strip; the band should hold clearly more
    expect(near).toBeGreaterThan(0.4)
  })
})
