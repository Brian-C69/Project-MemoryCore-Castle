import { describe, expect, it } from 'vitest'
import { shouldLoad, shouldPlay } from './videoloop.js'

describe('a living wallpaper video', () => {
  it('plays only when on, on HOME, in a visible window, and motion is allowed', () => {
    expect(shouldPlay({ enabled: true, visible: true, awake: true, calm: false })).toBe(true)
    expect(shouldPlay({ enabled: false, visible: true, awake: true })).toBe(false)
    expect(shouldPlay({ enabled: true, visible: false, awake: true })).toBe(false)
    expect(shouldPlay({ enabled: true, visible: true, awake: false })).toBe(false)
    expect(shouldPlay({ enabled: true, visible: true, awake: true, calm: true })).toBe(false)
    expect(shouldPlay()).toBe(false)
  })

  it('is never even loaded while Moving wallpapers is off or motion is reduced', () => {
    expect(shouldLoad({ enabled: true })).toBe(true)
    expect(shouldLoad({ enabled: false })).toBe(false)
    expect(shouldLoad({ enabled: true, calm: true })).toBe(false)
  })
})
