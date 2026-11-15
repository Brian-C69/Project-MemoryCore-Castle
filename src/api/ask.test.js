import { describe, expect, it, vi, beforeEach } from 'vitest'
vi.mock('./castle.js', () => ({ terminalState: vi.fn(), terminalWrite: vi.fn() }))
vi.mock('../components/typeline.js', () => ({ typeLine: vi.fn((write, text) => write(text)) }))
import { terminalState, terminalWrite } from './castle.js'
import { askCompanion } from './ask.js'

describe('asking the companion from a room', () => {
  beforeEach(() => vi.clearAllMocks())

  it('pastes every line as one request, without empty ones', async () => {
    terminalState.mockResolvedValue({ running: true })
    await askCompanion(['Paint a wallpaper.', '', 'Tell me when you are done.'], 'Alice')
    expect(terminalWrite.mock.calls[0][0]).toBe('\u001b[200~Paint a wallpaper.\nTell me when you are done.\u001b[201~')
  })

  it('asks for the companion to be woken first', async () => {
    terminalState.mockResolvedValue({ running: false })
    await expect(askCompanion(['Paint a wallpaper.'], 'Alice')).rejects.toThrow('Wake Alice first')
    expect(terminalWrite).not.toHaveBeenCalled()
  })
})
