import { describe, expect, it } from 'vitest'
import { finishedIds, requestLine } from './imagejobs.js'

const job = (id, phase) => ({ id, phase, label: id, message: '' })

describe("a room's one request line", () => {
  it('shows the newest request still drawing before any failure', () => {
    expect(requestLine([job('a', 'prompt-saved'), job('b', 'working'), job('c', 'prompt-saved')]).id).toBe('b')
  })
  it('shows the newest failure when nothing is drawing', () => {
    expect(requestLine([job('a', 'prompt-saved'), job('b', 'completed'), job('c', 'prompt-saved')]).id).toBe('c')
  })
  it('shows nothing when every request is done', () => {
    expect(requestLine([job('a', 'completed')])).toBeNull()
    expect(requestLine()).toBeNull()
  })
  it('names the finished ones to clear', () => {
    expect(finishedIds([job('a', 'completed'), job('b', 'working'), job('c', 'completed')])).toEqual(['a', 'c'])
  })
})
