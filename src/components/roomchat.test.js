import { describe, expect, it } from 'vitest'
import { initial, isMine, reactionChips, readers, replySnippet, replyingLine, roomFeatures, ticks, toggleReaction } from './roomchat.js'

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


describe('reactions and replies in the room', () => {
  it('shows them only when the room lists them', () => {
    expect(roomFeatures({})).toEqual({ replies: false, reactions: false })
    expect(roomFeatures({ features: ['replies'] })).toEqual({ replies: true, reactions: false })
  })
  it('chips the reactions with a count, in order', () => {
    const line = { reactions: [{ emoji: '😂', count: 1, mine: false }, { emoji: '👍', count: 2, mine: true }, { emoji: '😢', count: 0 }] }
    expect(reactionChips(line).map((r) => r.emoji)).toEqual(['👍', '😂'])
    expect(reactionChips({})).toEqual([])
  })
  it('a press adds a reaction, or takes back the member own', () => {
    const line = { reactions: [{ emoji: '👍', count: 2, mine: true }] }
    expect(toggleReaction(line, '👍')).toBe(false)
    expect(toggleReaction(line, '❤️')).toBe(true)
  })
  it('quotes a reply, loaded or not, and a removed original', () => {
    const lines = [{ id: 4, text: 'the lake' }]
    expect(replySnippet({ replyTo: { id: 4, name: 'Mira', text: 'the  lake' } }, lines)).toEqual({ id: 4, name: 'Mira', text: 'the lake', loaded: true, gone: false })
    expect(replySnippet({ replyTo: { id: 9, name: 'Mira', text: '' } }, lines)).toEqual({ id: 9, name: 'Mira', text: 'a removed message', loaded: false, gone: true })
    expect(replySnippet({ text: 'hi' })).toBeNull()
  })
  it('names the line being answered above the composer', () => {
    expect(replyingLine({ text: 'hello there' }, 'Mira')).toBe('Mira: hello there')
    expect(replyingLine({ text: 'x'.repeat(100) }, 'Mira')).toHaveLength(87)
  })
})
