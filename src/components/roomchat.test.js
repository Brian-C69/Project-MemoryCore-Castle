import { describe, expect, it } from 'vitest'
import { initial, isMine, readers, ticks } from './roomchat.js'

const me = { name: 'Rin', kind: 'member' }
const line = (id, name, kind) => ({ id, text: 'x', author: { name, kind } })

describe('the room as a chat', () => {
  it('knows the member own lines by id, or by name and kind together', () => {
    expect(isMine(line(5, 'Someone', 'member'), [5], me)).toBe(true)
    expect(isMine(line(6, 'Rin', 'member'), [], me)).toBe(true)
    expect(isMine(line(7, 'Rin', 'member_assistant'), [], me)).toBe(false)
    expect(isMine(line(8, 'Kiyo', 'host'), [], me)).toBe(false)
    expect(isMine({ id: 9, text: 'welcome', author: { kind: 'system' } }, [9], me)).toBe(false)
  })

  it('names who read a line, never the member themself', () => {
    const seen = [
      { name: 'Kiyo', kind: 'host', seen: 12 },
      { name: 'Rin', kind: 'member', seen: 20 },
      { name: 'Alice', kind: 'assistant', seen: 9 },
    ]
    expect(readers(line(10, 'Rin', 'member'), seen, me)).toEqual(['Kiyo'])
    expect(readers(line(13, 'Rin', 'member'), seen, me)).toEqual([])
  })

  it('shows one tick in the room and two once read', () => {
    const seen = [{ name: 'Kiyo', kind: 'host', seen: 12 }]
    expect(ticks(line(10, 'Rin', 'member'), seen, me)).toEqual({ mark: '✓✓', read: true, title: 'Read by Kiyo' })
    expect(ticks(line(15, 'Rin', 'member'), seen, me).mark).toBe('✓')
    expect(ticks({ text: 'sending' }, seen, me).mark).toBe('')
  })

  it('draws a letter, or a spark for an AI, when there is no picture', () => {
    expect(initial({ name: 'kiyo', kind: 'host' })).toBe('K')
    expect(initial({ name: "Rin's Mika", kind: 'member_assistant' })).toBe('✦')
    expect(initial({})).toBe('?')
  })
})
