import { describe, expect, it } from 'vitest'
import { MOVES, stateLine } from './projectshelf.js'

describe('project shelves', () => {
  it('offers the moves that make sense from each shelf', () => {
    expect(MOVES.active.map((m) => m.to)).toEqual(['paused', 'archived', 'deleted'])
    expect(MOVES.paused.map((m) => m.to)).toEqual(['active', 'archived', 'deleted'])
    expect(MOVES.archived.map((m) => m.to)).toEqual(['active', 'deleted'])
    expect(MOVES.deleted.map((m) => m.to)).toEqual(['active'])
  })

  it('always asks before a delete and for a pause reason', () => {
    for (const shelf of Object.values(MOVES)) {
      for (const m of shelf) if (m.to === 'deleted') expect(m.ask).toBe('delete')
    }
    expect(MOVES.active[0].ask).toBe('pause')
  })

  it('says how long a deleted project waits', () => {
    expect(stateLine({ shelf: 'active' })).toBe('Active')
    expect(stateLine({ shelf: 'deleted', daysLeft: 30 })).toBe('Deleted · gone for good in 30 days')
    expect(stateLine({ shelf: 'deleted', daysLeft: 1 })).toBe('Deleted · gone for good tomorrow')
    expect(stateLine({ shelf: 'deleted', daysLeft: 0 })).toBe('Deleted · goes today')
    expect(stateLine({ shelf: 'something new' })).toBe('Active')
  })
})
