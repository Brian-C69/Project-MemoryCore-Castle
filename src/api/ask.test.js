import { describe, expect, it, vi, beforeEach } from 'vitest'
vi.mock('./castle.js', () => ({ imageRequest: vi.fn(), pictureWay: vi.fn(), terminalState: vi.fn(), terminalWrite: vi.fn() }))
vi.mock('../components/typeline.js', () => ({ typeLine: vi.fn((write, text) => write(text)) }))
import { imageRequest, pictureWay, terminalState, terminalWrite } from './castle.js'
import { askImage } from './ask.js'
import { referenceLine } from '../components/appearance.js'

describe('picture routing', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    pictureWay.mockResolvedValue('codex')
  })

  it('on the ChatGPT plan passes output, reference and changes to Codex without writing to the conversation', async () => {
    await expect(askImage(['Keep the face.', referenceLine('C:\\My Pictures\\Companion.png'), 'Save it as C:\\Memory\\appearance\\portrait.png.', 'Tell me the cost first and ask before you draw.'])).resolves.toBe('codex')
    expect(imageRequest).toHaveBeenCalledWith(expect.stringContaining('Keep the face.'), 'C:\\Memory\\appearance\\portrait.png', ['C:\\My Pictures\\Companion.png'], 'appearance', null)
    expect(imageRequest.mock.calls[0][0]).not.toContain('Tell me the cost first')
    expect(terminalWrite).not.toHaveBeenCalled()
  })

  it('tells the queue what the picture is for', async () => {
    await askImage(['Draw a coat.', 'Save it as C:\\Memory\\appearance\\wardrobe\\coat\\turnaround.png.'], 'Outfit turnaround: Coat')
    expect(imageRequest.mock.calls[0][4]).toBe('Outfit turnaround: Coat')
  })

  it('propagates queue errors', async () => {
    imageRequest.mockRejectedValueOnce(new Error('Codex unavailable'))
    await expect(askImage(['Draw a lake.'])).rejects.toThrow('Codex unavailable')
    expect(terminalWrite).not.toHaveBeenCalled()
  })

  it('with an OpenAI key and Claude Code asks the companion, keeping the cost question', async () => {
    pictureWay.mockResolvedValue('companion')
    terminalState.mockResolvedValue({ running: true })
    await expect(askImage(['Draw a lake.', '', 'Save it as C:\\Memory\\appearance\\portrait.png.', 'Tell me the cost first and ask before you draw.'], 'Standard look', 'Alice')).resolves.toBe('companion')
    expect(imageRequest).not.toHaveBeenCalled()
    const typed = terminalWrite.mock.calls[0][0]
    expect(typed).toContain('Draw a lake.\nSave it as C:\\Memory\\appearance\\portrait.png.\nTell me the cost first')
  })

  it('asks for the companion to be woken before it is asked to draw', async () => {
    pictureWay.mockResolvedValue('companion')
    terminalState.mockResolvedValue({ running: false })
    await expect(askImage(['Draw a lake.'], null, 'Alice')).rejects.toThrow('Wake Alice first')
    expect(imageRequest).not.toHaveBeenCalled()
  })

  it('with no way to draw still saves the prompt through the queue', async () => {
    pictureWay.mockResolvedValue('none')
    await expect(askImage(['Draw a lake.'])).resolves.toBe('none')
    expect(imageRequest).toHaveBeenCalled()
    expect(terminalWrite).not.toHaveBeenCalled()
  })
})
