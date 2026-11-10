import { describe, expect, it } from 'vitest'
import { ENTER, ENTER_DELAY_MS, typeLine } from './typeline.js'

describe('typing a line into the session', () => {
  it('types the text, pauses, then presses Enter - in that order', async () => {
    const steps = []
    await typeLine(
      async (data) => steps.push(['write', data]),
      'save diary',
      async (ms) => steps.push(['wait', ms]),
    )
    expect(steps).toEqual([['write', 'save diary'], ['wait', ENTER_DELAY_MS], ['write', ENTER]])
  })

  it('pauses long enough for Codex, which ignores an Enter within about 50 ms of the text', () => {
    expect(ENTER_DELAY_MS).toBeGreaterThanOrEqual(100)
    expect(ENTER).toBe('\r')
  })

  it('does not press Enter when the text could not be written', async () => {
    const written = []
    const write = async (data) => {
      if (data === 'save diary') throw new Error('no session')
      written.push(data)
    }
    await expect(typeLine(write, 'save diary', async () => {})).rejects.toThrow('no session')
    expect(written).toEqual([])
  })
})
