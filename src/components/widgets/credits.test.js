import { describe, expect, it } from 'vitest'
import { agoLabel, barOf, dimLine, isCreditWidget, monthNow, parseUsd, providerOf, valueLine, wantsSignIn } from './credits.js'

const NOW = Date.UTC(2026, 10, 16, 3, 0, 0)
const MIN = 60000

describe('HOME credit widgets', () => {
  it('maps a widget to its service and key', () => {
    expect(providerOf('credit-elevenlabs')).toBe('elevenlabs')
    expect(isCreditWidget('credit-suno')).toBe(true)
    expect(isCreditWidget('usage')).toBe(false)
  })

  it('says how long ago plainly', () => {
    expect(agoLabel(NOW - 20000, NOW)).toBe('just now')
    expect(agoLabel(NOW - 8 * MIN, NOW)).toBe('8 min ago')
    expect(agoLabel(NOW - 3 * 60 * MIN, NOW)).toBe('3 h ago')
    expect(agoLabel(NOW - 72 * 60 * MIN, NOW)).toBe('3 d ago')
    expect(agoLabel(null, NOW)).toBe('')
  })

  it('shows OpenAI as read, counted down, or typed', () => {
    const read = { provider: 'openai', value: 18.22, whole: 18.22, base: 18.22, at: NOW - 8 * MIN, source: 'openai', callsSince: 0, estimate: false }
    expect(valueLine(read)).toBe('USD 18.22')
    expect(dimLine(read, NOW)).toBe('from OpenAI · 8 min ago')
    const spent = { ...read, value: 17.47, estimate: true, callsSince: 2 }
    expect(valueLine(spent)).toBe('~ USD 17.47')
    expect(dimLine(spent, NOW)).toBe('from OpenAI · 8 min ago · 2 calls since')
    const typed = { ...read, source: 'hand', estimate: true, at: NOW - 30000 }
    expect(dimLine(typed, NOW)).toBe('set just now')
    expect(valueLine({ provider: 'openai', value: null })).toBe('—')
    expect(dimLine({ provider: 'openai', value: null }, NOW)).toBe('Sign in, or set your balance')
  })

  it('tells a sign-out and a silent page apart from a figure', () => {
    const out = { provider: 'openai', value: 18.22, at: NOW - MIN, state: { state: 'signed-out', at: NOW } }
    expect(dimLine(out, NOW)).toBe('signed out of OpenAI - sign in again')
    const quiet = { provider: 'seedance', value: 2.45, base: 2.45, at: NOW - 60 * MIN, source: 'byteplus', state: { state: 'no-answer', at: NOW - 5 * MIN } }
    expect(dimLine(quiet, NOW)).toBe("BytePlus's page did not answer 5 min ago")
    expect(wantsSignIn(out)).toBe(true)
    expect(wantsSignIn({ provider: 'openai', state: { state: 'signed-in', at: NOW } })).toBe(false)
    expect(wantsSignIn({ provider: 'suno' })).toBe(false)
  })

  it('shows the Seedance month and its renders', () => {
    const month = { provider: 'seedance', value: 2.5, base: 2.0, at: NOW - 3 * MIN, source: 'byteplus', callsSince: 1, estimate: true }
    expect(valueLine(month)).toBe('~ USD 2.50')
    expect(dimLine(month, NOW)).toBe('this month · from BytePlus · 3 min ago · 1 render')
    expect(dimLine({ provider: 'seedance', value: 0, base: null, callsSince: 0 }, NOW)).toBe('this month · counted here')
  })

  it('shows Suno credits and ElevenLabs characters left', () => {
    expect(valueLine({ provider: 'suno', value: 768 })).toBe('768')
    expect(dimLine({ provider: 'suno', value: 768 }, NOW)).toBe('credits')
    const eleven = { provider: 'elevenlabs', value: 19412, limit: 37670, resetsAt: Date.UTC(2026, 11, 3, 12) }
    expect(valueLine(eleven)).toBe('19,412')
    expect(dimLine(eleven, NOW)).toBe('left of 37,670 characters · resets Dec 3')
    expect(dimLine({ provider: 'suno', value: null, note: 'Suno refused this key. Check it in Settings > Keys.' }, NOW)).toContain('Suno refused')
  })

  it('draws a bar only from an honest whole', () => {
    expect(barOf({ provider: 'suno', value: 700, whole: 768 })).toEqual({ fill: 700 / 768, low: false })
    expect(barOf({ provider: 'openai', value: 1, whole: 18.22 }).low).toBe(true)
    expect(barOf({ provider: 'seedance', value: 9, whole: 10 }).low).toBe(true)
    expect(barOf({ provider: 'seedance', value: 2, whole: null })).toBe(null)
    expect(barOf({ provider: 'suno', value: null, whole: 768 })).toBe(null)
  })

  it('reads a typed amount and the member month', () => {
    expect(parseUsd('12.50')).toBe(12.5)
    expect(parseUsd('USD 1,250')).toBe(1250)
    expect(parseUsd('$3')).toBe(3)
    expect(parseUsd('abc')).toBe(null)
    expect(parseUsd('1.234')).toBe(null)
    const m = monthNow(new Date(2026, 10, 16, 10))
    expect(m.month).toBe('2026-11')
    expect(m.start).toBe(new Date(2026, 10, 1).getTime())
  })
})
