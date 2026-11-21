import { describe, expect, it } from 'vitest'
import { MARKING_WHY, viewerControls, viewerIndex } from './pictureviewer.js'

describe('the picture viewer, looking and marking', () => {
  it('offers zoom, next and previous, and the pen while looking', () => {
    const c = viewerControls({ count: 3, markable: true })
    expect(c.zoom).toEqual({ shown: true, on: true, why: '' })
    expect(c.nav).toEqual({ shown: true, on: true, why: '' })
    expect(c.markup).toEqual({ shown: true, on: true, why: '' })
    expect(c.keys).toBe(true)
  })

  it('rests every control while the marks are not yet kept, and says why', () => {
    const c = viewerControls({ count: 3, markable: true, marking: true })
    for (const one of [c.zoom, c.nav, c.markup]) expect(one).toEqual({ shown: true, on: false, why: MARKING_WHY })
    expect(c.keys).toBe(false)
  })

  it('brings the controls back once marking ends (kept or left)', () => {
    const during = viewerControls({ count: 2, markable: true, marking: true })
    const after = viewerControls({ count: 2, markable: true, marking: false })
    expect(during.nav.on).toBe(false)
    expect(after.nav.on).toBe(true)
    expect(after.zoom.on).toBe(true)
  })

  it('shows no next and previous for a single picture, and no pen for one that cannot be marked', () => {
    const c = viewerControls({ count: 1, markable: false })
    expect(c.nav).toEqual({ shown: false, on: false, why: '' })
    expect(c.markup).toEqual({ shown: false, on: false, why: '' })
    expect(c.zoom.on).toBe(true)
    // a hidden control gives no reason, even while marking
    expect(viewerControls({ count: 1, marking: true }).nav.why).toBe('')
  })

  it('stands on the marked copy where the original was, or the first picture when it is gone', () => {
    expect(viewerIndex(['a.png', 'b-marked.png', 'c.png'], 'b-marked.png')).toBe(1)
    expect(viewerIndex(['a.png'], 'gone.png')).toBe(0)
    expect(viewerIndex([], 'x.png')).toBe(0)
  })
})
