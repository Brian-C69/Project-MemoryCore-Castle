import { describe, expect, it } from 'vitest'
import { installState, installedListings, priceText, publishAction, shopForm, shopFormProblem, shopVisible, votesText } from './shop.js'

const signedIn = (marketplace) => ({ signedIn: true, marketplace })
const yours = { id: 'two-cats', source: 'yours', workflow: { name: 'Two cats' } }

describe('the Marketplace on the page', () => {
  it('the tab shows only when the Portfolio says it is live, for a signed-in member', () => {
    expect(shopVisible(null)).toBe(false)
    expect(shopVisible({ signedIn: true })).toBe(false)
    expect(shopVisible(signedIn({ live: false }))).toBe(false)
    expect(shopVisible({ signedIn: false, marketplace: { live: true } })).toBe(false)
    expect(shopVisible(signedIn({ live: true }))).toBe(true)
  })

  it('Publish shows on the member’s own working workflows, closed until selling opens', () => {
    expect(publishAction(signedIn({ live: false, selling: true }), yours).show).toBe(false)
    expect(publishAction(signedIn({ live: true, selling: false }), yours)).toEqual({ show: true, enabled: false, label: 'Selling opens soon' })
    expect(publishAction(signedIn({ live: true, selling: true }), yours)).toEqual({ show: true, enabled: true, label: 'Publish' })
    expect(publishAction(signedIn({ live: true, selling: true }), { ...yours, source: 'built-in' }).show).toBe(false)
    expect(publishAction(signedIn({ live: true, selling: true }), { ...yours, source: 'bought' }).show).toBe(false)
    expect(publishAction(signedIn({ live: true, selling: true }), { ...yours, error: 'broken' }).show).toBe(false)
  })

  it('prices and votes read as words', () => {
    expect(priceText(9.9)).toBe('RM 9.90')
    expect(priceText('x')).toBe('')
    expect(votesText({ up: 0, down: 0 })).toBe('No votes yet')
    expect(votesText({ up: 12, down: 1 })).toBe('👍 12 · 👎 1')
  })

  it('a purchase already on the rail reads as installed', () => {
    const installed = installedListings([{ source: 'bought', listing: '42' }, { source: 'yours' }, { source: 'bought' }])
    expect([...installed]).toEqual(['42'])
    expect(installState({ listingId: 42 }, installed)).toBe('installed')
    expect(installState({ listingId: 7 }, installed)).toBe('install')
  })

  it('My shop starts from what the Portfolio holds, or the companion as keeper', () => {
    expect(shopForm(null, 'Lyra')).toEqual({ name: '', keeperName: 'Lyra', greeting: '', keeperCard: '', open: true })
    expect(shopForm({ name: 'Mira', keeperName: 'Mira', greeting: 'Hi', keeperCard: 'c', open: false }).open).toBe(false)
  })

  it('My shop is checked before it is saved', () => {
    const good = { name: "Mira's Atelier", keeperName: 'Mira', greeting: 'Welcome!', keeperCard: 'I paint.', open: true }
    expect(shopFormProblem(good)).toBe('')
    expect(shopFormProblem({ ...good, name: '  ' })).toContain('shop a name')
    expect(shopFormProblem({ ...good, keeperName: 'x'.repeat(41) })).toContain('keeper a name')
    expect(shopFormProblem({ ...good, greeting: 'x'.repeat(161) })).toContain('greeting')
    expect(shopFormProblem({ ...good, keeperCard: 'x'.repeat(1201) })).toContain('keeper card')
    expect(shopFormProblem(good, 'C:/m/keeper.gif')).toContain('PNG, JPEG or WebP')
    expect(shopFormProblem(good, 'C:/m/keeper.PNG', 'C:/m/back.webp')).toBe('')
  })
})
