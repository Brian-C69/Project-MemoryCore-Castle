import { describe, expect, it } from 'vitest'
import { PICKER, REACTIONS, fileLoader, initial, isMine, isOneEmoji, reactionChips, readers, replySnippet, replyingLine, roomFeatures, ticks, toggleReaction } from './roomchat.js'

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
    expect(roomFeatures({})).toEqual({ replies: false, reactions: false, customReactions: false })
    expect(roomFeatures({ features: ['replies'] })).toEqual({ replies: true, reactions: false, customReactions: false })
    expect(roomFeatures({ features: ['reactions', 'customReactions'] }).customReactions).toBe(true)
  })
  it('chips the six first, then any other emoji the room sent, in its order', () => {
    const line = { reactions: [{ emoji: '🔥', count: 3 }, { emoji: '😂', count: 1 }, { emoji: '🇲🇾', count: 1, mine: true }, { emoji: '👍', count: 2 }, { emoji: '🎉', count: 0 }] }
    expect(reactionChips(line).map((r) => r.emoji)).toEqual(['👍', '😂', '🔥', '🇲🇾'])
  })
  it('knows one emoji from words, two emoji or plain text', () => {
    for (const ok of ['🔥', '🇲🇾', '1️⃣', '👨‍👩‍👧', '👍🏽', '❤️', ' 🎉 ']) expect(isOneEmoji(ok), ok).toBe(true)
    for (const no of ['', ' ', 'hi', 'a😀', '🔥🔥', ':)', '1', null]) expect(isOneEmoji(no), String(no)).toBe(false)
  })
  it('offers a picker of single emoji beyond the six', () => {
    expect(PICKER.length).toBeGreaterThanOrEqual(30)
    expect(PICKER.every(isOneEmoji)).toBe(true)
    expect(PICKER.some((e) => REACTIONS.includes(e))).toBe(false)
    expect(new Set(PICKER).size).toBe(PICKER.length)
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

describe('loading the room\'s pictures and voice lines', () => {
  const later = () => new Promise((resolve) => setTimeout(resolve, 0))

  it('keeps every file when several finish together, in any order', async () => {
    // the page adds each file to what it holds when the file ARRIVES
    let files = {}
    const arrive = {}
    const fetchFile = (id) => new Promise((resolve) => { arrive[id] = () => resolve(`data:image/jpeg;base64,${id}`) })
    const load = fileLoader(fetchFile, (id, data) => { files = { ...files, [id]: data } })
    const all = [load('11'), load('12'), load('13')]
    await later() // the loader starts each request a moment after it is asked
    arrive['12']()
    arrive['13']()
    arrive['11']()
    await Promise.all(all)
    expect(files).toEqual({ 11: 'data:image/jpeg;base64,11', 12: 'data:image/jpeg;base64,12', 13: 'data:image/jpeg;base64,13' })
  })

  it('asks once per file however many lines want it, and never again once it is there', async () => {
    let asked = 0
    const load = fileLoader(async () => { asked += 1; return 'data:x' }, () => {})
    await Promise.all([load('7'), load('7'), load('7')])
    await load('7')
    expect(asked).toBe(1)
  })

  it('asks again after a wait when a file failed, not before, and not forever as "loading"', async () => {
    let clock = 1000
    let fails = 1
    let asked = 0
    const got = []
    const load = fileLoader(async () => { asked += 1; if (fails-- > 0) throw new Error('room not ready'); return 'data:ok' }, (id, data) => got.push([id, data]), { now: () => clock, retryMs: 10000 })
    await load('9')
    expect(asked).toBe(1)
    clock += 5000
    await load('9')
    expect(asked, 'too soon to ask again').toBe(1)
    clock += 6000
    await load('9')
    expect(asked).toBe(2)
    expect(got).toEqual([['9', 'data:ok']])
  })

  it('treats an empty answer as a failure and tells the page, so it can ask again later', async () => {
    const failedIds = []
    const load = fileLoader(async () => '', () => { throw new Error('must not be called') }, { onFail: (id) => failedIds.push(id) })
    await load('3')
    await later()
    expect(failedIds).toEqual(['3'])
  })

  it('ignores a line with no file', async () => {
    let asked = 0
    const load = fileLoader(async () => { asked += 1; return 'x' }, () => {})
    await load(undefined)
    await load(null)
    await load('')
    expect(asked).toBe(0)
  })
})
