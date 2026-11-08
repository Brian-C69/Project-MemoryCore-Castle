import { describe, expect, it } from 'vitest'
import { localSend, mergeEntries, resetEntries } from './ticks.js'

const states = (list) => list.map((m) => [m.role, m.text, m.state ?? null])

describe("the member's message ticks", () => {
  it('sent while the companion works: sending, then queued, then read when it is taken in mid-turn', () => {
    let list = [localSend('add upload and download')]
    expect(states(list)).toEqual([['member', 'add upload and download', 'sending']])
    list = mergeEntries(list, [{ id: 'q1', role: 'member', text: 'add upload and download', state: 'queued' }])
    expect(states(list)).toEqual([['member', 'add upload and download', 'queued']])
    list = mergeEntries(list, [{ id: 'r1', role: 'receipt', text: 'add upload and download', state: 'absorbed_mid_turn' }])
    expect(states(list)).toEqual([['member', 'add upload and download', 'read']])
  })

  it('queued, then read as the next turn: shown once', () => {
    let list = mergeEntries([], [{ id: 'q1', role: 'member', text: 'one  more\nthing', state: 'queued' }])
    list = mergeEntries(list, [
      { id: 'r1', role: 'receipt', text: 'one more thing', state: 'dequeued' },
      { id: 'u1', role: 'member', text: 'one more thing' },
      { id: 'a1', role: 'companion', text: 'yes' },
    ])
    expect(states(list)).toEqual([['member', 'one  more\nthing', 'read'], ['companion', 'yes', null]])
  })

  it('sent while the companion is idle: sending, then read', () => {
    let list = [{ id: 'a0', role: 'companion', text: 'hi' }, localSend('hello')]
    list = mergeEntries(list, [{ id: 'u1', role: 'member', text: 'hello' }])
    expect(states(list)).toEqual([['companion', 'hi', null], ['member', 'hello', 'read']])
  })

  it('a queued message taken back leaves Chat', () => {
    let list = mergeEntries([], [{ id: 'q1', role: 'member', text: 'oops', state: 'queued' }])
    list = mergeEntries(list, [{ id: 'r1', role: 'receipt', text: 'oops', state: 'popped_for_edit' }])
    expect(list).toEqual([])
  })

  it('a full reload keeps only the just-sent messages the record does not hold yet', () => {
    const old = [localSend('kept'), localSend('answered')]
    const list = resetEntries(old, [{ id: 'u1', role: 'member', text: 'answered' }])
    expect(states(list)).toEqual([['member', 'answered', 'read'], ['member', 'kept', 'sending']])
  })
})
