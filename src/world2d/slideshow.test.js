import { describe, expect, it } from 'vitest'
import { MAX_ZOOM, MIN_ZOOM, kenBurns, maxPan, nextIndex, shuffle, transformCss } from './slideshow.js'

const PICTURES = Array.from({ length: 12 }, (_, i) => `pic-${i}.jpg`)

describe('shuffle', () => {
  it('keeps every picture exactly once', () => {
    const out = shuffle(PICTURES, 5)
    expect(out).toHaveLength(PICTURES.length)
    expect([...out].sort()).toEqual([...PICTURES].sort())
  })

  it('is repeatable with the same seed and differs with another', () => {
    expect(shuffle(PICTURES, 5)).toEqual(shuffle(PICTURES, 5))
    expect(shuffle(PICTURES, 5)).not.toEqual(shuffle(PICTURES, 6))
  })

  it('never changes the list it was given', () => {
    const copy = [...PICTURES]
    shuffle(PICTURES, 9)
    expect(PICTURES).toEqual(copy)
  })

  it('handles empty and missing lists', () => {
    expect(shuffle([], 1)).toEqual([])
    expect(shuffle(undefined, 1)).toEqual([])
    expect(shuffle(['one'], 1)).toEqual(['one'])
  })
})

describe('nextIndex', () => {
  it('steps forward and wraps at the end', () => {
    expect(nextIndex(0, 3)).toBe(1)
    expect(nextIndex(2, 3)).toBe(0)
  })

  it('steps back and wraps at the start', () => {
    expect(nextIndex(0, 3, -1)).toBe(2)
    expect(nextIndex(1, 3, -1)).toBe(0)
    expect(nextIndex(1, 3, -7)).toBe(stepByOne(1, 3, -7))
  })

  it('gives -1 when there are no pictures', () => {
    expect(nextIndex(0, 0)).toBe(-1)
  })

  it('treats a missing index as the first', () => {
    expect(nextIndex(undefined, 4)).toBe(1)
  })
})

// a slow reference: step one at a time
function stepByOne(index, length, step) {
  let i = index
  for (let n = 0; n < Math.abs(step); n++) i = (i + Math.sign(step) + length) % length
  return i
}

describe('Ken Burns', () => {
  it('zooms within the allowed range', () => {
    for (let n = 0; n < 100; n++) {
      const { from, to } = kenBurns(n, 3)
      for (const end of [from, to]) {
        expect(end.scale).toBeGreaterThanOrEqual(MIN_ZOOM - 0.001)
        expect(end.scale).toBeLessThanOrEqual(MAX_ZOOM + 0.001)
      }
    }
  })

  it('never slides far enough to show an edge', () => {
    for (let n = 0; n < 200; n++) {
      const { from, to } = kenBurns(n, 8)
      for (const end of [from, to]) {
        expect(Math.abs(end.x)).toBeLessThanOrEqual(maxPan(end.scale) + 1e-9)
        expect(Math.abs(end.y)).toBeLessThanOrEqual(maxPan(end.scale) + 1e-9)
      }
    }
  })

  it('takes turns zooming in and out', () => {
    const even = kenBurns(4, 1)
    const odd = kenBurns(5, 1)
    expect(even.to.scale).toBeGreaterThan(even.from.scale)
    expect(odd.to.scale).toBeLessThan(odd.from.scale)
  })

  it('is repeatable per slide and seed', () => {
    expect(kenBurns(3, 2)).toEqual(kenBurns(3, 2))
    expect(kenBurns(3, 2)).not.toEqual(kenBurns(4, 2))
  })

  it('writes a CSS transform', () => {
    expect(transformCss({ scale: 1.1, x: 2, y: -1.5 })).toBe('scale(1.1) translate(2%, -1.5%)')
  })

  it('allows no slide at all without zoom', () => {
    expect(maxPan(1)).toBe(0)
  })
})
