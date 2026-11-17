import { describe, expect, it } from 'vitest'
import { canSubmit, cleanRepoName, githubLine, landingFolder, nameFromLink, OTHER, setupSentence, STACK, stackChoice, stackPreview, validRepoName } from './newproject.js'

describe('the New project helpers', () => {
  it('reads the name from a clone link', () => {
    expect(nameFromLink('https://github.com/owner/my-app.git')).toBe('my-app')
    expect(nameFromLink('https://github.com/owner/my-app')).toBe('my-app')
    expect(nameFromLink('https://github.com/owner/my-app/')).toBe('my-app')
    expect(nameFromLink('  git@github.com:owner/order.tracker.git ')).toBe('order.tracker')
    expect(nameFromLink('https://github.com/owner/repo?tab=readme#top')).toBe('repo')
    expect(nameFromLink('')).toBe('')
    expect(nameFromLink(null)).toBe('')
    expect(nameFromLink('just-words')).toBe('')
  })

  it('knows a valid repo name', () => {
    expect(validRepoName('my-app')).toBe(true)
    expect(validRepoName('My_App.v2')).toBe(true)
    expect(validRepoName('9lives')).toBe(true)
    expect(validRepoName('.hidden')).toBe(false)
    expect(validRepoName('has space')).toBe(false)
    expect(validRepoName('slash/name')).toBe(false)
    expect(validRepoName('ünïcode')).toBe(false)
    expect(validRepoName('')).toBe(false)
    expect(validRepoName(undefined)).toBe(false)
    expect(validRepoName('a'.repeat(100))).toBe(true)
    expect(validRepoName('a'.repeat(101))).toBe(false)
  })

  it('cleans any text into a valid repo name', () => {
    expect(cleanRepoName('Project MemoryCore Castle')).toBe('Project-MemoryCore-Castle')
    expect(cleanRepoName('My App (old)')).toBe('My-App-old')
    expect(cleanRepoName('.dotfiles')).toBe('dotfiles')
    expect(cleanRepoName('--x--')).toBe('x')
    expect(cleanRepoName('???')).toBe('')
    expect(cleanRepoName(null)).toBe('')
    expect(validRepoName(cleanRepoName('Kiyo Software / TechLab'))).toBe(true)
    expect(cleanRepoName('b'.repeat(140))).toHaveLength(100)
  })

  it('picks the GitHub line', () => {
    expect(githubLine(null)).toBe('asking')
    expect(githubLine({ installed: false, signedIn: false, owners: [] })).toBe('install')
    expect(githubLine({ installed: true, signedIn: false, owners: [] })).toBe('signin')
    expect(githubLine({ installed: true, signedIn: true, login: 'me', owners: ['me'] })).toBe('ready')
  })

  it('shows where the project lands', () => {
    expect(landingFolder('D:\\Projects\\', 'System', 'app')).toBe('D:\\Projects\\System\\app')
    expect(landingFolder('D:\\Projects', '', 'app')).toBe('D:\\Projects\\app')
    expect(landingFolder('D:\\Projects', 'Web', ' ')).toBe('D:\\Projects\\Web\\<name>')
    expect(landingFolder(null, 'Web', 'app')).toBe('')
    expect(landingFolder('/Users/lyra/Projects/', 'Web', 'app')).toBe('/Users/lyra/Projects/Web/app')
    expect(landingFolder('/Users/lyra/Projects', '', ' ')).toBe('/Users/lyra/Projects/<name>')
    // the suggested folder before any choice: the sheet lands and submits there straight away
    expect(landingFolder('C:\\Users\\Lyra\\Projects', '', 'order-tracker')).toBe('C:\\Users\\Lyra\\Projects\\order-tracker')
    expect(canSubmit({ mode: 'link', root: 'C:\\Users\\Lyra\\Projects', link: 'https://github.com/o/order-tracker.git', name: 'order-tracker', working: false })).toBe(true)
  })

  it('lets the sheet make the project only when it can', () => {
    const ready = { installed: true, signedIn: true, owners: ['me'] }
    const link = { mode: 'link', root: 'D:\\P', link: 'https://github.com/o/r', name: 'r', working: false }
    expect(canSubmit(link)).toBe(true)
    expect(canSubmit({ ...link, link: '  ' })).toBe(false)
    expect(canSubmit({ ...link, root: null })).toBe(false)
    expect(canSubmit({ ...link, working: true })).toBe(false)
    expect(canSubmit({ ...link, name: '.bad' })).toBe(false)
    expect(canSubmit({ ...link, name: ' r ' })).toBe(true)
    const create = { mode: 'create', root: 'D:\\P', name: 'new-app', owner: 'me', github: ready, working: false }
    expect(canSubmit(create)).toBe(true)
    expect(canSubmit({ ...create, owner: '' })).toBe(false)
    expect(canSubmit({ ...create, github: { ...ready, signedIn: false } })).toBe(false)
    expect(canSubmit({ ...create, github: null })).toBe(false)
    expect(canSubmit({ ...create, mode: 'other' })).toBe(false)
  })
})

describe('the stack a new project may start with', () => {
  it('says nothing when nothing is chosen', () => {
    expect(stackPreview({})).toBe('')
    expect(setupSentence({})).toBe('')
  })
  it('names one choice', () => {
    expect(stackPreview({ frontend: 'Vue.js' })).toBe('Vue.js')
    expect(setupSentence({ frontend: 'Vue.js' })).toBe('Set up a new project: Vue.js frontend. Start with a plan: set up the environment, then scaffold the project. Ask me before anything costs money or goes online.')
  })
  it('names all five in order, hosting last', () => {
    const picks = { mobile: 'Flutter', hosting: 'Vercel', database: 'PostgreSQL', backend: 'NestJS', frontend: 'Next.js' }
    expect(stackPreview(picks)).toBe('Next.js + NestJS + PostgreSQL + Flutter on Vercel')
    expect(setupSentence(picks)).toContain('Next.js frontend, NestJS backend, PostgreSQL database, Flutter mobile app, deployed on Vercel. Start with a plan: set up the environment, scaffold the project, then deployment.')
  })
  it("passes the member's own words for Other through, trimmed and cut", () => {
    expect(stackChoice(OTHER, '  Elixir   Phoenix ')).toBe('Elixir Phoenix')
    expect(stackChoice(OTHER, 'x'.repeat(80)).length).toBe(60)
    expect(stackPreview({ backend: OTHER }, { backend: 'Elixir Phoenix' })).toBe('Elixir Phoenix')
    expect(stackPreview({ backend: OTHER }, { backend: '   ' })).toBe('')
  })
  it('handles hosting alone and mobile alone', () => {
    expect(stackPreview({ hosting: 'Render' })).toBe('hosted on Render')
    expect(setupSentence({ hosting: 'Render' })).toContain('Set up a new project, deployed on Render.')
    expect(setupSentence({ mobile: 'Android (Kotlin)' })).toContain('Android (Kotlin) mobile app.')
  })
  it('keeps one table for every choice', () => {
    expect(STACK.map((s) => s.id)).toEqual(['frontend', 'backend', 'database', 'hosting', 'mobile'])
  })
})

