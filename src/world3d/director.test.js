import { describe, expect, it } from 'vitest'
import { MAX_S, MIN_S, RESTING_WEIGHTS, SHOTS, WALKING_WEIGHTS, WEIGHTS, nextShot, pickWeighted } from './director.js'
import { seeded } from './random.js'

describe('the camera director', () => {
  it('never shows the same shot twice in a row', () => {
    const rand = seeded(7)
    let previous = null
    for (let i = 0; i < 300; i++) {
      const next = nextShot({ previous, rand })
      expect(next.shot).not.toBe(previous)
      expect(SHOTS).toContain(next.shot)
      previous = next.shot
    }
  })

  it('holds each shot 8 to 20 seconds', () => {
    const rand = seeded(3)
    for (let i = 0; i < 300; i++) {
      const { seconds } = nextShot({ rand })
      expect(seconds).toBeGreaterThanOrEqual(MIN_S)
      expect(seconds).toBeLessThanOrEqual(MAX_S)
    }
    expect(MIN_S).toBe(8)
    expect(MAX_S).toBe(20)
  })

  it('is repeatable with the same seed', () => {
    const run = (seed) => {
      const rand = seeded(seed)
      let previous = null
      return Array.from({ length: 20 }, () => {
        const next = nextShot({ previous, rand })
        previous = next.shot
        return `${next.shot}:${next.seconds.toFixed(3)}:${next.glide}`
      })
    }
    expect(run(42)).toEqual(run(42))
    expect(run(42)).not.toEqual(run(43))
  })

  it('follows the weights over many picks', () => {
    const rand = seeded(11)
    const counts = Object.fromEntries(SHOTS.map((s) => [s, 0]))
    const N = 20000
    for (let i = 0; i < N; i++) counts[pickWeighted(WEIGHTS, null, rand)]++
    const total = Object.values(WEIGHTS).reduce((a, b) => a + b, 0)
    for (const [shot, w] of Object.entries(WEIGHTS)) expect(counts[shot] / N).toBeCloseTo(w / total, 1)
    expect(counts.follow).toBeGreaterThan(counts.crane)
  })

  it('keeps to walking shots while the crystal walks', () => {
    const rand = seeded(5)
    for (let i = 0; i < 200; i++) expect(Object.keys(WALKING_WEIGHTS)).toContain(nextShot({ walking: true, rand }).shot)
  })

  it('keeps to calm shots while the crystal rests', () => {
    const rand = seeded(9)
    for (let i = 0; i < 200; i++) expect(Object.keys(RESTING_WEIGHTS)).toContain(nextShot({ resting: true, rand }).shot)
  })

  it('keeps one shot when asked, gliding into it', () => {
    const rand = seeded(1)
    for (let i = 0; i < 20; i++) {
      const next = nextShot({ only: 'orbit', previous: 'orbit', rand })
      expect(next.shot).toBe('orbit')
      expect(next.glide).toBe(true)
    }
  })

  it('honours custom hold times, even given the wrong way round', () => {
    const rand = seeded(2)
    for (let i = 0; i < 50; i++) {
      const { seconds } = nextShot({ holdMin: 15, holdMax: 4, rand })
      expect(seconds).toBeGreaterThanOrEqual(4)
      expect(seconds).toBeLessThanOrEqual(15)
    }
  })

  it('mixes glides and cuts', () => {
    const rand = seeded(4)
    const glides = Array.from({ length: 200 }, () => nextShot({ rand }).glide)
    expect(glides).toContain(true)
    expect(glides).toContain(false)
  })

  it('returns the only shot left when a single weight remains', () => {
    expect(pickWeighted({ wide: 1 }, 'wide', seeded(1))).toBe('wide')
  })
})
