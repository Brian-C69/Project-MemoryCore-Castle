import { describe, expect, it } from 'vitest'
import { EMBER, energyLook, quotaWords } from './energy.js'

describe('the quota glow', () => {
  it('keeps the normal glow when the limit is unknown', () => {
    expect(energyLook(null)).toEqual({ factor: 1, warm: 0, pulse: 0 })
    expect(energyLook(undefined).factor).toBe(1)
  })

  it('shines fully at full quota and fades as it is used', () => {
    expect(energyLook(1).factor).toBe(1)
    expect(energyLook(0.5).factor).toBeLessThan(1)
    expect(energyLook(0.5).factor).toBeGreaterThan(energyLook(0.3).factor)
    expect(energyLook(0.5).warm).toBe(0)
  })

  it('turns warm and pulses under a fifth, and is a faint ember at nothing left', () => {
    expect(energyLook(0.13).warm).toBeGreaterThan(0)
    expect(energyLook(0.13).pulse).toBeGreaterThan(0)
    expect(energyLook(0)).toEqual({ factor: EMBER, warm: 1, pulse: 0.25 })
    expect(energyLook(-3).factor).toBe(EMBER)
    expect(energyLook(7).factor).toBe(1)
  })

  it('says the quota in plain words', () => {
    expect(quotaWords(null)).toBe('')
    expect(quotaWords({ leftPercent: 72.4 })).toBe('72% left')
    const now = new Date('2026-11-09T05:00:00Z')
    expect(quotaWords({ leftPercent: 13, resetsAt: Date.parse('2026-11-12T21:00:00Z') }, now)).toMatch(/^13% left \(resets \w{3}\)$/)
    expect(quotaWords({ leftPercent: 50, resetsAt: Date.parse('2026-11-09T09:00:00Z') }, now)).toMatch(/^50% left \(resets .*\d.*\)$/)
  })
})
