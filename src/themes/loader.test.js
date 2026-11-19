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
})

describe('living wallpapers', () => {
  it('gives the two shores and the river their looping videos and leaves the drawn motion to The Constellation alone', () => {
    const themes = builtInThemes()
    const byId = (id) => themes.find((t) => t.id === id)
    for (const id of ['star-shore', 'green-river', 'sunset-shore']) {
      expect(byId(id).motionVideoUrl).toBeTruthy()
      expect(byId(id).motion).toBeUndefined()
    }
    expect(byId('constellation').motion?.kind).toBe('twinkle')
    expect(byId('constellation').motionVideoUrl).toBeNull()
  })
})
