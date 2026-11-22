import { describe, expect, it } from 'vitest'
import { agoText, addDays, agendaDays, cellItems, dayTitle, filterItems, gridRange, groupByDay, iso, itemLine, itemMark, monthGrid, monthTitle, ownerRoom, parseIso, shiftMonth, weekdayNames } from './calendar.js'

describe('calendar dates', () => {
  it('reads only real dates', () => {
    expect(iso(parseIso('2026-11-22'))).toBe('2026-11-22')
    expect(parseIso('2026-02-30')).toBe(null)
    expect(parseIso('22/11/2026')).toBe(null)
    expect(parseIso('')).toBe(null)
  })
  it('adds days across month and year ends', () => {
    expect(addDays('2026-11-30', 1)).toBe('2026-12-01')
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01')
    expect(addDays('2028-02-28', 1)).toBe('2028-02-29')
    expect(addDays('2026-03-01', -1)).toBe('2026-02-28')
  })
  it('names the week from the chosen first day', () => {
    expect(weekdayNames(1)).toEqual(['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'])
    expect(weekdayNames(0)[0]).toBe('Sun')
  })
  it('titles days and months', () => {
    expect(monthTitle(2026, 10)).toBe('November 2026')
    expect(dayTitle('2026-11-22')).toBe('Sunday 22 Nov')
  })
  it('moves month to month across a year', () => {
    expect(shiftMonth(2026, 11, 1)).toEqual({ year: 2027, month: 0 })
    expect(shiftMonth(2026, 0, -1)).toEqual({ year: 2025, month: 11 })
  })
})

describe('month grid', () => {
  it('starts on Monday: November 2026 begins on a Sunday, so the first row starts the 26th of October', () => {
    const weeks = monthGrid(2026, 10, 1)
    expect(weeks[0][0].date).toBe('2026-10-26')
    expect(weeks[0][6].date).toBe('2026-11-01')
    expect(weeks[0][6].inMonth).toBe(true)
    expect(weeks[0][0].inMonth).toBe(false)
    expect(weeks.length).toBe(6)
    expect(weeks[5][0].date).toBe('2026-11-30')
    expect(weeks[5][6].date).toBe('2026-12-06')
  })
  it('starts on Sunday: the first row of November 2026 starts on the 1st', () => {
    const weeks = monthGrid(2026, 10, 0)
    expect(weeks[0][0].date).toBe('2026-11-01')
    expect(weeks.length).toBe(5)
    expect(weeks[4][6].date).toBe('2026-12-05')
  })
  it('every row has seven days and the days run on without gaps', () => {
    for (const start of [0, 1]) {
      for (let m = 0; m < 12; m++) {
        const weeks = monthGrid(2026, m, start)
        const flat = weeks.flat()
        expect(weeks.every((w) => w.length === 7)).toBe(true)
        for (let i = 1; i < flat.length; i++) expect(flat[i].date).toBe(addDays(flat[i - 1].date, 1))
        expect(flat.filter((c) => c.inMonth).length).toBe(new Date(2026, m + 1, 0).getDate())
      }
    }
  })
  it('a month that needs six rows gets six; February 2027 on Monday gets four', () => {
    expect(monthGrid(2026, 7, 1).length).toBe(6)
    expect(monthGrid(2027, 1, 1).length).toBe(4)
  })
  it('reports the first and last day shown, to ask for exactly that window', () => {
    expect(gridRange(monthGrid(2026, 10, 1))).toEqual({ from: '2026-10-26', to: '2026-12-06' })
  })
})

describe('calendar items', () => {
  const items = [
    { kind: 'reminder', title: 'Call', date: '2026-11-22', time: null, owner: 'home' },
    { kind: 'task', title: 'Write', date: '2026-11-22', time: null, owner: 'projects' },
    { kind: 'automation', title: 'Briefing', date: '2026-11-23', time: '09:00', owner: 'automations' },
    { kind: 'diary', title: 'Page', date: '2026-11-22', time: null, owner: 'diary' },
  ]
  it('filters by kind and keeps everything for a kind not mentioned', () => {
    expect(filterItems(items, { task: false }).map((i) => i.kind)).toEqual(['reminder', 'automation', 'diary'])
    expect(filterItems(items, {}).length).toBe(4)
    expect(filterItems(null, {})).toEqual([])
  })
  it('groups by day', () => {
    const by = groupByDay(items)
    expect(by['2026-11-22'].length).toBe(3)
    expect(by['2026-11-23'].length).toBe(1)
  })
  it('shows three items in a cell, or two and "+n more"', () => {
    expect(cellItems(items.slice(0, 3), 3)).toEqual({ shown: items.slice(0, 3), more: 0 })
    const four = cellItems(items, 3)
    expect(four.shown.length).toBe(2)
    expect(four.more).toBe(2)
    expect(cellItems(undefined)).toEqual({ shown: [], more: 0 })
  })
  it('builds an agenda of only the days that have something, inside the window', () => {
    const days = agendaDays([...items, { kind: 'task', title: 'Far', date: '2027-03-01' }, { kind: 'task', title: 'Past', date: '2026-11-01' }], '2026-11-22', 30)
    expect(days.map((d) => d.date)).toEqual(['2026-11-22', '2026-11-23'])
    expect(days[0].items.length).toBe(3)
  })
  it('writes a line with the time when there is one', () => {
    expect(itemLine(items[2])).toBe('09:00 Briefing')
    expect(itemLine(items[0])).toBe('Call')
  })
  it('marks a holiday and says how long ago an update was', () => {
    expect(itemMark({ kind: 'holiday', state: 'ok' })).toBe('#')
    const now = new Date(2026, 10, 22, 12, 0)
    expect(agoText('2026-11-22 12:00', now)).toBe('just now')
    expect(agoText('2026-11-22 11:59', now)).toBe('1 minute ago')
    expect(agoText('2026-11-22 11:30', now)).toBe('30 minutes ago')
    expect(agoText('2026-11-22 11:00', now)).toBe('1 hour ago')
    expect(agoText('2026-11-22 03:00', now)).toBe('9 hours ago')
    expect(agoText('2026-11-20 12:00', now)).toBe('2 days ago')
    expect(agoText(null, now)).toBe('not yet')
    expect(agoText('2026-11-22 13:00', now)).toBe('just now')
  })
  it('marks a suggestion with a question mark', () => {
    expect(itemMark({ kind: 'event', state: 'proposed' })).toBe('?')
    expect(itemMark({ kind: 'event', state: 'active' })).toBe('+')
  })
  it('opens only a room Castle really has', () => {
    expect(ownerRoom(items[1])).toBe('projects')
    expect(ownerRoom({ owner: 'somewhere-else' })).toBe(null)
  })
})
