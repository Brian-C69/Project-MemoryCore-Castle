import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { describe, expect, it } from 'vitest'

// The screens explain themselves in a small (?) (HelpTip), never in walls of grey text. A static
// hint line - no {{ }}, no v-if / v-else - longer than this is an explanation that belongs in a tip.
const LIMIT = 160
const src = join(__dirname, '..')

function vueFiles(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) return vueFiles(path)
    return name.endsWith('.vue') ? [path] : []
  })
}

// Every class="hint" element on one line: its file, line number, attributes and inner text.
export function longHints(text, file = '', limit = LIMIT) {
  const found = []
  text.split(/\r?\n/).forEach((line, i) => {
    const open = line.match(/<(p|span|div|small)\b([^>]*\bclass="[^"]*\bhint\b[^"]*"[^>]*)>(.*?)<\/\1>/)
    if (!open) return
    const [, , attrs, inner] = open
    if (/\bv-(if|else-if|else|show)\b/.test(attrs) || inner.includes('{{')) return
    const words = inner.replace(/<[^>]+>/g, '').replace(/&[a-z]+;/g, ' ').trim()
    if (words.length > limit) found.push(`${file}:${i + 1} (${words.length} characters)`)
  })
  return found
}

describe('explanations live in a (?), not on the page', () => {
  it('finds a long static hint and lets live or short lines be', () => {
    const long = `<p class="hint">${'x'.repeat(200)}</p>`
    expect(longHints(long, 'A.vue')).toEqual(['A.vue:1 (200 characters)'])
    expect(longHints(`<p v-if="busy" class="hint">${'x'.repeat(200)}</p>`)).toEqual([])
    expect(longHints(`<p class="hint">{{ line }} ${'x'.repeat(200)}</p>`)).toEqual([])
    expect(longHints('<p class="hint">Skip any you don\'t have.</p>')).toEqual([])
  })

  it('has no wall of grey text on any screen', () => {
    const found = vueFiles(src).flatMap((path) => longHints(readFileSync(path, 'utf8'), relative(src, path).replaceAll('\\', '/')))
    expect(found, `move these into a HelpTip:\n${found.join('\n')}`).toEqual([])
  })
})
