import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

// A component used in App.vue's template but never imported is not a build error: Vue draws it as an
// unknown element and the card or room silently never shows. This reads the file and says so.
describe('App.vue', () => {
  const source = readFileSync(new URL('./App.vue', import.meta.url), 'utf8')
  const used = [...new Set([...source.matchAll(/<([A-Z][A-Za-z0-9]+)[ \n\r/>]/g)].map((m) => m[1]))]
  it('finds the components it uses', () => {
    expect(used.length).toBeGreaterThan(5)
    expect(used).toContain('CalendarWidget')
  })
  it.each(used)('imports <%s>', (name) => {
    expect(source.includes(`import ${name} from '`)).toBe(true)
  })
})
