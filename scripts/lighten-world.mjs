// Prepares one of Castle's own 3D worlds from its exports:
//   npm run world:lighten -- crystal-isle [exports folder]
// The exports default to "Project Resources/Design/<world>/hd" (GLBs named by their picture's
// number); the prepared pieces land in public/worlds/<world>/ beside its preview.
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'

const [world, exportsFolder] = process.argv.slice(2)
if (!world || !/^[a-z0-9][a-z0-9-]*$/.test(world)) {
  console.error('Say which world: npm run world:lighten -- <world> [exports folder]')
  process.exit(2)
}
const root = resolve(import.meta.dirname, '..')
const manifest = join(root, 'src', 'worlds', world, 'world.json')
if (!existsSync(manifest)) {
  console.error(`There is no src/worlds/${world}/world.json`)
  process.exit(2)
}
const from = resolve(exportsFolder ?? join(root, 'Project Resources', 'Design', world, 'hd'))
const to = join(root, 'public', 'worlds', world)
const helper = join(root, 'src-tauri', 'resources', 'tools', 'castle-world.ps1')
const run = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', helper, '-Prepare', '-Manifest', manifest, '-In', from, '-Out', to], { stdio: 'inherit' })
process.exit(run.status ?? 1)
