import { describe, expect, it } from 'vitest'
import { RATES, clampVolume, loadPrefs, nextRate, playbackRegistry, rateLabel, savePrefs } from './playerstate.js'

const memory = () => {
  const kept = {}
  return { getItem: (k) => (k in kept ? kept[k] : null), setItem: (k, v) => { kept[k] = v }, kept }
}

describe("the audio player's speed", () => {
  it('goes through the speeds one click at a time and comes back to normal', () => {
    let rate = 1
    const seen = []
    for (let i = 0; i < RATES.length; i++) { rate = nextRate(rate); seen.push(rate) }
    expect(seen).toEqual([1.25, 1.5, 2, 0.75, 1])
  })

  it('starts again from normal for a speed that is not in the list', () => {
    expect(nextRate(3)).toBe(1)
    expect(nextRate(undefined)).toBe(1)
  })

  it('writes a speed as the member reads it', () => {
    expect(rateLabel(1)).toBe('1×')
    expect(rateLabel(1.25)).toBe('1.25×')
    expect(rateLabel(0.75)).toBe('0.75×')
  })
})

describe("the audio player's volume", () => {
  it('stays between nothing and full, and an unusable value is full volume', () => {
    expect(clampVolume(0.4)).toBe(0.4)
    expect(clampVolume(-3)).toBe(0)
    expect(clampVolume(7)).toBe(1)
    expect(clampVolume('0.5')).toBe(0.5)
    expect(clampVolume(NaN)).toBe(1)
    expect(clampVolume(undefined)).toBe(1)
  })

  it('remembers the volume and the mute', () => {
    const storage = memory()
    savePrefs(storage, { volume: 0.3, muted: true })
    expect(loadPrefs(storage)).toEqual({ volume: 0.3, muted: true })
  })

  it('starts at full volume, unmuted, when nothing was kept or what was kept is unreadable', () => {
    expect(loadPrefs(memory())).toEqual({ volume: 1, muted: false })
    const broken = memory()
    broken.setItem('castle.player', '{not json')
    expect(loadPrefs(broken)).toEqual({ volume: 1, muted: false })
    const odd = memory()
    odd.setItem('castle.player', JSON.stringify({ volume: 9, muted: 'yes' }))
    expect(loadPrefs(odd)).toEqual({ volume: 1, muted: false })
  })

  it('does not fail when the browser refuses to keep anything', () => {
    const refusing = { getItem: () => { throw new Error('blocked') }, setItem: () => { throw new Error('blocked') } }
    expect(() => savePrefs(refusing, { volume: 0.5, muted: false })).not.toThrow()
    expect(loadPrefs(refusing)).toEqual({ volume: 1, muted: false })
  })
})

describe('one clip at a time', () => {
  it('stops the clip that was playing when another starts', () => {
    const stopped = []
    const registry = playbackRegistry()
    registry.claim('a', () => stopped.push('a'))
    registry.claim('b', () => stopped.push('b'))
    expect(stopped).toEqual(['a'])
    registry.claim('c', () => stopped.push('c'))
    expect(stopped).toEqual(['a', 'b'])
  })

  it('does not stop a clip because it started again', () => {
    const stopped = []
    const registry = playbackRegistry()
    registry.claim('a', () => stopped.push('a'))
    registry.claim('a', () => stopped.push('a'))
    expect(stopped).toEqual([])
  })

  it('leaves the next clip alone once the first one has ended', () => {
    const stopped = []
    const registry = playbackRegistry()
    registry.claim('a', () => stopped.push('a'))
    registry.release('a')
    registry.claim('b', () => stopped.push('b'))
    expect(stopped).toEqual([])
  })

  it('ignores a release from a clip that is not the one playing', () => {
    const stopped = []
    const registry = playbackRegistry()
    registry.claim('a', () => stopped.push('a'))
    registry.release('z')
    registry.claim('b', () => stopped.push('b'))
    expect(stopped).toEqual(['a'])
  })
})
