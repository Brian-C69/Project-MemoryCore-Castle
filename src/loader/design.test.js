import { describe, expect, it } from 'vitest'
import { MAX_STORED_IMAGE, forStorage, fromStorage, linesFor, loaderDesign, safeColour } from './design.js'

describe("a theme's say over the start-up screen", () => {
  it('asks for nothing when the theme has no loader block, or an empty one', () => {
    expect(loaderDesign({ id: 'a' })).toBeNull()
    expect(loaderDesign({ loader: {} })).toBeNull()
    expect(loaderDesign({ loader: 'nope' })).toBeNull()
  })

  it('reads and cleans what a theme asks for', () => {
    const d = loaderDesign({
      loader: { brand: '  SAKURA NIGHT  ', quips: [' Counting petals. ', '', 5, 'x'.repeat(130)], quipsMode: 'replace', accent: '#ff9ccf', position: 'center' },
      loaderUrl: '/assets/loader-abc.webp',
    })
    expect(d).toEqual({ brand: 'SAKURA NIGHT', quips: ['Counting petals.'], quipsMode: 'replace', accent: '#ff9ccf', position: 'center', imageUrl: '/assets/loader-abc.webp' })
  })

  it('falls back to Castle\'s own choices for anything odd', () => {
    const d = loaderDesign({ loader: { brand: 'x'.repeat(80), quipsMode: 'explode', position: 'sideways', accent: 'red; background:url(x)' } })
    expect(d.brand.length).toBe(40)
    expect(d.quipsMode).toBe('add')
    expect(d.position).toBe('bottom')
    expect(d.accent).toBeNull()
  })

  it('takes the picture only from what the theme loader already read, never from the JSON', () => {
    expect(loaderDesign({ loader: { brand: 'A', image: 'https://evil.example/x.png' } }).imageUrl).toBeNull()
    expect(loaderDesign({ loader: { brand: 'A' }, loaderUrl: 'data:image/webp;base64,AAAA' }).imageUrl).toBe('data:image/webp;base64,AAAA')
  })

  it('lets only plain colours through', () => {
    for (const ok of ['#fff', '#ff9ccf', '#ff9ccfcc', 'rgb(255, 156, 207)', 'rgba(255,156,207,0.5)', 'hsl(330 100% 80%)']) expect(safeColour(ok)).toBe(ok)
    for (const bad of ['url(x)', 'red', '#ggg', 'rgb(1,2,3); color:red', 'rgb(1,2,3) /* x */', '', null, 5]) expect(safeColour(bad)).toBeNull()
  })

  it("adds a theme's lines to Castle's, or puts them in their place", () => {
    const base = ['a', 'b']
    expect(linesFor(base, null)).toEqual(base)
    expect(linesFor(base, { quips: [], quipsMode: 'replace' })).toEqual(base)
    expect(linesFor(base, { quips: ['c'], quipsMode: 'add' })).toEqual(['a', 'b', 'c'])
    expect(linesFor(base, { quips: ['c'], quipsMode: 'replace' })).toEqual(['c'])
  })

  it('remembers a design for the next start, and trusts none of it more than a theme', () => {
    const design = loaderDesign({ loader: { brand: 'HELLO', accent: '#112233' }, loaderUrl: 'data:image/webp;base64,AAAA' })
    const back = fromStorage(JSON.stringify(forStorage(design, '#abcdef')))
    expect(back).toMatchObject({ brand: 'HELLO', accent: '#112233', imageUrl: 'data:image/webp;base64,AAAA' })
    // no loader block: the theme's own accent colour is still remembered
    expect(fromStorage(JSON.stringify(forStorage(null, '#abcdef'))).accent).toBe('#abcdef')
    // a picture too big for storage is left out, the rest kept
    const big = { ...design, imageUrl: 'data:image/webp;base64,' + 'A'.repeat(MAX_STORED_IMAGE) }
    expect(forStorage(big, '#000').imageUrl).toBeNull()
    // a tampered value
    const tampered = fromStorage(JSON.stringify({ brand: 'x', accent: 'url(evil)', position: 'oops', imageUrl: 'javascript:alert(1)' }))
    expect(tampered).toMatchObject({ accent: null, position: 'bottom', imageUrl: null })
    expect(fromStorage('not json')).toBeNull()
    expect(fromStorage(null)).toBeNull()
  })
})
