import { describe, expect, it } from 'vitest'
import { shouldPlay } from './ambientmusic.js'

describe('Castle’s own player: should it play', () => {
  it('plays when music is on, the WORLD room is on, and no world window has it', () => {
    expect(shouldPlay({ music: true, roomOn: true, screensOn: false })).toBe(true)
  })
  it('stays quiet once a world window is really playing it', () => {
    expect(shouldPlay({ music: true, roomOn: true, screensOn: true })).toBe(false)
  })
  it('stays quiet while the music switch is off', () => {
    expect(shouldPlay({ music: false, roomOn: true, screensOn: false })).toBe(false)
  })
  it('stays quiet while the WORLD room itself is off', () => {
    expect(shouldPlay({ music: true, roomOn: false, screensOn: false })).toBe(false)
  })
})
