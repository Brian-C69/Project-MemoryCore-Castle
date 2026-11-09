// Built-in 2D worlds: each folder in ./worlds/ is one world, its pictures the scenes in name
// order. A new world is a new folder (and a line in TITLES for its name).
const files = import.meta.glob('./worlds/*/*.{jpg,jpeg,png,webp}', { eager: true, query: '?url', import: 'default' })

const TITLES = {
  'isles-of-memory': 'Isles of Memory',
}

export function builtInWorlds() {
  const byId = {}
  for (const [path, url] of Object.entries(files).sort(([a], [b]) => a.localeCompare(b))) {
    const id = path.split('/')[2]
    ;(byId[id] ??= { id, title: TITLES[id] ?? id.replace(/-/g, ' '), pictures: [] }).pictures.push(url)
  }
  return Object.values(byId)
}
