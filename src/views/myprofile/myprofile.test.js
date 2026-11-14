import { describe, expect, it } from 'vitest'
import { barWidth, CARDS, deltaText, glowing, grouped, monthLong, monthShort, progressLine, tileDeltas, yearMark } from './myprofile.js'

const point = (month, mbti, growth, focus, wellbeing) => ({
  month,
  mbti,
  headline: [
    { grp: 'headline', label: 'Growth', score: growth },
    { grp: 'headline', label: 'Focus', score: focus },
    { grp: 'headline', label: 'Wellbeing', score: wellbeing },
  ],
})

const timeline = [point('2025-11', 'INTJ-A', 70, 60, 65), point('2025-12', 'INTJ-A', 76, 56, 66), point('2026-01', 'INTJ-T', 77, 57, 66), point('2026-02', 'INTJ-T', 77, 70, 50)]

describe('MY PROFILE', () => {
  it('names the months', () => {
    expect(monthShort('2026-03')).toBe('MAR')
    expect(monthLong('2026-03')).toBe('March 2026')
    expect(monthLong('nonsense')).toBe('nonsense')
    expect(timeline.map((_, i) => yearMark(timeline, i))).toEqual(['2025', '', '2026', ''])
  })

  it('measures each tile against the month before', () => {
    expect(tileDeltas(timeline, '2025-12')).toEqual({ Growth: 6, Focus: -4, Wellbeing: 1 })
    expect(tileDeltas(timeline, '2025-11')).toEqual({})
    expect(tileDeltas(timeline, '2030-01')).toEqual({})
    expect([deltaText(6), deltaText(-4), deltaText(0), deltaText(null)]).toEqual(['+6', '-4', '0', ''])
  })

  it('glows on a new type or a big move, never on the first month', () => {
    expect([...glowing(timeline)]).toEqual(['2026-01', '2026-02'])
    expect(glowing([point('2026-01', 'ENFP', 10, 10, 10)]).size).toBe(0)
    // a month with no type is not a change of type
    expect(glowing([point('2026-01', 'ENFP', 10, 10, 10), point('2026-02', null, 12, 10, 10)]).size).toBe(0)
  })

  it('groups a reading for the cards and keeps the bars on their track', () => {
    const g = grouped({
      scores: [
        { grp: 'headline', label: 'Growth', score: 70 },
        { grp: 'think', label: 'Analytical', score: 92 },
        { grp: 'shadow', label: 'Control', score: 69 },
        { grp: 'unknown', label: 'x', score: 1 },
      ],
      notes: [{ grp: 'think', line: 'one' }, { grp: 'patterns', line: 'again' }],
    })
    expect(g.scores.think.map((s) => s.label)).toEqual(['Analytical'])
    expect(g.scores.values).toEqual([])
    expect(g.notes).toMatchObject({ think: ['one'], patterns: ['again'], shadow: [] })
    expect(grouped(null).scores.headline).toEqual([])
    expect(CARDS.map((c) => c.tone)).toEqual(['gold', 'gold', 'violet'])
    expect([barWidth(140), barWidth(-3), barWidth('62')]).toEqual(['100%', '0%', '62%'])
  })

  it('says how the scan is going', () => {
    expect(progressLine({ month: '2026-03', done: 2, total: 9, state: 'reading' })).toBe('Reading March 2026… (3 of 9)')
    expect(progressLine({ month: '2026-03', done: 3, total: 9, state: 'read' })).toBe('March 2026 read (3 of 9)')
    expect(progressLine({ month: '2026-03', done: 3, total: 9, state: 'skipped', problem: 'No reading came back for that month.' })).toContain('passed over')
    expect(progressLine(null)).toBe('')
  })
})
