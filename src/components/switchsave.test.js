import { describe, expect, it } from 'vitest'
import { canSaveNow, needsSavePrompt, saveChoices } from './switchsave.js'

describe('save before switching', () => {
  it('offers the save buttons the memory folder supports', () => {
    expect(saveChoices({}).map((c) => c.command)).toEqual(['save diary'])
    expect(saveChoices({ projects: true, saveMemory: true }).map((c) => c.command)).toEqual(['save diary', 'save project', 'save memory'])
    expect(saveChoices().map((c) => c.label)).toEqual(['Save diary'])
  })

  it('asks only when a session is running and the provider really changes', () => {
    expect(needsSavePrompt({ running: true, target: 'codex', current: 'claude' })).toBe(true)
    expect(needsSavePrompt({ running: false, target: 'codex', current: 'claude' })).toBe(false)
    expect(needsSavePrompt({ running: true, target: 'claude', current: 'claude' })).toBe(false)
    expect(needsSavePrompt({ running: true, target: '', current: 'claude' })).toBe(false)
  })

  it('a queued save may proceed only while the session is still running', () => {
    expect(canSaveNow({ running: true })).toBe(true)
    expect(canSaveNow({ running: false })).toBe(false)
  })
})
