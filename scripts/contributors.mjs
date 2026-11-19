// node scripts/contributors.mjs  (npm run contributors; npm run bump runs it too)
// Counts who has merged pull requests into Castle and packs the counts and their small GitHub
// avatars (the founder's too) into src/contributors/, for Settings > About's wall. Only this script asks GitHub; the
// page reads the files. Offline or refused, it says so in one line and keeps the old files, so a
// release never stops here.
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { REPO } from './release-tools.mjs'
import { OWNER, countMerged, countsFile, countsLine, smallAvatar } from './contributors-tools.mjs'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const folder = join(root, 'src', 'contributors')
const avatars = join(folder, 'avatars')

function token() {
  if (process.env.GITHUB_TOKEN) return process.env.GITHUB_TOKEN
  try {
    return execFileSync('gh', ['auth', 'token'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim() || null
  } catch {
    return null
  }
}

function headersFor(auth) {
  const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'memorycore-castle-contributors' }
  if (auth) headers.Authorization = `Bearer ${auth}`
  return headers
}

// The founder's avatar: the first circle on the wall (never counted).
async function founderAvatar(auth) {
  try {
    const answer = await fetch(`https://api.github.com/users/${OWNER}`, { headers: headersFor(auth) })
    return answer.ok ? (await answer.json()).avatar_url || null : null
  } catch {
    return null
  }
}

async function closedPulls(auth) {
  const headers = headersFor(auth)
  const all = []
  for (let page = 1; page <= 50; page++) {
    const answer = await fetch(`https://api.github.com/repos/${REPO}/pulls?state=closed&per_page=100&page=${page}`, { headers })
    if (!answer.ok) throw new Error(`GitHub answered ${answer.status}`)
    const pulls = await answer.json()
    all.push(...pulls)
    if (pulls.length < 100) break
  }
  return all
}

export async function refresh() {
  let people
  const auth = token()
  try {
    people = countMerged(await closedPulls(auth))
  } catch (why) {
    return `Contributors: kept the last counts (${why.message || why})`
  }
  mkdirSync(avatars, { recursive: true })
  const founder = { login: OWNER, avatar: await founderAvatar(auth) }
  for (const person of [founder, ...people]) {
    if (!person.avatar) continue
    try {
      const answer = await fetch(smallAvatar(person.avatar))
      if (answer.ok) writeFileSync(join(avatars, `${person.login}.png`), Buffer.from(await answer.arrayBuffer()))
    } catch {
      // the wall shows initials for this one until the next refresh
    }
  }
  const logins = new Set([OWNER, ...people.map((p) => p.login)].map((login) => `${login}.png`))
  for (const file of readdirSync(avatars)) {
    if (file.endsWith('.png') && !logins.has(file)) rmSync(join(avatars, file))
  }
  const file = countsFile(people, new Date().toISOString())
  writeFileSync(join(folder, 'counts.json'), `${JSON.stringify(file, null, 2)}\n`)
  return countsLine(file)
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  if (!existsSync(folder)) mkdirSync(folder, { recursive: true })
  console.log(await refresh())
}
