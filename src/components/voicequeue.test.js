import { describe, expect, it } from 'vitest'
import { createVoiceQueue } from './voicequeue.js'

describe('her voice lines, one after another', () => {
  it('a voice that lands while another plays waits, then plays when the first ends', () => {
    const q = createVoiceQueue()
    expect(q.arrive('a.mp3', false)).toBe('a.mp3')
    expect(q.arrive('b.mp3', true)).toBe(null)
    expect(q.arrive('b.mp3', true)).toBe(null)
    expect(q.waiting).toEqual(['b.mp3'])
    expect(q.ended()).toBe('b.mp3')
    expect(q.ended()).toBe(null)
  })

  it('his click plays at once and clears the line', () => {
    const q = createVoiceQueue()
    q.arrive('a.mp3', false)
    q.arrive('b.mp3', true)
    expect(q.choose('old.mp3')).toBe('old.mp3')
    expect(q.waiting).toEqual([])
    q.stop()
    expect(q.ended()).toBe(null)
  })
})
