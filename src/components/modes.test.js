import { describe, expect, it } from 'vitest'
import { contextTone, pressesTo, readContext, readMode, SHIFT_TAB, WARM_FROM } from './modes.js'

const box = ['╭──────────────────────────────────────╮', '│ >                                    │', '╰──────────────────────────────────────╯']

describe('the mode from the Log screen', () => {
  it('reads each footer', () => {
    expect(readMode([...box, '  ⏵⏵ accept edits on (shift+tab to cycle)'])).toBe('accept')
    expect(readMode([...box, '  ⏸ plan mode on (shift+tab to cycle)'])).toBe('plan')
    expect(readMode([...box, '  ⏵⏵ auto mode on (shift+tab to cycle)'])).toBe('auto')
    expect(readMode([...box, '  ⏵⏵ bypass permissions on (shift+tab to cycle)'])).toBe('auto')
    expect(readMode([...box, '  ? for shortcuts'])).toBe('manual')
  })

  it('takes the newest footer, and none when the screen shows no footer', () => {
    expect(readMode(['  ⏸ plan mode on', 'some output', ...box, '  ⏵⏵ accept edits on'])).toBe('accept')
    expect(readMode(['Thinking...', 'Reading 3 files'])).toBe(null)
    expect(readMode([])).toBe(null)
    expect(readMode(null)).toBe(null)
  })

  it('counts the presses along the cycle', () => {
    expect(pressesTo('manual', 'plan')).toBe(2)
    expect(pressesTo('plan', 'manual')).toBe(2)
    expect(pressesTo('auto', 'manual')).toBe(1)
    expect(pressesTo('accept', 'accept')).toBe(0)
    expect(pressesTo(null, 'plan')).toBe(1)
    expect(SHIFT_TAB).toBe('\u001b[Z')
  })
})

describe('the context from the status line', () => {
  it('reads the newest status line', () => {
    expect(readContext([...box, 'Opus 4.1 | 42% | 120.3k tokens'])).toEqual({ percent: 42, tokens: '120.3k' })
    expect(readContext(['Opus | 10% | 9k tokens', ...box, 'Opus | 61% | 1.2M tokens'])).toEqual({ percent: 61, tokens: '1.2M' })
    expect(readContext(['Sonnet | 0% | 0 tokens'])).toEqual({ percent: 0, tokens: '0' })
    expect(readContext(['Claude | --% | -- tokens'])).toBe(null)
    expect(readContext(['Thinking...'])).toBe(null)
    expect(readContext(null)).toBe(null)
  })

  it('stays neutral, then turns warm near the top', () => {
    expect([0, 50, WARM_FROM - 1, WARM_FROM, 100].map(contextTone)).toEqual(['neutral', 'neutral', 'neutral', 'warm', 'warm'])
  })
})
