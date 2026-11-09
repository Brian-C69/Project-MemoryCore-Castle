// node scripts/latest-json.mjs [out-folder]
// After an official build: copies the installer and its signature under plain names into the
// out folder (default release-out/) and writes latest.json and notes.md beside them - the three
// files a GitHub release carries, and the notes it shows.
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { REPO, assetName, changelogSection, downloadUrl, latestJson, versionOf } from './release-tools.mjs'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const out = join(root, process.argv[2] ?? 'release-out')
const repo = process.env.CASTLE_REPO || REPO
const version = versionOf(readFileSync(join(root, 'package.json'), 'utf8'))
const nsis = join(root, 'src-tauri', 'target', 'release', 'bundle', 'nsis')

const files = existsSync(nsis) ? readdirSync(nsis) : []
const installer = files.find((f) => f.includes(`_${version}_`) && f.endsWith('-setup.exe'))
if (!installer || !files.includes(`${installer}.sig`)) {
  console.error(`No signed ${version} installer in ${nsis} - build the official release first.`)
  process.exit(1)
}

const notes = changelogSection(readFileSync(join(root, 'CHANGELOG.md'), 'utf8'), version)
if (!notes) console.warn(`CHANGELOG.md has no words for ${version}; the release will have no notes.`)

mkdirSync(out, { recursive: true })
const asset = assetName(installer)
copyFileSync(join(nsis, installer), join(out, asset))
copyFileSync(join(nsis, `${installer}.sig`), join(out, `${asset}.sig`))
const latest = latestJson({
  version,
  notes,
  date: Date.now(),
  signature: readFileSync(join(nsis, `${installer}.sig`), 'utf8'),
  url: downloadUrl(version, asset, repo),
})
writeFileSync(join(out, 'latest.json'), JSON.stringify(latest, null, 2) + '\n')
writeFileSync(join(out, 'notes.md'), (notes || `MemoryCore Castle ${version}`) + '\n')
console.log(`Release files for ${version} in ${out}: ${asset}, ${asset}.sig, latest.json, notes.md`)
