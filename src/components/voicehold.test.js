import { describe, expect, it } from 'vitest'
import { CLEAR_LINE, HOLD_EVERY_MS, isMicShortcut, MAX_RECORDING_MS, MIC_SHORTCUT, readPrompt, voiceMic, voiceProblem, voiceWorking } from './voicehold.js'

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
  let capTimer = null
  const next = () => screens[Math.min(read++, screens.length - 1)]
  let cleared = false
  const mic = voiceMic({
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
    after: (fn, ms) => {
      capTimer = { fn, ms }
      return 2
    },
    stopAfter: () => {
      capTimer = null
    },
  })
  return { mic, typed, tick: () => tick, cap: () => capTimer }
}

const words = promptOf('❯ please book the table for', '  seven tonight')
const speaking = () => [[...promptOf('❯ please book'), '  listening…'], [...promptOf('❯ '), 'Transcribing…'], words, words]

describe('the mic, a click on and a click off', () => {
  it('types Spaces at a held key’s pace from the first click until the second', async () => {
    const { mic, typed, tick } = fakeSession([promptOf('❯ ')])
    expect(mic.on()).toBe(false)
    expect(mic.start()).toBe(true)
    // a second start while on changes nothing (the caller's second click is a stop)
    expect(mic.start()).toBe(false)
    expect(mic.on()).toBe(true)
    expect(tick().ms).toBe(HOLD_EVERY_MS)
    expect(HOLD_EVERY_MS).toBeLessThan(120)
    tick().fn()
    tick().fn()
    expect(typed).toEqual([' ', ' ', ' '])
    await mic.stop()
    expect(tick()).toBe(null)
    expect(mic.on()).toBe(false)
  })

  it('waits for the words, brings them to Chat and empties the prompt', async () => {
    const { mic, typed } = fakeSession(speaking())
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
    // the Spaces a very short recording may leave in the prompt are taken away
    expect(typed.at(-1)).toBe(CLEAR_LINE)
  })

  it('says where the words are when the prompt cannot be read', async () => {
    const { mic } = fakeSession([['Do you want to proceed?', '❯ 1. Yes']])
    mic.start()
    const out = await mic.stop()
    expect(out.text).toBe('')
    expect(out.note).toMatch(/open Log/)
  })

  it('does nothing on a stop or a cancel while off', async () => {
    const { mic, typed } = fakeSession([promptOf('❯ ')])
    expect(await mic.stop()).toEqual({ text: '', note: '' })
    expect(await mic.cancel()).toEqual({ text: '', note: '' })
    expect(typed).toEqual([])
  })
})

describe('Esc cancels', () => {
  it('waits for Claude Code, empties its prompt and brings no words', async () => {
    const { mic, typed, tick, cap } = fakeSession(speaking())
    mic.start()
    const out = await mic.cancel()
    expect(out.text).toBe('')
    expect(out.note).toBe('Cancelled.')
    expect(typed.at(-1)).toBe(CLEAR_LINE.repeat(3))
    expect(tick()).toBe(null)
    expect(cap()).toBe(null)
    expect(mic.on()).toBe(false)
  })

  it('still empties the Spaces when nothing was heard', async () => {
    const { mic, typed } = fakeSession([promptOf('❯ ')])
    mic.start()
    expect((await mic.cancel()).text).toBe('')
    expect(typed.at(-1)).toBe(CLEAR_LINE)
  })
})

describe('the 2-minute cap', () => {
  it('asks the caller to stop as usual after two minutes', async () => {
    const { mic, cap } = fakeSession(speaking())
    let capped = 0
    mic.start({ onCap: () => (capped += 1) })
    expect(cap().ms).toBe(MAX_RECORDING_MS)
    expect(MAX_RECORDING_MS).toBe(120000)
    cap().fn()
    expect(capped).toBe(1)
    // the stop that follows brings the words like a click would
    expect((await mic.stop()).text).toBe('please book the table for seven tonight')
  })

  it('is cleared by a stop, and a late cap does nothing once off', async () => {
    const { mic, cap } = fakeSession([promptOf('❯ ')])
    let capped = 0
    mic.start({ onCap: () => (capped += 1) })
    const late = cap().fn
    await mic.stop()
    expect(cap()).toBe(null)
    late()
    expect(capped).toBe(0)
  })
})

describe('the shortcut', () => {
  const key = (over) => ({ key: 'M', code: 'KeyM', shiftKey: true, metaKey: false, ctrlKey: false, altKey: false, ...over })

  it('is ⌘⇧M on a Mac and Ctrl+Shift+M elsewhere', () => {
    expect(isMicShortcut(key({ metaKey: true }), true)).toBe(true)
    expect(isMicShortcut(key({ ctrlKey: true }), false)).toBe(true)
    // a layout that gives another letter on that key still counts by its code, and "m" by its letter
    expect(isMicShortcut(key({ metaKey: true, key: 'µ' }), true)).toBe(true)
    expect(isMicShortcut(key({ ctrlKey: true, code: 'Semicolon', key: 'm' }), false)).toBe(true)
    expect(MIC_SHORTCUT).toBe('Ctrl+Shift+M')
  })

  it('leaves every other key alone: Cut, the other platform’s keys, M without Shift', () => {
    expect(isMicShortcut(key({ metaKey: true, key: 'x', code: 'KeyX', shiftKey: false }), true)).toBe(false)
    expect(isMicShortcut(key({ metaKey: true, key: 'X', code: 'KeyX' }), true)).toBe(false)
    expect(isMicShortcut(key({ ctrlKey: true }), true)).toBe(false)
    expect(isMicShortcut(key({ metaKey: true }), false)).toBe(false)
    expect(isMicShortcut(key({ metaKey: true, shiftKey: false, key: 'm' }), true)).toBe(false)
    expect(isMicShortcut(key({ metaKey: true, altKey: true }), true)).toBe(false)
    expect(isMicShortcut(key({ metaKey: true, ctrlKey: true }), true)).toBe(false)
    expect(isMicShortcut(key({}), true)).toBe(false)
    expect(isMicShortcut(null, true)).toBe(false)
  })
})
