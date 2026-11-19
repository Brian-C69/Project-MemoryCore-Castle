import { describe, expect, it } from 'vitest'
import { asksFor, choicesFor, greetingOf, itemFacts, shopFacts, shopOf, startStep } from './shopscene.js'

const detail = {
  slug: 'miras-atelier',
  name: "Mira's Atelier",
  keeper: { name: 'Mira', picture: 'https://afif-pms.sgp1.digitaloceanspaces.com/k.png' },
  backdrop: 'https://afif-pms.sgp1.digitaloceanspaces.com/b.png',
  greeting: 'Welcome to my atelier!',
  keeperCard: 'Mira paints postcards.',
  seller: { name: 'Mira' },
  items: [{ id: 42, name: 'Sakura Postcard', priceMyr: 9.9 }],
}
const postcard = {
  id: 42,
  name: 'Sakura Postcard',
  description: 'Turns your picture into a sakura postcard.',
  priceMyr: 9.9,
  votes: { up: 12, down: 1 },
  updatedAt: '2026-11-19T12:00:00Z',
  seller: { name: 'Mira' },
  prompt: 'SECRET',
  blocks: [
    { type: 'picture', label: 'Your picture' },
    { type: 'describe', label: 'Write prompt' },
    { type: 'chips', label: '' },
    { type: 'create', label: '' },
  ],
}

describe('a visit to a shop', () => {
  it('starts at the greeting, or at the explanation when an item was picked in the town', () => {
    expect(startStep(null)).toBe('greet')
    expect(startStep(postcard)).toBe('explain')
  })

  it('a shop from the door, or just the seller before the shops door exists', () => {
    const shop = shopOf(detail, null)
    expect(shop).toMatchObject({ slug: 'miras-atelier', keeperName: 'Mira', greeting: 'Welcome to my atelier!', keeperCard: 'Mira paints postcards.' })
    const stall = shopOf(null, postcard)
    expect(stall).toMatchObject({ slug: '', name: "Mira's stall", keeperName: 'Mira', keeperPicture: '' })
    expect(stall.items).toEqual([postcard])
  })

  it('the greeting is the owner’s line, or a plain welcome - never an AI', () => {
    expect(greetingOf(shopOf(detail))).toBe('Welcome to my atelier!')
    expect(greetingOf(shopOf(null, postcard))).toBe("Welcome to Mira's stall. Would you like to see what is on the counter?")
  })

  it('it asks for what its input blocks ask, not its steps', () => {
    expect(asksFor(postcard.blocks)).toEqual(['Your picture', 'A choice'])
    expect(asksFor(undefined)).toEqual([])
  })

  it('the keeper is handed public facts only', () => {
    const facts = itemFacts(postcard)
    expect(facts).toEqual({
      id: '42', name: 'Sakura Postcard', kind: 'workflow', description: 'Turns your picture into a sakura postcard.',
      priceMyr: 9.9, votesUp: 12, votesDown: 1, updatedAt: '2026-11-19T12:00:00Z',
      blocks: [{ type: 'picture', label: 'Your picture' }, { type: 'describe', label: 'Write prompt' }, { type: 'chips', label: '' }, { type: 'create', label: '' }],
    })
    expect(JSON.stringify(facts)).not.toContain('SECRET')
    expect(shopFacts(shopOf(detail))).toEqual({ name: "Mira's Atelier", keeperName: 'Mira', keeperCard: 'Mira paints postcards.', seller: 'Mira' })
  })

  it('each step offers its own choices', () => {
    expect(choicesFor('greet').map((c) => c.id)).toEqual(['items', 'question', 'leave'])
    expect(choicesFor('explain', postcard).map((c) => c.label)).toEqual(['Ask a question', 'Buy for RM 9.90', 'See other items'])
    expect(choicesFor('question', postcard).map((c) => c.id)).toEqual(['buy', 'items'])
    expect(choicesFor('explain', null).map((c) => c.id)).toEqual(['question', 'items'])
  })
})
