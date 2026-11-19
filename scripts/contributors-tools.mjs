// The contributors wall's pure parts (Settings > About), shared by contributors.mjs and its test:
// who has merged pull requests, and the file the page reads. The page itself never asks GitHub.

export const OWNER = 'Kiyoraka'

const isBot = (user) => !user || user.type === 'Bot' || /\[bot\]$/i.test(user.login || '')

// Merged pull requests per login, the owner and bots left out; the most first, then by login.
// `pulls` are GitHub's pull objects (state=closed); a closed pull that was never merged counts nothing.
export function countMerged(pulls, owner = OWNER) {
  const counts = new Map()
  const avatars = new Map()
  for (const pull of pulls || []) {
    const user = pull?.user
    if (!pull?.merged_at || isBot(user)) continue
    if (user.login.toLowerCase() === owner.toLowerCase()) continue
    counts.set(user.login, (counts.get(user.login) || 0) + 1)
    if (user.avatar_url) avatars.set(user.login, user.avatar_url)
  }
  return [...counts.entries()]
    .map(([login, merged]) => ({ login, merged, avatar: avatars.get(login) || null }))
    .sort((a, b) => b.merged - a.merged || a.login.localeCompare(b.login))
}

// The file the page reads: when it was counted, the merged total and each person.
export function countsFile(people, at) {
  return {
    at,
    merged: people.reduce((sum, p) => sum + p.merged, 0),
    people: people.map(({ login, merged }) => ({ login, merged })),
  }
}

// One line for the release road.
export function countsLine(file) {
  const n = file.people.length
  return `Contributors: ${n} ${n === 1 ? 'contributor' : 'contributors'}, ${file.merged} merged`
}

// A GitHub avatar at a small size (avatar urls may already carry a query).
export function smallAvatar(url, size = 96) {
  return `${url}${url.includes('?') ? '&' : '?'}s=${size}`
}
