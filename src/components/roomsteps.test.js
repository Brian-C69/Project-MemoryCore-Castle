import { describe, expect, it } from 'vitest'
import { rememberLibraryOpened, rememberedLibraryOpened, roomSteps, signInLine } from './roomsteps.js'

describe('the room door steps', () => {
  it('starts on the Library, then asks to connect Castle', () => {
    expect(signInLine(null, false)).toMatch(/opens the Library/)
    expect(signInLine(null, true)).toMatch(/Connect Castle/)
    expect(roomSteps(null)[0].state).toBe('todo')
  })

  it('says who is signed in, and a run-out sign-in connects again', () => {
    expect(signInLine({ signedIn: true, name: 'Rin' }, true)).toBe('Signed in as Rin.')
    expect(signInLine({ signedIn: false, expired: true }, false)).toMatch(/ran out.*Connect Castle/)
  })

  it('walks the card and the Foundation', () => {
    const m = { signedIn: true, card: 'active', foundation: 'PENDING' }
    const [one, two, three] = roomSteps(m, true)
    expect([one.state, two.state, three.state]).toEqual(['done', 'done', 'wait'])
    expect(roomSteps({ ...m, foundation: 'APPROVED', roomOpen: true })[2].line).toBe('You are in.')
    expect(roomSteps({ signedIn: true, card: 'inactive' })[1].line).toMatch(/not active yet/)
  })

  it('remembers the Library press, and survives blocked storage', () => {
    const box = new Map()
    const storage = { getItem: (k) => box.get(k) ?? null, setItem: (k, v) => box.set(k, v) }
    expect(rememberedLibraryOpened(storage)).toBe(false)
    rememberLibraryOpened(storage)
    expect(rememberedLibraryOpened(storage)).toBe(true)
    const blocked = { getItem: () => { throw new Error('no') }, setItem: () => { throw new Error('no') } }
    expect(rememberedLibraryOpened(blocked)).toBe(false)
    expect(() => rememberLibraryOpened(blocked)).not.toThrow()
  })
})
