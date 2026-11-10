import { describe, expect, it } from 'vitest'
import { card, megabytes, moveLine, nextStep, progressLine, steps } from './database.js'

const none = { state: 'notInstalled', external: null }
const stopped = { state: 'stopped', port: 3307 }
const running = { state: 'running', port: 3307 }
const xampp = (answering) => ({ state: 'external', port: 3306, answering })
const empty = { name: null, reachable: false, movedIn: false, canRestore: false }
const made = { name: 'lyra_memorycoredb', reachable: true, movedIn: false, canRestore: false }
const home = { ...made, movedIn: true }

describe('the database steps', () => {
  it('walks install, start, create, move in order', () => {
    expect(nextStep(none, empty)).toBe('install')
    expect(nextStep(stopped, empty)).toBe('start')
    expect(nextStep(running, empty)).toBe('create')
    expect(nextStep(running, made)).toBe('move')
    expect(nextStep(running, home)).toBe(null)
  })

  it('never counts a database as created or moved in while the engine is stopped', () => {
    // the .env still names it, but nothing answers
    expect(nextStep(stopped, { ...home, reachable: false })).toBe('start')
    expect(steps(stopped, home).map((s) => s.ok)).toEqual([true, false, false, false])
  })

  it('treats an existing server as installed, and running only while it answers', () => {
    expect(steps(xampp(true), made)[0].label).toBe('Using the database already on this PC')
    expect(nextStep(xampp(false), made)).toBe('start')
    expect(nextStep(xampp(true), made)).toBe('move')
  })

  it('names the database once it exists', () => {
    expect(steps(running, made)[2].label).toBe('lyra_memorycoredb created')
    expect(steps(running, empty)[2].label).toBe('Your memory database created')
  })

  it('still answers before the core has', () => {
    expect(nextStep(null, null)).toBe('install')
  })
})

describe('the words', () => {
  it('rounds the download size and the progress', () => {
    expect(megabytes(95024126)).toBe('91 MB')
    expect(progressLine(0, 0)).toBe('Downloading…')
    expect(progressLine(41, 95)).toBe('Downloading… 43%')
    expect(progressLine(200, 95)).toBe('Downloading… 100%')
  })

  it('says what will move, in plain counts', () => {
    expect(moveLine({ files: 14, diary: 6, memory: 4, reminders: 3, projects: 3 })).toBe('14 files will move: 6 diary pages, 4 memory files, 3 reminders, 3 projects.')
    expect(moveLine({ files: 1, diary: 1, memory: 0, reminders: 0, projects: 0 })).toBe('1 file will move: 1 diary page.')
    expect(moveLine({ files: 0, diary: 0, memory: 0, reminders: 0, projects: 0 })).toMatch(/nothing to move/)
    expect(moveLine(null)).toBe('')
  })
})

describe('the HOME card', () => {
  it('offers the one thing to do next', () => {
    expect(card(null, null)).toEqual({ line: 'Checking…', button: null })
    expect(card(none, empty).button).toEqual({ label: 'Set up', action: 'open' })
    expect(card(stopped, made)).toEqual({ line: 'Stopped.', button: { label: 'Start', action: 'start' } })
    expect(card(running, made).line).toBe('Running · your memory is still in files.')
    expect(card(running, home)).toEqual({ line: 'lyra_memorycoredb · running', button: null })
  })

  it('never offers to start a server Castle does not own', () => {
    expect(card(xampp(false), made).button.action).toBe('open')
  })
})
