import { describe, expect, it } from 'vitest'
import { edgeEnabled, labelOf, nextEnabled, normalise } from './glassselect.js'

describe('the glass dropdown', () => {
  it('takes objects or plain strings', () => {
    expect(normalise(['24h', { value: '12h', label: '12-hour', disabled: true }])).toEqual([
      { value: '24h', label: '24h', disabled: false },
      { value: '12h', label: '12-hour', disabled: true },
    ])
    expect(normalise([{ value: 3 }])).toEqual([{ value: 3, label: '3', disabled: false }])
  })
  it('shows the label of the value, or the placeholder', () => {
    const o = normalise([{ value: 'low', label: 'Low' }])
    expect(labelOf(o, 'low')).toBe('Low')
    expect(labelOf(o, 'high', 'Choose...')).toBe('Choose...')
  })
  it('steps over disabled options and wraps', () => {
    const o = normalise(['a', { value: 'b', disabled: true }, 'c'])
    expect(nextEnabled(o, 0, 1)).toBe(2)
    expect(nextEnabled(o, 2, 1)).toBe(0)
    expect(nextEnabled(o, 0, -1)).toBe(2)
    expect(edgeEnabled(o)).toBe(0)
    expect(edgeEnabled(o, true)).toBe(2)
  })
  it('finds nothing when every option is disabled', () => {
    expect(nextEnabled(normalise([{ value: 'x', disabled: true }]), 0, 1)).toBe(-1)
    expect(nextEnabled([], 0, 1)).toBe(-1)
  })
})
