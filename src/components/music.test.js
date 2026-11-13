import { describe, expect, it } from 'vitest'
import { kindFor, musicFrom, tracksFor, worldWindowOpen } from './music.js'
import { allWorlds } from '../worlds/registry.js'

describe("Castle's music", () => {
  it('steps aside only for a world window that is really open on another screen', () => {
    expect(worldWindowOpen([])).toBe(false)
    expect(worldWindowOpen(undefined)).toBe(false)
    // a screen saved as on but with no window (unplugged, or Castle's own) does not count
    expect(worldWindowOpen([{ open: false, isMain: false }, { open: false, isMain: true }])).toBe(false)
    expect(worldWindowOpen([{ open: true, isMain: true }])).toBe(false)
    expect(worldWindowOpen([{ open: false, isMain: true }, { open: true, isMain: false }])).toBe(true)
  })

  it('reads the switch, the volume and the WORLD room from the settings', () => {
    expect(musicFrom({ world: { music: true, volume: 0.3 }, rooms: { world: true } })).toEqual({ music: true, volume: 0.3, roomOn: true })
    expect(musicFrom({ world: { music: false }, rooms: { world: false } })).toEqual({ music: false, volume: 0.5, roomOn: false })
    // nothing saved yet: quiet, half volume, the room on
    expect(musicFrom(undefined)).toEqual({ music: false, volume: 0.5, roomOn: true })
  })

  it("plays the music of the kind of world on the screen, then the companion's songs", () => {
    const worlds = allWorlds()
    expect(kindFor(worlds, '2d/mine')).toBe('2d')
    expect(kindFor(worlds, '3d/floating-isles')).toBe('3d')
    expect(kindFor(worlds, 'my/gone')).toBe('3d') // unknown: the default world's kind
    const tracks = tracksFor('2d', [{ name: '03-morning-light.mp3', path: 'D:\\songs\\03-morning-light.mp3' }], (path) => `asset://${path}`)
    expect(tracks[0].title).toBe('Castle Theme')
    expect(tracks.at(-1)).toEqual({ title: 'morning light', url: 'asset://D:\\songs\\03-morning-light.mp3' })
    expect(tracksFor('3d', undefined, String).map((t) => t.title)).toEqual(['Floating Isles', 'Floating Isles (second take)'])
  })
})
