import { describe, expect, it } from 'vitest'
import { ASK_TO_COMMIT, ago, badges, canPush, codeOf, commitPaths, diffLines, nextTicks, pushQuestion, withNewestPage } from './git.js'

describe('the Source panel helpers', () => {
  it('reads a file letter', () => {
    expect(codeOf(' M')).toMatchObject({ letter: 'M', word: 'modified', tone: 'changed' })
    expect(codeOf('A ')).toMatchObject({ letter: 'A', tone: 'new' })
    expect(codeOf('??')).toMatchObject({ letter: '??', word: 'new', tone: 'new' })
    expect(codeOf('R ')).toMatchObject({ letter: 'R', word: 'renamed', tone: 'moved' })
    expect(codeOf('UU')).toMatchObject({ letter: 'U', word: 'conflict', tone: 'conflict' })
    expect(codeOf('AA')).toMatchObject({ letter: 'U' })
    expect(codeOf(' D')).toMatchObject({ letter: 'D', tone: 'gone' })
    expect(codeOf('MM')).toMatchObject({ letter: 'M' })
    expect(codeOf(undefined)).toMatchObject({ letter: 'M', tone: 'changed' })
  })

  it('says how long ago', () => {
    const now = Date.parse('2026-11-13T16:30:00+08:00')
    expect(ago('2026-11-13T16:29:50+08:00', now)).toBe('just now')
    expect(ago('2026-11-13T16:28:00+08:00', now)).toBe('2 minutes ago')
    expect(ago('2026-11-13T16:29:00+08:00', now)).toBe('1 minute ago')
    expect(ago('2026-11-13T15:30:00+08:00', now)).toBe('1 hour ago')
    expect(ago('2026-11-12T10:00:00+08:00', now)).toBe('yesterday')
    expect(ago('2026-11-08T10:00:00+08:00', now)).toBe('5 days ago')
    expect(ago('2025-01-02T10:00:00+08:00', now)).toBe('2 Jan 2025')
    expect(ago('not a date', now)).toBe('')
  })

  it('orders badges branch, remote, tag and drops HEAD', () => {
    expect(badges(['tag: v1.0', 'origin/main', 'main', 'origin/HEAD', 'HEAD']).map((b) => [b.text, b.kind])).toEqual([
      ['main', 'branch'],
      ['origin/main', 'remote'],
      ['v1.0', 'tag'],
    ])
    expect(badges(undefined)).toEqual([])
  })

  it('colours a diff line by line', () => {
    const lines = diffLines('diff --git a/x b/x\r\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n same')
    expect(lines.map((l) => l.kind)).toEqual(['meta', 'meta', 'meta', 'hunk', 'del', 'add', 'ctx'])
    expect(lines[0].text).toBe('diff --git a/x b/x')
    expect(diffLines(null)).toEqual([{ kind: 'ctx', text: '' }])
  })

  it('keeps a rename whole in a commit', () => {
    const files = [{ path: 'b.js', from: 'a.js' }, { path: 'c.js' }, { path: 'd.js' }]
    expect(commitPaths(files, new Set(['b.js', 'd.js']))).toEqual(['b.js', 'a.js', 'd.js'])
    expect(commitPaths(files, new Set())).toEqual([])
  })

  it('ticks new files, forgets gone ones and keeps an untick', () => {
    const before = [{ path: 'a.js' }, { path: 'b.js' }, { path: 'gone.js' }]
    const ticked = new Set(['a.js', 'gone.js']) // b.js was unticked by the member
    const fresh = [{ path: 'a.js' }, { path: 'b.js' }, { path: 'new.js' }]
    expect([...nextTicks(before, ticked, fresh)].sort()).toEqual(['a.js', 'new.js'])
    // the first read ticks everything
    expect([...nextTicks([], new Set(), fresh)].sort()).toEqual(['a.js', 'b.js', 'new.js'])
  })

  it('asks one clear push question and knows when there is nothing to push', () => {
    expect(pushQuestion({ branch: 'main', upstream: 'origin/main', ahead: 2 })).toBe('Push 2 commits to origin/main?')
    expect(pushQuestion({ branch: 'main', upstream: 'origin/main', ahead: 1 })).toBe('Push 1 commit to origin/main?')
    expect(pushQuestion({ branch: 'dev', upstream: null, ahead: 0 })).toBe('Push dev to origin/dev for the first time?')
    expect(canPush({ branch: 'main', upstream: 'origin/main', ahead: 0 })).toBe(false)
    expect(canPush({ branch: 'main', upstream: 'origin/main', ahead: 3 })).toBe(true)
    expect(canPush({ branch: 'dev', upstream: null, ahead: 0 })).toBe(true)
    expect(canPush({ branch: 'HEAD', upstream: null, ahead: 0 })).toBe(false)
    expect(canPush(null)).toBe(false)
  })

  it('keeps the older commits below a fresh newest page', () => {
    const c = (hash) => ({ hash })
    const shown = [c('e'), c('d'), c('c'), c('b'), c('a')]
    // two new commits on top: the page now ends at d, and c, b, a stay below
    expect(withNewestPage([c('g'), c('f'), c('e'), c('d')], shown).map((x) => x.hash)).toEqual(['g', 'f', 'e', 'd', 'c', 'b', 'a'])
    // nothing new
    expect(withNewestPage([c('e'), c('d')], shown).map((x) => x.hash)).toEqual(['e', 'd', 'c', 'b', 'a'])
    // history rewritten: start over
    expect(withNewestPage([c('x'), c('y')], shown)).toBeNull()
    expect(withNewestPage([], shown)).toBeNull()
  })

  it('asks the companion in plain words', () => {
    expect(ASK_TO_COMMIT).toBe('Please commit the current changes with a clear message.')
  })
})
