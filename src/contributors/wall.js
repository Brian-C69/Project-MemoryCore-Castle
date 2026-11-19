// Settings > About's contributors wall, kept pure so it is tested: the counted people joined with
// the hand-written names and lists, in count order, the first `limit` shown until "Show all".

export const WALL_LIMIT = 8
export const NO_LIST = 'Thank you for your pull requests.'

// "Bernard Choong" -> "BC", "OneNonly148" -> "ON", "rin" -> "R".
export function initialsOf(name = '') {
  const words = String(name).trim().split(/[\s_-]+/).filter(Boolean)
  if (words.length > 1) return (words[0][0] + words[1][0]).toUpperCase()
  const word = words[0] || '?'
  const caps = word.match(/[A-Z]/g)
  return (caps && caps.length > 1 ? caps.slice(0, 2).join('') : word[0]).toUpperCase()
}

// -> { total, merged, hidden, people: [{ login, name, merged, built, initials }] }
export function wall(counts, people = {}, { all = false, limit = WALL_LIMIT } = {}) {
  const everyone = (counts?.people || [])
    .filter((p) => p?.login && p.merged > 0)
    .slice()
    .sort((a, b) => b.merged - a.merged || a.login.localeCompare(b.login))
    .map(({ login, merged }) => {
      const known = people[login] || {}
      const name = known.name || login
      return { login, name, merged, built: known.built?.length ? known.built : [NO_LIST], initials: initialsOf(name) }
    })
  const shown = all ? everyone : everyone.slice(0, limit)
  return {
    total: everyone.length,
    merged: everyone.reduce((sum, p) => sum + p.merged, 0),
    hidden: everyone.length - shown.length,
    people: shown,
  }
}

export const contributorsLine = (n) => `${n} ${n === 1 ? 'contributor' : 'contributors'}`
export const mergedLine = (n) => `${n} merged`
