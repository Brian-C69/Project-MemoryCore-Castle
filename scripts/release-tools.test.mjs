import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { VERSION_FILES, assetName, bumpText, changelogSection, changelogSections, compareVersions, downloadUrl, isVersion, latestJson, releaseChangelog } from './release-tools.mjs'

describe('versions', () => {
  it('reads and orders plain versions', () => {
    expect(isVersion('0.2.0')).toBe(true)
    expect(isVersion('v0.2.0')).toBe(false)
    expect(isVersion('0.2')).toBe(false)
    expect(compareVersions('0.2.0', '0.1.9')).toBe(1)
    expect(compareVersions('0.10.0', '0.9.0')).toBe(1)
    expect(compareVersions('1.0.0', '1.0.0')).toBe(0)
    expect(compareVersions('0.1.0', '0.2.0')).toBe(-1)
    expect(() => compareVersions('x', '0.1.0')).toThrow()
  })

  it('writes the version in every file that carries it, and nothing else', () => {
    const pkg = '{\n  "name": "castle",\n  "version": "0.1.0",\n  "dependencies": { "a": { "version": "9.9.9" } }\n}\n'
    expect(bumpText('package.json', pkg, '0.2.0')).toBe(pkg.replace('"0.1.0"', '"0.2.0"'))
    const lock = '{\n  "version": "0.1.0",\n  "packages": {\n    "": {\n      "version": "0.1.0"\n    },\n    "node_modules/x": {\n      "version": "7.0.0"\n    }\n  }\n}\n'
    const bumped = bumpText('package-lock.json', lock, '0.2.0')
    expect(bumped.match(/"0\.2\.0"/g)).toHaveLength(2)
    expect(bumped).toContain('"7.0.0"')
    const toml = '[package]\nname = "memorycore-castle"\nversion = "0.1.0"\n\n[dependencies]\nserde = { version = "1" }\n'
    expect(bumpText('src-tauri/Cargo.toml', toml, '0.2.0')).toContain('version = "0.2.0"\n\n[dependencies]\nserde = { version = "1" }')
    const cargoLock = '[[package]]\nname = "memchr"\nversion = "2.7.4"\n\n[[package]]\nname = "memorycore-castle"\nversion = "0.1.0"\n'
    expect(bumpText('src-tauri/Cargo.lock', cargoLock, '0.2.0')).toBe(cargoLock.replace('"0.1.0"', '"0.2.0"'))
    expect(() => bumpText('README.md', 'x', '0.2.0')).toThrow()
    expect(() => bumpText('package.json', '{}', '0.2.0')).toThrow()
  })

  it('finds the version in the real files', () => {
    for (const file of VERSION_FILES) {
      const text = readFileSync(new URL(`../${file}`, import.meta.url), 'utf8')
      expect(() => bumpText(file, text, '9.9.9'), file).not.toThrow()
    }
  })
})

describe('the changelog', () => {
  const log = '# Changelog\n\nintro\n\n## Unreleased\n\n- a new thing\n\n## 0.2.0\n\n- the room\n\n## 0.1.0 - 2026-11-09\n\n- the first\n'

  it('reads each version and its words', () => {
    const s = changelogSections(log)
    expect(s.map((x) => x.version)).toEqual([null, '0.2.0', '0.1.0'])
    expect(s[2].date).toBe('2026-11-09')
    expect(changelogSection(log, '0.1.0')).toBe('- the first')
    expect(changelogSection(log, '9.9.9')).toBe('')
  })

  it('moves what waits under Unreleased into a dated version', () => {
    const merged = releaseChangelog(log, '0.2.0', '2026-11-10')
    expect(merged).toContain('## Unreleased\n\n## 0.2.0 - 2026-11-10\n\n- a new thing\n- the room')
    expect(changelogSection(merged, '0.2.0')).toBe('- a new thing\n- the room')
    // a version not drafted yet is made right under Unreleased
    const next = releaseChangelog(log.replace('- a new thing', '- later'), '0.3.0', '2026-11-12')
    expect(next).toContain('## Unreleased\n\n## 0.3.0 - 2026-11-12\n\n- later\n\n## 0.2.0')
  })

  it('refuses a release with nothing to say', () => {
    const merged = releaseChangelog(log, '0.2.0', '2026-11-10')
    expect(() => releaseChangelog(merged, '0.3.0', '2026-11-12')).toThrow(/Nothing waits/)
    expect(() => releaseChangelog('# none', '0.3.0', '2026-11-12')).toThrow(/Unreleased/)
  })

  it('has words for the version in package.json', () => {
    const text = readFileSync(new URL('../CHANGELOG.md', import.meta.url), 'utf8')
    const version = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8')).version
    expect(changelogSection(text, version)).not.toBe('')
  })
})

describe('latest.json', () => {
  it('is the shape the updater reads', () => {
    const asset = assetName('MemoryCore Castle_0.2.0_x64-setup.exe')
    expect(asset).toBe('MemoryCore-Castle_0.2.0_x64-setup.exe')
    const url = downloadUrl('0.2.0', asset)
    expect(url).toBe('https://github.com/Kiyoraka/Project-MemoryCore-Castle/releases/download/v0.2.0/MemoryCore-Castle_0.2.0_x64-setup.exe')
    const j = latestJson({ version: '0.2.0', notes: '- new', date: Date.UTC(2026, 10, 10, 1, 2, 3), signature: 'abc\n', url })
    expect(j).toEqual({ version: '0.2.0', notes: '- new', pub_date: '2026-11-10T01:02:03Z', platforms: { 'windows-x86_64': { signature: 'abc', url } } })
    expect(() => latestJson({ version: '0.2.0', url })).toThrow()
  })
})
