import { describe, expect, it } from 'vitest'
import { SOUND_SHARE, shouldSound, soundLevel } from './themesound.js'

describe("a theme's looping sound", () => {
  it('is heard only while on, on HOME, in a visible window', () => {
    expect(shouldSound({ on: true, visible: true, awake: true })).toBe(true)
    expect(shouldSound({ on: false, visible: true, awake: true })).toBe(false)
    expect(shouldSound({ on: true, visible: false, awake: true })).toBe(false)
    expect(shouldSound({ on: true, visible: true, awake: false })).toBe(false)
    expect(shouldSound()).toBe(false)
  })

  it('sits under the music at a share of its slider, never outside 0..1', () => {
    expect(soundLevel(1)).toBe(SOUND_SHARE)
    expect(soundLevel(0.5)).toBeCloseTo(0.3)
    expect(soundLevel(0)).toBe(0)
    expect(soundLevel(3)).toBe(SOUND_SHARE)
    expect(soundLevel(-1)).toBe(0)
    expect(soundLevel('x')).toBeCloseTo(0.3)
  })
})
