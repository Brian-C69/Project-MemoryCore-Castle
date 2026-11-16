import { describe, expect, it } from 'vitest'
import { rsi, sma, stochastic } from './indicators.js'

describe('sma', () => {
  it('averages the trailing window once it has enough bars', () => {
    expect(sma([1, 2, 3, 4, 5], 3)).toEqual([null, null, 2, 3, 4])
  })
  it('is null for every bar when the period does not fit', () => {
    expect(sma([1, 2], 5)).toEqual([null, null])
  })
})

describe('rsi', () => {
  it('reaches 100 when every bar gains and 0 when every bar loses', () => {
    const up = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24]
    expect(rsi(up, 14).at(-1)).toBe(100)
    const down = [...up].reverse()
    expect(rsi(down, 14).at(-1)).toBe(0)
  })
  it('sits at 50 when nothing moves', () => {
    const flat = new Array(16).fill(10)
    expect(rsi(flat, 14).at(-1)).toBe(50)
  })
  it('is null until the period has filled', () => {
    const closes = [1, 2, 3, 4, 5]
    expect(rsi(closes, 14)).toEqual(closes.map(() => null))
  })
})

describe('stochastic', () => {
  it('places %K by where the close sits in the recent high/low range', () => {
    const highs = [100, 105, 110, 108, 107]
    const lows = [90, 92, 90, 95, 96]
    const closes = [95, 100, 100, 101, 96]
    // over the 5-bar window the range is 90..110; the last close (96) sits 30% up it
    const { k } = stochastic(highs, lows, closes, 5, 3)
    expect(k[4]).toBeCloseTo(30, 5)
    expect(k.slice(0, 4)).toEqual([null, null, null, null])
  })
  it('%D is %K\'s own moving average, starting once %K has filled', () => {
    const highs = [10, 10, 10, 10, 10, 10]
    const lows = [0, 0, 0, 0, 0, 0]
    const closes = [5, 6, 7, 8, 9, 10]
    const { k, d } = stochastic(highs, lows, closes, 3, 2)
    expect(d[0]).toBeNull()
    expect(d[2]).toBeNull() // %K just started at index 2, needs one more for %D
    expect(d[3]).toBeCloseTo((k[2] + k[3]) / 2, 5)
  })
  it('treats a flat high/low range as the middle, never dividing by zero', () => {
    const { k } = stochastic([10, 10], [10, 10], [10, 10], 2, 1)
    expect(k[1]).toBe(50)
  })
})
