import { describe, expect, it } from 'vitest'
import { MAX_MS, MIN_MS, cleanQuips, holdFor, nextQuip, percent, versionLabel } from './loader.js'
import { version } from '../../package.json'
import { QUIPS } from './quips.js'

describe("Castle's start-up screen", () => {
  it('has plenty of lines, none empty, none repeated, none too long to read', () => {
    expect(QUIPS.length).toBeGreaterThanOrEqual(20)
    expect(new Set(QUIPS).size).toBe(QUIPS.length)
    for (const q of QUIPS) {
      expect(q.trim()).toBe(q)
      expect(q.length).toBeGreaterThan(5)
      expect(q.length).toBeLessThanOrEqual(80)
    }
  })

  it('never shows the same line twice in a row', () => {
    const list = ['a', 'b', 'c']
    let last = ''
    for (let i = 0; i < 60; i += 1) {
      const pick = nextQuip(list, last)
      expect(pick).not.toBe(last)
      last = pick
    }
    expect(nextQuip(['only'], 'only')).toBe('only')
    expect(nextQuip([], '')).toBe('')
  })

  it('shows the version of the build, and nothing when there is none', () => {
    expect(versionLabel('0.2.14')).toBe('v0.2.14')
    expect(versionLabel(' 1.0.0-beta.1 ')).toBe('v1.0.0-beta.1')
    expect(versionLabel('')).toBe('')
    expect(versionLabel(undefined)).toBe('')
    expect(versionLabel('<script>')).toBe('')
    expect(versionLabel(version)).toMatch(/^v\d+\.\d+\.\d+/)
  })

  it('fills the bar from a sliver to full, however many pieces there are', () => {
    expect(percent(0, 5)).toBe(4)
    expect(percent(0, 0)).toBe(4)
    expect(percent(2, 5)).toBe(40)
    expect(percent(5, 5)).toBe(100)
    expect(percent(9, 5)).toBe(100)
  })

  it('stays up for the minimum so the words can be read, and never waits a negative time', () => {
    expect(holdFor(0)).toBe(MIN_MS)
    expect(holdFor(MIN_MS - 400)).toBe(400)
    expect(holdFor(MIN_MS + 5000)).toBe(0)
    expect(MAX_MS).toBeGreaterThan(MIN_MS * 4)
  })

  it("accepts a build's own lines only when they are plain, short strings", () => {
    expect(cleanQuips(['  Hello there  ', '', 3, null, 'x'.repeat(121), 'Fine'])).toEqual(['Hello there', 'Fine'])
    expect(cleanQuips('not a list')).toEqual([])
    expect(cleanQuips(undefined)).toEqual([])
    expect(cleanQuips(Array.from({ length: 100 }, (_, i) => `line ${i}`)).length).toBe(60)
  })
})
