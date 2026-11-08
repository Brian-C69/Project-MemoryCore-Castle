import { describe, expect, it } from 'vitest'
import { readChoices } from './choices.js'

describe('readChoices', () => {
  it('reads a permission prompt inside a box', () => {
    const screen = [
      '╭────────────────────────────────────────────╮',
      '│ Bash command                               │',
      '│   cargo test                               │',
      '│ Do you want to proceed?                    │',
      '│ ❯ 1. Yes                                   │',
      "│   2. Yes, and don't ask again for cargo    │",
      '│   3. No, and tell Claude what to do (esc)  │',
      '╰────────────────────────────────────────────╯',
      '',
    ]
    expect(readChoices(screen)).toEqual({
      question: 'Do you want to proceed?',
      options: [
        { number: 1, label: 'Yes' },
        { number: 2, label: "Yes, and don't ask again for cargo" },
        { number: 3, label: 'No, and tell Claude what to do' },
      ],
    })
  })

  it('reads a question menu without a box, the marker on any line', () => {
    const screen = ['Which layout should the first theme use?', '  1. The Watch', '❯ 2. Beside Her', '  3. The Night Sky']
    const menu = readChoices(screen)
    expect(menu.question).toBe('Which layout should the first theme use?')
    expect(menu.options.map((o) => o.number)).toEqual([1, 2, 3])
  })

  it('ignores her ordinary numbered list, which has no marker', () => {
    const screen = ['Three things wait:', '1. The installer', '2. The walk', '3. The Pair screen', '', '> ']
    expect(readChoices(screen)).toBeNull()
  })

  it('ignores a menu that has scrolled far above the bottom', () => {
    const screen = ['Do you want to proceed?', '❯ 1. Yes', '  2. No', ...Array.from({ length: 12 }, (_, i) => `line ${i}`)]
    expect(readChoices(screen)).toBeNull()
  })

  it('needs options counted from one', () => {
    expect(readChoices(['Pick?', '❯ 2. Yes', '  3. No'])).toBeNull()
  })

  it('reads a question form with descriptions under the options and a divider (Nov 7 11:31 PM)', () => {
    const screen = [
      '  ← ☒ GitHub  ☐ Crypto AI  ✓ Submit  →',
      '',
      '│ "Finish the system" for Crypto AI Trading: nothing of that name exists yet',
      '│ anywhere on this PC (the Crypto Trading Management System is a different,',
      '│ finished project). To build it I need its shape. Which is it?',
      '',
      '  1. Plan it with me now',
      '     I open plan mode: scope, stack (FastAPI + Vue 3 like the Crypto TMS, or',
      '     Python bot only), exchange and paper-first rule.',
      '  2. You have a brief somewhere',
      '     Tell me where (a chat, a file, a terminal) and I read it first, then plan',
      '     from it.',
      '  3. Scaffold + plan only tonight',
      '     I write the plan and the project doc now; the build waits for the next',
      '     session.',
      '❯ 4. just put in the weekly board',
      '────────────────────────────────────────────────────────────',
      '  5. Chat about this',
      '',
      '  Enter to select · Tab/Arrow keys to navigate · ctrl+g to edit in Notepad · Esc',
      '  to cancel',
      '──────────────────────────────────────── fantasy-gate-visual-novel ─',
    ]
    const menu = readChoices(screen)
    expect(menu.question).toBe('Which is it?')
    expect(menu.options.map((o) => o.label)).toEqual([
      'Plan it with me now',
      'You have a brief somewhere',
      'Scaffold + plan only tonight',
      'just put in the weekly board',
      'Chat about this',
    ])
    expect(menu.options[0].hint).toMatch(/^I open plan mode/)
    expect(menu.options[4].hint).toBeUndefined()
  })

  it('reads nothing from an empty screen', () => {
    expect(readChoices([])).toBeNull()
    expect(readChoices(['', '   '])).toBeNull()
  })
})
