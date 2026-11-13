import { describe, expect, it } from 'vitest'
import { absPath, fileFacts, languageOf, openedFor, sizeWords, textLines, visibleRows } from './tree.js'

const levels = {
  '': [
    { name: 'resources', rel: 'resources', dir: true },
    { name: 'node_modules', rel: 'node_modules', dir: true, ignored: true },
    { name: 'composer.json', rel: 'composer.json', dir: false },
  ],
  resources: [{ name: 'js', rel: 'resources/js', dir: true }],
  'resources/js': [{ name: 'app.js', rel: 'resources/js/app.js', dir: false }],
}

describe('the FILES helpers', () => {
  it('says sizes', () => {
    expect([912, 2150, 3_400_000].map(sizeWords)).toEqual(['912 B', '2.1 KB', '3.2 MB'])
    expect(sizeWords(null)).toBe('')
    expect(sizeWords(undefined)).toBe('')
  })

  it('shows opened folders under their parents', () => {
    const rows = visibleRows(levels, new Set(['resources']))
    expect(rows.map((r) => [r.entry.name, r.depth])).toEqual([
      ['resources', 0],
      ['js', 1],
      ['node_modules', 0],
      ['composer.json', 0],
    ])
    expect(visibleRows(levels, new Set()).map((r) => r.entry.name)).toEqual(['resources', 'node_modules', 'composer.json'])
  })

  it('filters to matches and the folders above them', () => {
    const rows = visibleRows(levels, new Set(), 'APP')
    expect(rows.map((r) => r.entry.name)).toEqual(['resources', 'js', 'app.js'])
    expect(visibleRows(levels, new Set(), 'zzz')).toEqual([])
  })

  it('names languages', () => {
    expect([languageOf('app.js'), languageOf('Home.vue'), languageOf('LICENSE'), languageOf('.gitignore')]).toEqual(['JavaScript', 'Vue', '', ''])
  })

  it('keeps opened folders per project', () => {
    openedFor('D:\\a').add('src')
    expect(openedFor('D:\\a').has('src')).toBe(true)
    expect(openedFor('D:\\b').has('src')).toBe(false)
  })

  it('builds the whole path for the chat', () => {
    expect(absPath('D:\\Work\\Baek\\', 'resources/js/app.js')).toBe('D:\\Work\\Baek\\resources\\js\\app.js')
    expect(absPath('D:/Work/Baek', 'a.txt')).toBe('D:/Work/Baek\\a.txt')
  })

  it('splits text into lines and says the facts', () => {
    expect(textLines('a\r\nb\n')).toEqual(['a', 'b'])
    expect(textLines('')).toEqual([''])
    expect(fileFacts({ kind: 'text', text: 'x\ny\n', size: 2150 }, 'app.js')).toBe('2.1 KB · 2 lines · JavaScript')
    expect(fileFacts({ kind: 'text', text: 'x', size: 1 }, 'notes')).toBe('1 B · 1 line')
    expect(fileFacts({ kind: 'picture', url: 'data:', size: 912 }, 'a.png')).toBe('912 B')
    expect(fileFacts(null, 'a.js')).toBe('')
  })
})
