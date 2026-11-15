import { describe, expect, it } from 'vitest'
import { improvementLine, itemReferenceLine, lookSpec, pictureCost, qualityLine, QUALITIES, referenceLine } from './appearance.js'

describe('a drawing named by its parts', () => {
  it('carries the ids, the trimmed words and the quality', () => {
    expect(lookSpec('tryon', { outfit: 'coat', hair: 'bob', changes: '  warmer  ', quality: 'low' })).toEqual({ kind: 'tryon', outfit: 'coat', hair: 'bob', changes: 'warmer', quality: 'low' })
    expect(lookSpec('standard', { changes: '   ' })).toEqual({ kind: 'standard', outfit: null, hair: null, changes: null, quality: 'style' })
  })
  it('prices the style default as the style file does', () => {
    expect(pictureCost('standard', 'style')).toBe(0.25)
    expect(pictureCost('tryon', 'style')).toBe(0.06)
    expect(pictureCost('outfit', 'low')).toBe(0.03)
  })
})

describe('the chosen quality', () => {
  it('overrides the style only when the member chose one', () => {
    expect(qualityLine('style')).toBe('')
    expect(qualityLine(undefined)).toBe('')
    expect(qualityLine('ultra')).toBe('')
    for (const q of ['low', 'medium', 'high']) expect(qualityLine(q)).toContain(`-Quality ${q}`)
  })
  it('prices every quality but the style default', () => {
    expect(QUALITIES.map((q) => q.id)).toEqual(['style', 'low', 'medium', 'high'])
    expect(QUALITIES.filter((q) => q.usd === null).map((q) => q.id)).toEqual(['style'])
  })
})

describe('appearance image edit requests', () => {
  it('carries reference paths with spaces and quotes as data and preserves identity', () => {
    const file = 'C:\\My Pictures\\Mira "night".png'
    const request = referenceLine(file)
    expect(request).toContain(JSON.stringify(file))
    expect(request).toContain('-Reference')
    expect(request).toContain('Preserve its face and body proportions')
    expect(referenceLine(null)).toBe('')
  })
  it('includes explicit changes without producing an empty instruction', () => {
    expect(improvementLine('  Keep the face; change the dress.  ')).toContain('"Keep the face; change the dress."')
    expect(improvementLine('   ')).toBe('')
  })
  it('describes an outfit or hairstyle picture as the source for that part only, never the reference', () => {
    const file = 'C:\\Wardrobe\\coat "blue".png'
    const outfit = itemReferenceLine('outfit', file)
    expect(outfit).toContain(JSON.stringify(file))
    expect(outfit).toContain('outfit reference image as the source for the clothes only')
    expect(outfit).toContain('Do not pass it as the -Reference')
    expect(outfit).toContain('face, body or identity')
    expect(itemReferenceLine('hair', file)).toContain('hairstyle reference image as the source for the hair only')
    expect(itemReferenceLine('outfit', null)).toBe('')
  })
})
