import { expect, it } from 'vitest'
import { formatClock } from './clock.js'

it('formats local midnight and noon in both clock formats', () => {
  expect(formatClock(new Date(2026, 10, 14, 0, 5), '24h')).toBe('00:05')
  expect(formatClock(new Date(2026, 10, 14, 0, 5), '12h')).toBe('12:05 AM')
  expect(formatClock(new Date(2026, 10, 14, 12, 0), '12h')).toBe('12:00 PM')
})
it('pads minutes and converts afternoon hours without changing the underlying time', () => {
  const date = new Date(2026, 10, 14, 23, 9)
  expect(formatClock(date, '24h')).toBe('23:09')
  expect(formatClock(date, '12h')).toBe('11:09 PM')
  expect(date.getHours()).toBe(23)
})
