import { describe, expect, it } from 'vitest'
import { sectionFor, unseenSections } from './changelog.js'
import changelog from '../../CHANGELOG.md?raw'
import pkg from '../../package.json'

const log = '# Changelog\n\n## Unreleased\n\n- soon\n\n## 0.3.0 - 2026-12-01\n\n- three\n\n## 0.2.0 - 2026-11-10\n\n- two\n\n## 0.1.0\n\n- one\n'

describe("what's new", () => {
  it('says nothing on the very first launch or when nothing is new', () => {
    expect(unseenSections(log, null, '0.3.0')).toEqual([])
    expect(unseenSections(log, '0.3.0', '0.3.0')).toEqual([])
    expect(unseenSections(log, '0.3.0', '0.2.0')).toEqual([])
  })

  it('shows every version since the last one seen, newest first', () => {
    expect(unseenSections(log, '0.2.0', '0.3.0').map((s) => s.version)).toEqual(['0.3.0'])
    const both = unseenSections(log, '0.1.0', '0.3.0')
    expect(both.map((s) => s.version)).toEqual(['0.3.0', '0.2.0'])
    expect(both[1]).toEqual({ version: '0.2.0', date: '2026-11-10', body: '- two' })
    // a version not released yet (still under Unreleased) never shows
    expect(unseenSections(log, '0.1.0', '0.2.0').map((s) => s.version)).toEqual(['0.2.0'])
  })

  it('finds one version for About', () => {
    expect(sectionFor(log, '0.1.0').body).toBe('- one')
    expect(sectionFor(log, '9.9.9')).toBeNull()
  })

  it('has words for the running version in the bundled changelog', () => {
    expect(sectionFor(changelog, pkg.version)).not.toBeNull()
  })
})
