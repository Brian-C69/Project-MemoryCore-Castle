import { describe, expect, it } from 'vitest'
import { filterBooks, previewLine, priceOf, shelvesOf } from './library.js'

const books = [
  { shelf: 'pattern', title: 'Keys cannot destroy', excerpt: 'an agent key' },
  { shelf: 'pattern', title: 'Retry with care', excerpt: 'backoff' },
  { shelf: 'architecture', title: 'Static live demo', summary: 'alive without a server' },
]

describe('the library', () => {
  it('counts its shelves, all first', () => {
    expect(shelvesOf(books)).toEqual([
      { key: '', label: 'All', count: 3 },
      { key: 'pattern', label: 'Pattern', count: 2 },
      { key: 'architecture', label: 'Architecture', count: 1 },
    ])
    expect(shelvesOf(books, { pattern: 'Patterns' })[1].label).toBe('Patterns')
  })
  it('filters by shelf and every word of the search', () => {
    expect(filterBooks(books, 'pattern').length).toBe(2)
    expect(filterBooks(books, '', 'without server').map((b) => b.title)).toEqual(['Static live demo'])
    expect(filterBooks(books, 'pattern', 'server')).toEqual([])
  })
  it('says when a Spiral book is only a preview', () => {
    expect(previewLine({ preview: false }, false)).toBe('')
    expect(previewLine({ preview: true, joinToRead: true }, false)).toContain('free')
    expect(previewLine({ preview: true, previewWords: 100, wordCount: 1000 }, false)).toContain('sign in')
    expect(previewLine({ preview: true, previewWords: 100, wordCount: 1000 }, true)).toContain('not on your shelf')
  })
  it('prices a card', () => {
    expect(priceOf({ free: true })).toBe('Free')
    expect(priceOf({ priceCents: 300, currency: 'usd' })).toBe('$3.00')
    expect(priceOf({ priceCents: 1000, currency: 'myr' })).toBe('10.00 MYR')
  })
})
