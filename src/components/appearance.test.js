import { describe, expect, it } from 'vitest'
import { improvementLine, referenceLine } from './appearance.js'

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
})
