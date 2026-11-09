// node scripts/tell-portfolio.mjs - run by the release workflow after publishing: sends the release
// (version, notes, where to download) to the MemoryCore Foundation's site, which keeps the public
// releases page. Needs PORTFOLIO_RELEASE_TOKEN; PORTFOLIO_RELEASE_URL overrides the address.
// The same version sent twice is the same release (the site keeps one).
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { REPO } from './release-tools.mjs'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const token = process.env.PORTFOLIO_RELEASE_TOKEN
if (!token) {
  console.log('No PORTFOLIO_RELEASE_TOKEN - the site is not told.')
  process.exit(0)
}
const address = process.env.PORTFOLIO_RELEASE_URL || 'https://www.afifmaahi-portfolio.com/api/integration/castle/releases'
const latest = JSON.parse(readFileSync(join(root, 'release-out', 'latest.json'), 'utf8'))
const repo = process.env.CASTLE_REPO || REPO
const body = {
  version: latest.version,
  notes: latest.notes,
  url: `https://github.com/${repo}/releases/tag/v${latest.version}`,
  downloadUrl: latest.platforms['windows-x86_64'].url,
  publishedAt: latest.pub_date,
}

const answer = await fetch(address, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}`, 'User-Agent': `MemoryCoreCastle-release/${latest.version}` },
  body: JSON.stringify(body),
})
if (!answer.ok) {
  console.error(`The site answered ${answer.status}: ${(await answer.text()).slice(0, 300)}`)
  process.exit(1)
}
console.log(`The site knows about ${latest.version} (${answer.status}).`)
