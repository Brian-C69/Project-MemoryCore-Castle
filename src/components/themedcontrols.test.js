// The rule for native controls: they follow the theme. A plain <select> lets Windows draw a grey
// list that ignores it, and a scrollbar colour written as a fixed value ignores a theme that sets
// --scrollbar-thumb. This reads every page and fails on a new one, so the rule keeps holding.
import { describe, expect, it } from 'vitest'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))

function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) return files(path)
    return path.endsWith('.vue') ? [path] : []
  })
}
const pages = files(root).map((path) => ({ path: path.slice(root.length).split(String.fromCharCode(92)).join('/'), text: readFileSync(path, 'utf8') }))
const styleOf = (text) => [...text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]).join('\n')
const templateOf = (text) => (text.match(/<template>([\s\S]*)<\/template>/) ?? ['', ''])[1]

describe('native controls follow the theme', () => {
  it('no page uses a plain <select>; GlassSelect draws the list in the theme', () => {
    const offenders = pages.filter((p) => !p.path.endsWith('GlassSelect.vue') && /<select[\s>]/.test(templateOf(p.text)))
    expect(offenders.map((p) => p.path)).toEqual([])
  })

  it('scrollbar colours and width read the theme variables, not fixed values', () => {
    const bad = []
    for (const p of pages) {
      for (const m of styleOf(p.text).matchAll(/(?<!-)scrollbar-(color|width):\s*([^;]+);/g)) {
        const value = m[2].trim()
        const ok = m[1] === 'color' ? value === 'var(--scrollbar-thumb) var(--scrollbar-track)' : value === 'var(--scrollbar-width)' || value === 'none'
        if (!ok) bad.push(`${p.path}: scrollbar-${m[1]}: ${value}`)
      }
    }
    expect(bad).toEqual([])
  })

  it('the theme variables exist with defaults, and a theme may set them like any colour', () => {
    const app = pages.find((p) => p.path === 'App.vue').text
    for (const name of ['--scrollbar-thumb', '--scrollbar-track', '--scrollbar-width', '--control-accent', '--color-scheme']) {
      expect(app).toContain(`${name}:`)
    }
    expect(app).toMatch(/color-scheme:\s*var\(--color-scheme\)/)
    expect(app).toMatch(/accent-color:\s*var\(--control-accent\)/)
  })
})
