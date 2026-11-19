import { describe, expect, it } from 'vitest'
import { CLEAR_LINE, HOLD_EVERY_MS, readPrompt, voiceHold, voiceProblem, voiceWorking } from './voicehold.js'

const rule = '─'.repeat(60)
// Claude Code 2.1's prompt as the Log shows it (copied from a live 80-column screen)
const promptOf = (...lines) => ['', '  ◐ medium · /effort', rule, ...lines, rule, '  ⏵⏵ auto mode on (shift+tab to cycle)']

describe("reading Claude Code's prompt off the Log screen", () => {
  it('joins the screen lines of a long prompt into one text', () => {
    const screen = promptOf(
      '❯ alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi',
      '  omicron pi rho sigma tau upsilon phi chi psi omega',
    )
    expect(readPrompt(screen)).toEqual({
      text: 'alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega',
      lines: 2,
    })
  })

  it('reads an empty prompt (its dim placeholder already left out) as no words', () => {
    expect(readPrompt(promptOf('❯ '))).toEqual({ text: '', lines: 1 })
  })

  it("takes the prompt box, never the member's earlier message in the conversation", () => {
    const screen = ['❯ what I said before', '', '● The answer', ...promptOf('❯ hello there')]
    expect(readPrompt(screen).text).toBe('hello there')
  })

  it("reads the older rounded box too", () => {
    const screen = ['╭──────────────────────╮', '│ > hello from the box │', '╰──────────────────────╯']
    expect(readPrompt(screen)).toEqual({ text: 'hello from the box', lines: 1 })
  })

  it('finds no prompt while a question is up or the screen is empty', () => {
    expect(readPrompt(['Do you want to proceed?', '❯ 1. Yes', '  2. No'])).toBe(null)
    expect(readPrompt([rule, '❯ cut off at the bottom'])).toBe(null)
    expect(readPrompt([])).toBe(null)
    expect(readPrompt(null)).toBe(null)
  })

  it("tells when Claude Code is still at work, and what went wrong", () => {
    expect(voiceWorking([...promptOf('❯ '), '  listening…'])).toBe(true)
    expect(voiceWorking([...promptOf('❯ '), '  · keep holding…'])).toBe(true)
    expect(voiceWorking([...promptOf('❯ '), 'Transcribing…'])).toBe(true)
    expect(voiceWorking(promptOf('❯ hello'))).toBe(false)
    expect(voiceProblem([...promptOf('❯ '), '  No speech detected.'])).toBe('No speech detected.')
    expect(voiceProblem(promptOf('❯ '))).toBe('')
  })
})

// A pretend session: what was typed, and the screens Claude Code shows one read after another.
function fakeSession(screens) {
  const typed = []
  let read = 0
  let tick = null
  const next = () => screens[Math.min(read++, screens.length - 1)]
  let cleared = false
  const mic = voiceHold({
    write: async (data) => {
      typed.push(data)
      if (data.includes(CLEAR_LINE)) cleared = true
    },
    readScreen: () => (cleared ? promptOf('❯ ') : next()),
    readTyped: () => (cleared ? promptOf('❯ ') : screens[Math.max(0, Math.min(read - 1, screens.length - 1))]),
    pause: async () => {},
    every: (fn, ms) => {
      tick = { fn, ms }
      return 1
    },
    stopEvery: () => {
      tick = null
    },
  })
  return { mic, typed, tick: () => tick }
}

describe('holding the mic', () => {
  it('types Spaces at a held key’s pace until release', async () => {
    const { mic, typed, tick } = fakeSession([promptOf('❯ ')])
    expect(mic.start()).toBe(true)
    expect(mic.start()).toBe(false)
    expect(mic.held()).toBe(true)
    expect(tick().ms).toBe(HOLD_EVERY_MS)
    expect(HOLD_EVERY_MS).toBeLessThan(120)
    tick().fn()
    tick().fn()
    expect(typed).toEqual([' ', ' ', ' '])
    await mic.stop()
    expect(tick()).toBe(null)
    expect(mic.held()).toBe(false)
  })

  it('waits for the words, brings them to Chat and empties the prompt', async () => {
    const words = promptOf('❯ please book the table for', '  seven tonight')
    const { mic, typed } = fakeSession([
      [...promptOf('❯ please book'), '  listening…'],
      [...promptOf('❯ '), 'Transcribing…'],
      words,
      words,
    ])
    mic.start()
    const out = await mic.stop()
    expect(out).toEqual({ text: 'please book the table for seven tonight', note: '' })
    // one Ctrl+U per screen line of the prompt, and one more
    expect(typed.at(-1)).toBe(CLEAR_LINE.repeat(3))
  })

  it('brings nothing and says so when Claude Code heard no speech', async () => {
    const { mic, typed } = fakeSession([[...promptOf('❯ '), '  No speech detected.']])
    mic.start()
    expect(await mic.stop()).toEqual({ text: '', note: 'No speech detected.' })
    // the Spaces a short hold may leave in the prompt are taken away
    expect(typed.at(-1)).toBe(CLEAR_LINE)
  })

  it('says where the words are when the prompt cannot be read', async () => {
    const { mic } = fakeSession([['Do you want to proceed?', '❯ 1. Yes']])
    mic.start()
    const out = await mic.stop()
    expect(out.text).toBe('')
    expect(out.note).toMatch(/open Log/)
  })

  it('does nothing on a release without a hold', async () => {
    const { mic, typed } = fakeSession([promptOf('❯ ')])
    expect(await mic.stop()).toEqual({ text: '', note: '' })
    expect(typed).toEqual([])
  })
})
