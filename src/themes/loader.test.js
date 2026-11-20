import { describe, expect, it } from 'vitest'
import { SKINS, builtInThemes, skinClass } from './loader.js'

describe('a theme\'s widget skin', () => {
  it('gives HOME the class of a skin Castle knows, and nothing for any other name', () => {
    expect(skinClass({ widgetSkin: 'carved-wood' })).toBe('skin-carved-wood')
    expect(skinClass({ widgetSkin: 'neon-tiger' })).toBe('')
    expect(skinClass({})).toBe('')
    expect(skinClass(null)).toBe('')
  })

  it('names only skins Castle knows in the built-in themes, and none for The Constellation', () => {
    const themes = builtInThemes()
    for (const t of themes) if (t.widgetSkin) expect(SKINS).toContain(t.widgetSkin)
    expect(themes.find((t) => t.id === 'constellation')?.widgetSkin).toBeUndefined()
  })

  it('brings the three places of Nov 20 with their own skins and wallpapers, and no video yet', () => {
    const themes = builtInThemes()
    const expected = { 'windmill-hill': 'windmill-cloud', 'thousand-sails': 'lighthouse-beam', 'lantern-falls': 'still-water' }
    for (const [id, skin] of Object.entries(expected)) {
      const theme = themes.find((t) => t.id === id)
      expect(theme?.name).toMatch(/^The /)
      expect(theme.widgetSkin).toBe(skin)
      expect(skinClass(theme)).toBe(`skin-${skin}`)
      expect(theme.wallpaperUrl).toBeTruthy()
      expect(theme.motionVideoUrl).toBeNull()
    }
  })
})

describe('living wallpapers', () => {
  it('gives the two shores and the river their looping videos and leaves the drawn motion to The Constellation alone', () => {
    const themes = builtInThemes()
    const byId = (id) => themes.find((t) => t.id === id)
    for (const id of ['star-shore', 'green-river', 'sunset-shore']) {
      expect(byId(id).motionVideoUrl).toBeTruthy()
      expect(byId(id).motionSoundUrl).toBeTruthy()
      expect(byId(id).motion).toBeUndefined()
    }
    expect(byId('constellation').motionSoundUrl).toBeNull()
    expect(byId('constellation').motion?.kind).toBe('twinkle')
    expect(byId('constellation').motionVideoUrl).toBeNull()
  })
})
