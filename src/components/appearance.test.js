import { describe, expect, it } from 'vitest'
import { improvementLine, itemReferenceLine, referenceLine } from './appearance.js'

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
