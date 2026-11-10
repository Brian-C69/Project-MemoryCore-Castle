import { describe, expect, it } from 'vitest'
import {
  bookPages, calendarWeeks, DIARY_LOOKS, longDate, lookOf, monthsOf, openSpread, spreadAt, spreadLabel, spreadStart, startMonth, stepMonth, turn, undatedOf,
} from './diaryviews.js'

const page = (date, title, n = 1) => ({ title, date, file: `db/diary/${date ?? 'x'}-${n}.md` })
// as Castle gives them: newest month first, newest page first inside a month
const shelves = [
  { name: '2026-11', pages: [page('2026-11-10', 'Today'), page('2026-11-08', 'Evening', 2), page('2026-11-08', 'Morning', 1), page('2026-11-02', 'Second')] },
  { name: '2026-10', pages: [page('2026-10-27', 'Late October'), page('2026-10-12', 'Mid October')] },
  { name: 'Earlier', pages: [page(null, 'First Day')] },
]

describe('the three designs', () => {
  it('are list, calendar and book, and anything else is the list', () => {
    expect(DIARY_LOOKS.map((l) => l.id)).toEqual(['list', 'calendar', 'book'])
    expect(lookOf('book')).toBe('book')
    expect(lookOf('scroll')).toBe('list')
    expect(lookOf(undefined)).toBe('list')
  })
})

describe('the calendar', () => {
  it('knows the months that hold a page, oldest first', () => {
    expect(monthsOf(shelves)).toEqual(['2026-10', '2026-11'])
    expect(monthsOf([])).toEqual([])
    expect(undatedOf(shelves).map((p) => p.title)).toEqual(['First Day'])
  })

  it('lays a month out in weeks that start on Monday', () => {
    const weeks = calendarWeeks('2026-11', shelves)
    // 1 November 2026 is a Sunday: six empty cells, then the 1st
    expect(weeks[0].slice(0, 6)).toEqual([null, null, null, null, null, null])
    expect(weeks[0][6].day).toBe(1)
    expect(weeks[1][0].date).toBe('2026-11-02')
    expect(weeks.every((w) => w.length === 7)).toBe(true)
    expect(weeks.flat().filter(Boolean).length).toBe(30)
  })

  it('puts each page on its day, a day with two in the order they were written', () => {
    const cells = calendarWeeks('2026-11', shelves).flat().filter(Boolean)
    const on = (day) => cells.find((c) => c.day === day).pages.map((p) => p.title)
    expect(on(8)).toEqual(['Morning', 'Evening'])
    expect(on(10)).toEqual(['Today'])
    expect(on(9)).toEqual([])
    expect(cells.filter((c) => c.pages.length).map((c) => c.day)).toEqual([2, 8, 10])
  })

  it('handles a leap February and a month that needs six weeks', () => {
    expect(calendarWeeks('2028-02', []).flat().filter(Boolean).length).toBe(29)
    // August 2026 starts on a Saturday: 31 days over six weeks
    expect(calendarWeeks('2026-08', []).length).toBe(6)
  })

  it('steps across a year in both directions', () => {
    expect(stepMonth('2026-12', 1)).toBe('2027-01')
    expect(stepMonth('2026-01', -1)).toBe('2025-12')
    expect(stepMonth('2026-11', -14)).toBe('2025-09')
  })

  it('opens at the wanted page, else the newest month, else this month', () => {
    expect(startMonth(shelves, '2026-10-12', '2026-11-10')).toBe('2026-10')
    expect(startMonth(shelves, null, '2026-12-01')).toBe('2026-11')
    expect(startMonth([], null, '2026-12-01')).toBe('2026-12')
  })
})

describe('the book', () => {
  const pages = bookPages(shelves)

  it('reads oldest first, undated pages before the rest, a day in writing order', () => {
    expect(pages.map((p) => p.title)).toEqual(['First Day', 'Mid October', 'Late October', 'Second', 'Morning', 'Evening', 'Today'])
  })

  it('opens at the newest spread, or at the page asked for', () => {
    // seven pages: spreads start at 0, 2, 4, 6
    expect(openSpread(pages, '')).toBe(6)
    expect(openSpread(pages, 'db/diary/2026-10-27-1.md')).toBe(2)
    expect(openSpread([], '')).toBe(0)
    expect(spreadStart(5)).toBe(4)
  })

  it('shows two pages, and one on a last single page', () => {
    expect(spreadAt(pages, 2)).toEqual({ left: pages[2], right: pages[3] })
    expect(spreadAt(pages, 6)).toEqual({ left: pages[6], right: null })
    expect(spreadAt([], 0)).toEqual({ left: null, right: null })
  })

  it('turns by two and stops at both ends', () => {
    expect(turn(6, -1, 7)).toBe(4)
    expect(turn(0, -1, 7)).toBe(0)
    expect(turn(4, 1, 7)).toBe(6)
    expect(turn(6, 1, 7)).toBe(6)
    // an even number of pages: the last spread is full
    expect(turn(2, 1, 4)).toBe(2)
    expect(turn(0, 1, 0)).toBe(0)
  })

  it('says where the reader is', () => {
    expect(spreadLabel(2, 12)).toBe('pages 3-4 of 12')
    expect(spreadLabel(12, 13)).toBe('page 13 of 13')
    expect(spreadLabel(0, 0)).toBe('')
  })

  it('writes a date the way a diary does', () => {
    expect(longDate('2026-11-10')).toBe('Tuesday, November 10')
    expect(longDate(null)).toBe('')
  })
})
