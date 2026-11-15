// LIBRARY's pure parts: shelves with their counts, search, and what a book's reader says about it.

/** Shelves of `books` (each with `shelf`) as [{ key, label, count }], in order, plus "all" first. */
export function shelvesOf(books = [], labels = {}) {
  const counts = new Map()
  for (const b of books) counts.set(b.shelf, (counts.get(b.shelf) ?? 0) + 1)
  const words = (key) => labels[key] || String(key || 'general').replace(/[-_]/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
  return [{ key: '', label: 'All', count: books.length }, ...[...counts].map(([key, count]) => ({ key, label: words(key), count }))]
}

/** The books on `shelf` ('' = all) whose title, subtitle, summary or excerpt holds every word of `query`. */
export function filterBooks(books = [], shelf = '', query = '') {
  const words = String(query).toLowerCase().split(/\s+/).filter(Boolean)
  return books.filter((b) => {
    if (shelf && b.shelf !== shelf) return false
    const hay = [b.title, b.subtitle, b.summary, b.excerpt].filter(Boolean).join(' ').toLowerCase()
    return words.every((w) => hay.includes(w))
  })
}

/** The line above a Spiral book in the reader: '' when it is whole. */
export function previewLine(book, signedIn) {
  if (!book || !book.preview) return ''
  if (book.joinToRead) return 'A preview - sign in with your Library card to read the rest, free.'
  return signedIn ? `A preview (${book.previewWords} of ${book.wordCount} words) - this book is not on your shelf yet.` : `A preview (${book.previewWords} of ${book.wordCount} words) - sign in with your Library card to read the books you own.`
}

/** "$3.00" for a priced card, "Free" otherwise. */
export function priceOf(card) {
  if (!card || card.free || !card.priceCents) return 'Free'
  const amount = (card.priceCents / 100).toFixed(2)
  return (card.currency || 'usd').toLowerCase() === 'usd' ? `$${amount}` : `${amount} ${String(card.currency).toUpperCase()}`
}

/**
 * What the reader offers for a Spiral book, given `kept` (ref -> the edition in My books):
 *  'none'   - nothing to keep (no book, a preview, or an empty one)
 *  'keep'   - Add to My books
 *  'kept'   - In My books, the same edition
 *  'update' - In My books, but the Spiral Library has a newer edition
 */
export function keepState(book, kept = {}) {
  if (!book || book.preview || !String(book.body ?? '').trim()) return 'none'
  if (!Object.prototype.hasOwnProperty.call(kept ?? {}, book.ref)) return 'keep'
  return book.updatedAt && kept[book.ref] !== book.updatedAt ? 'update' : 'kept'
}
