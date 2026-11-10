import { describe, expect, it } from 'vitest'
import { level, resetWords, rows, staleWords, windowNow } from './usage.js'

const now = new Date('2026-11-11T03:00:00').getTime()
const hours = (h) => now + h * 3600e3

describe('usage card', () => {
  it('treats a window past its reset time as started over', () => {
    expect(windowNow({ usedPercent: 100, resetsAt: hours(-1) }, now)).toEqual({ usedPercent: 0, resetsAt: null, startedOver: true })
    expect(windowNow({ usedPercent: 40, resetsAt: hours(2) }, now).usedPercent).toBe(40)
    expect(windowNow(null, now)).toBeNull()
  })

  it('warms at 80% and is full at 100%', () => {
    expect([level(79), level(80), level(99.5), level(100)]).toEqual(['calm', 'warm', 'warm', 'full'])
  })

  it('says the reset time, with the weekday when it is not today', () => {
    expect(resetWords(hours(1), now)).toMatch(/^resets 4:00\sAM$/)
    expect(resetWords(new Date('2026-11-13T03:10:00').getTime(), now)).toMatch(/^resets Fri 3:10\sAM$/)
    expect(resetWords(null, now)).toBe('')
  })

  it('marks numbers older than ten minutes', () => {
    expect(staleWords(now - 5 * 60000, now)).toBe('')
    expect(staleWords(new Date('2026-11-11T02:10:00').getTime(), now)).toMatch(/^as of 2:10\sAM$/)
  })

  it('shows two bars for a CLI with numbers', () => {
    const card = rows({ status: 'ok', fiveHour: { usedPercent: 58.4, resetsAt: hours(2) }, weekly: { usedPercent: 61.5, resetsAt: hours(50) }, weeklyModel: 'Opus', observedAt: now }, now)
    expect(card.bars.map((b) => [b.label, b.percent, b.level])).toEqual([['5h', 58, 'calm'], ['Week · Opus', 62, 'calm']])
    expect(card.stale).toBe('')
  })

  it('shows one quiet line when there is nothing to show', () => {
    expect(rows({ status: 'not_installed' }, now)).toEqual({ message: 'Not installed', bars: [] })
    expect(rows({ status: 'signed_out' }, now).message).toBe('Sign in to see usage')
    expect(rows({ status: 'ok', fiveHour: null, weekly: null }, now).message).toBe('Usage not available yet')
  })
})
