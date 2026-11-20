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
      context: 'Bash command · cargo test',
      options: [
        { number: 1, label: 'Yes' },
        { number: 2, label: "Yes, and don't ask again for cargo" },
        { number: 3, label: 'No, and tell Claude what to do', tell: true },
      ],
    })
  })

  it("marks the plan card's free-text option, and only that one", () => {
    const screen = ['Would you like to proceed?', '❯ 1. Yes, and use auto mode', '  2. Yes, manually approve edits', '  3. Tell Claude what to change']
    const menu = readChoices(screen)
    expect(menu.options.map((o) => !!o.tell)).toEqual([false, false, true])
    const form = readChoices(['Which one?', '❯ 1. Plan it with me now', '  2. Later', '  3. Type something.'])
    expect(form.options.map((o) => !!o.tell)).toEqual([false, false, true])
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

  it('keeps a question whose paragraph ends in a parenthesis, its context, the descriptions and the tabs (Nov 11 7:40 AM)', () => {
    const screen = [
      '❯ btw i already start mysql',
      '────────────────────────────────────────────────────────────────────────────',
      'Planning: C:\\Users\\User\\.claude\\plans\\buzzing-tumbling-newt.md',
      '────────────────────────────────────────────────────────────────────────────',
      '←  □ Diary backup  □ Old lead text  ✓ Submit  →',
      '',
      "Once the diary's text is in the database and the markdown is deleted, where",
      "should its safety copy live? (Last night you chose 'production is the backup,",
      "no dump'. Since then the push to the Portfolio stopped twice at 2026-11-05",
      "with an error, and today's second entry is still not there.)",
      '',
      '❯ 1. Day files by the door (Recommended)',
      '     The diary door rewrites one plain text file per day at every save and git',
      '     commits it, exactly like my memory\'s shadow files.',
      '  2. Production only',
      '     Your decision from last night: the Portfolio is the backup, nothing kept',
      '     locally besides the rows.',
      '  3. One SQL dump in git',
      '     Diary rows join the committed database dump as one insert line per entry.',
      '  4. Type something.',
      '────────────────────────────────────────────────────────────────────────────',
      '  5. Chat about this',
      '',
      'Enter to select · Tab/Arrow keys to navigate · Esc to cancel',
    ]
    const menu = readChoices(screen)
    expect(menu.question).toBe("Once the diary's text is in the database and the markdown is deleted, where should its safety copy live?")
    expect(menu.context).toMatch(/^\(Last night you chose 'production is the backup, no dump'\. .* still not there\.\)$/)
    expect(menu.tabs).toEqual([
      { name: 'Diary backup', done: false },
      { name: 'Old lead text', done: false },
    ])
    expect(menu.options.map((o) => o.number)).toEqual([1, 2, 3, 4, 5])
    expect(menu.options[0].hint).toMatch(/^The diary door rewrites one plain text file/)
    expect(menu.options[3].tell).toBe(true)
  })

  it('marks an answered tab', () => {
    const menu = readChoices(['← ☒ GitHub  ☐ Crypto AI  ✓ Submit →', '', 'Which one?', '❯ 1. Yes', '  2. No'])
    expect(menu.tabs).toEqual([
      { name: 'GitHub', done: true },
      { name: 'Crypto AI', done: false },
    ])
    expect(menu.question).toBe('Which one?')
  })

  it('gives the whole paragraph as context when it asks no question', () => {
    const menu = readChoices(['Pick the layout for the first theme', '❯ 1. The Watch', '  2. Beside Her'])
    expect(menu.question).toBe('')
    expect(menu.context).toBe('Pick the layout for the first theme')
  })

  it('reads nothing from an empty screen', () => {
    expect(readChoices([])).toBeNull()
    expect(readChoices(['', '   '])).toBeNull()
  })

  it('reads the folder-trust question, whose options carry no numbers', () => {
    const screen = [
      '────────────────────────────────────────',
      'Accessing workspace:',
      '',
      'D:\\Work\\Project-Bus-Management-System',
      '',
      "Quick safety check: Is this a project you created or one you trust? (Like your own code, a well-known open source project, or work from your team). If not, take a moment to review what's in this folder first.",
      '',
      "Claude Code'll be able to read, edit, and execute files here.",
      '',
      'Security guide',
      '',
      '  No, exit',
      '❯ Yes, I trust this folder',
      '',
      'Enter to confirm · Esc to cancel',
    ]
    const menu = readChoices(screen)
    expect(menu.question).toBe('Quick safety check: Is this a project you created or one you trust?')
    expect(menu.context).toBe('D:\\Work\\Project-Bus-Management-System')
    expect(menu.options).toEqual([
      { number: 1, label: 'No, exit', keys: '\x1b[A\r' },
      { number: 2, label: 'Yes, I trust this folder', keys: '\r' },
    ])
    // a plain unnumbered list is still no card
    expect(readChoices(['Yes, I think so', 'No, not today'])).toBeNull()
  })

  it('gives a question of several answers its Submit row: the next tab in a form (Nov 19 2026)', () => {
    const screen = [
      '←  ☒ Who joins  ☒ Join does  ☒ Lyra tells  ✔ Submit  →',
      '',
      'Which notifications should Lyra give you for a class?',
      '',
      '❯ 1. [✓] Someone joined (Recommended)',
      "     A phone push in Lyra's voice the moment someone takes a seat.",
      '  2. [✓] Before class starts',
      '     The normal calendar reminders (15/30 min before) for a Class.',
      '  3. [✓] Someone left',
      '     A push when someone gives back their seat.',
      '  4. [ ] Type something',
      '     Submit',
      '────────────────────────────────────────',
      '  5. Chat about this',
      '',
      'Enter to select · Tab/Arrow keys to navigate · Esc to cancel',
    ]
    const menu = readChoices(screen)
    expect(menu.question).toBe('Which notifications should Lyra give you for a class?')
    expect(menu.tabs.map((t) => t.name)).toEqual(['Who joins', 'Join does', 'Lyra tells'])
    expect(menu.options.map((o) => o.number)).toEqual([1, 2, 3, 4, 5])
    expect(menu.options[3].hint).toBeUndefined()
    expect(menu.submit).toEqual({ label: 'Next →', keys: '\x1b[C' })
  })

  it('walks a lone question of several answers down to its Submit row and presses it', () => {
    const screen = [
      'Which ones?',
      '',
      '  1. [ ] Red',
      '❯ 2. [✓] Blue',
      '  3. [ ] Type something',
      '     Submit',
      '────────',
      '  4. Chat about this',
    ]
    const menu = readChoices(screen)
    expect(menu.tabs).toBeUndefined()
    expect(menu.submit).toEqual({ label: 'Submit', keys: '\x1b[B\x1b[B\r' })
    expect(readChoices(['Pick?', '❯ 1. Yes', '  2. No']).submit).toBeUndefined()
  })
})

