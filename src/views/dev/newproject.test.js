import { describe, expect, it } from 'vitest'
import { canSubmit, cleanRepoName, githubLine, landingFolder, nameFromLink, validRepoName } from './newproject.js'

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
