import { describe, expect, it } from 'vitest'
import { MOVES, kidsQuestion, progressLine, reorder, stateLine, treeRows } from './projectshelf.js'

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

describe('sub-projects and tasks', () => {
  const shelves = [
    {
      name: 'Active',
      pages: [
        { title: 'Montee', file: 'db/project/2.md' },
        { title: 'Castle', file: 'db/project/1.md', tree: { kids: 2, done: 1, tasks: 4 } },
        { title: 'Update chip', file: 'db/project/3.md', tree: { parent: 'db/project/1.md', kids: 0, done: 1, tasks: 2 } },
        { title: 'Codex usage', file: 'db/project/4.md', tree: { parent: 'db/project/1.md', kids: 0, done: 0, tasks: 2 } },
      ],
    },
    { name: 'Paused', pages: [{ title: 'Affin', file: 'db/project/9.md', tree: { parent: 'db/project/8.md', kids: 0, done: 0, tasks: 0 } }] },
  ]
  const draw = (s) => s.map((x) => [x.name, x.rows.map((r) => `${'  '.repeat(r.depth)}${r.page.title}`)])

  it('draws each sub-project under its parent', () => {
    expect(draw(treeRows(shelves))).toEqual([
      ['Active', ['Montee', 'Castle', '  Update chip', '  Codex usage']],
      // a sub-project whose parent is not on this shelf stands on its own
      ['Paused', ['Affin']],
    ])
    expect(treeRows(shelves)[0].rows[1]).toMatchObject({ kids: 2, open: true })
  })

  it('hides the sub-projects of a closed parent', () => {
    expect(draw(treeRows(shelves, '', new Set(['db/project/1.md'])))[0][1]).toEqual(['Montee', 'Castle'])
  })

  it('keeps and opens a parent whose sub-project matches a search', () => {
    expect(draw(treeRows(shelves, 'codex', new Set(['db/project/1.md'])))).toEqual([['Active', ['Castle', '  Codex usage']]])
    expect(draw(treeRows(shelves, 'castle'))).toEqual([['Active', ['Castle', '  Update chip', '  Codex usage']]])
    expect(treeRows(shelves, 'nothing like it')).toEqual([])
  })

  it('asks about the sub-projects beside a parent before it moves', () => {
    expect(kidsQuestion({ shelf: 'active', kidsBeside: 4 }, 'paused')).toEqual({ verb: 'pause', question: 'Also pause its 4 sub-projects?', all: 'Pause all of them', one: 'Only the parent' })
    expect(kidsQuestion({ shelf: 'active', kidsBeside: 1 }, 'archived').question).toBe('Also archive its sub-project?')
    expect(kidsQuestion({ shelf: 'deleted', kidsBeside: 2 }, 'active').verb).toBe('restore')
    expect(kidsQuestion({ shelf: 'paused', kidsBeside: 2 }, 'active').verb).toBe('resume')
    expect(kidsQuestion({ shelf: 'active', kidsBeside: 0 }, 'paused')).toBeNull()
  })

  it('says how far the tasks are', () => {
    expect(progressLine(3, 8)).toBe('3 of 8 tasks done · 38%')
    expect(progressLine(0, 0)).toBe('')
  })

  it('moves a dragged task to its new place', () => {
    expect(reorder([1, 2, 3, 4], 0, 2)).toEqual([2, 3, 1, 4])
    expect(reorder([1, 2, 3, 4], 3, 0)).toEqual([4, 1, 2, 3])
    expect(reorder([1, 2], 0, 5)).toEqual([1, 2])
  })
})
