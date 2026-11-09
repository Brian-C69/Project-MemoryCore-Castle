// node scripts/bump.mjs 0.2.0
// Writes the new version into every file that carries it and dates its changelog section (what
// waits under "## Unreleased" moves into it). Then commit, tag vX.Y.Z and push the tag: the
// release workflow builds, signs and publishes it.
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { VERSION_FILES, bumpText, compareVersions, isVersion, releaseChangelog, versionOf } from './release-tools.mjs'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const version = process.argv[2]
if (!isVersion(version)) {
  console.error('Usage: node scripts/bump.mjs <x.y.z>')
  process.exit(1)
}
const current = versionOf(readFileSync(join(root, 'package.json'), 'utf8'))
if (compareVersions(version, current) <= 0) {
  console.error(`${version} is not newer than ${current}.`)
  process.exit(1)
}

// everything is worked out first, then written, so a refusal changes nothing
const today = new Date()
const date = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`
const writes = VERSION_FILES.map((file) => {
  const path = join(root, file)
  return [path, bumpText(file, readFileSync(path, 'utf8'), version)]
})
const changelog = join(root, 'CHANGELOG.md')
writes.push([changelog, releaseChangelog(readFileSync(changelog, 'utf8'), version, date)])
for (const [path, text] of writes) writeFileSync(path, text)

console.log(`Castle ${current} -> ${version}. Next: commit, then "git tag v${version}" and push the tag.`)
