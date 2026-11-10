import { describe, expect, it, vi, beforeEach } from 'vitest'
vi.mock('./castle.js', () => ({ imageRequest: vi.fn(), terminalState: vi.fn(), terminalWrite: vi.fn() }))
import { imageRequest, terminalWrite } from './castle.js'
import { askImage } from './ask.js'
import { referenceLine } from '../components/appearance.js'
describe('Codex image routing', () => {
  beforeEach(() => vi.clearAllMocks())
  it('passes output, reference and changes without writing to the conversation', async () => {
    await askImage(['Keep the face.', referenceLine('C:\\My Pictures\\Mira.png'), 'Save it as C:\\Memory\\appearance\\portrait.png.', 'Tell me the cost first and ask before you draw.'])
    expect(imageRequest).toHaveBeenCalledWith(expect.stringContaining('Keep the face.'), 'C:\\Memory\\appearance\\portrait.png', ['C:\\My Pictures\\Mira.png'], 'appearance')
    expect(imageRequest.mock.calls[0][0]).not.toContain('Tell me the cost first')
    expect(terminalWrite).not.toHaveBeenCalled()
  })
  it('propagates routing errors rather than falling back to the main chat or API', async () => {
    imageRequest.mockRejectedValueOnce(new Error('Codex unavailable'))
    await expect(askImage(['Draw a lake.'])).rejects.toThrow('Codex unavailable')
    expect(terminalWrite).not.toHaveBeenCalled()
  })
})
