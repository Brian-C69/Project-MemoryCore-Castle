// Every world Castle can show on a screen, in one list:
//   3d/<slug>  a world made of model pieces (src/worlds/<slug>/world.json, pieces in public/worlds/<slug>/)
//   3d/floating-isles  the isles built from geometry alone - always ready, the fallback
//   3d/deep-space      a nebula and its stars drawn by code - always ready (recipe by Ashran Helmi)
//   2d/<slug>  a built-in slideshow (src/world2d/worlds/<slug>/)
//   2d/mine    the member's own pictures (theme wallpaper + what the companion drew)
//   my/<slug>  a world the member created with their companion (<memory>/worlds/<slug>/)
import { builtInWorlds as slideshows } from '../world2d/worlds.js'

const manifests = import.meta.glob('./*/world.json', { eager: true, import: 'default' })

export const WORLD_ID = /^(2d|3d|my)\/[a-z0-9][a-z0-9-]{0,47}$/
export const DEFAULT_WORLD = '3d/floating-isles'
export const ROLES = ['base', 'companion', 'islet', 'isle', 'prop']

export const isWorldId = (id) => typeof id === 'string' && WORLD_ID.test(id)

export const PROCEDURAL = {
  id: DEFAULT_WORLD,
  kind: '3d',
  engine: 'procedural',
  title: 'Floating Isles',
  line: 'Five little isles joined by bridges of light, the crystal wandering between them - built in, always ready',
  preview: '/worlds/floating-isles/preview.jpg',
  camera: { default: 'director', scale: 1 },
  pieces: [],
  source: 'built-in',
}

// Deep Space - recipe by Ashran Helmi · ashranhelmi.com (src/world3d/space.js).
export const SPACE = {
  id: '3d/deep-space',
  kind: '3d',
  engine: 'space',
  title: 'Deep Space',
  line: 'A nebula and its stars, drawn by code - recipe by Ashran Helmi · ashranhelmi.com',
  credit: 'Recipe by Ashran Helmi · ashranhelmi.com',
  preview: '/worlds/deep-space/preview.jpg',
  camera: { default: 'orbit', scale: 1 },
  pieces: [],
  source: 'built-in',
}

export const MY_PICTURES = {
  id: '2d/mine',
  kind: '2d',
  engine: 'mine',
  title: 'My pictures',
  line: 'Your theme wallpaper and everything your companion draws, as a slideshow',
  preview: '',
  pieces: [],
  source: 'yours',
}

// A world.json made into a registry entry; `base` is where its files are served from.
export function fromManifest(manifest, { idPrefix = '3d', base, source = 'built-in', previewUrl, urlOf = (file) => `${base}${file}` } = {}) {
  const slug = String(manifest.slug ?? '')
  const pieces = (manifest.pieces ?? []).filter((p) => p && ROLES.includes(p.role) && typeof p.file === 'string')
  return {
    id: `${idPrefix}/${slug}`,
    kind: '3d',
    engine: 'glb',
    title: manifest.title || slug.replace(/-/g, ' '),
    line: manifest.line || '',
    layout: manifest.layout === 'archipelago' ? 'archipelago' : 'centrepiece',
    camera: { default: 'director', scale: 1, ...(manifest.camera ?? {}) },
    glow: manifest.glow || '#8fe6ff',
    preview: previewUrl ?? `${base}preview.jpg`,
    pieces: pieces.map((p) => ({ ...p, url: urlOf(p.file) })),
    source,
  }
}

export function builtIn3d() {
  return Object.values(manifests)
    .map((m) => fromManifest(m, { base: `/worlds/${m.slug}/` }))
    .sort((a, b) => a.title.localeCompare(b.title))
}

export function builtIn2d() {
  return slideshows().map((w) => ({
    id: `2d/${w.id}`,
    kind: '2d',
    engine: 'slideshow',
    title: w.title,
    line: w.line ?? '',
    preview: w.pictures[0] ?? '',
    pictures: w.pictures,
    pieces: [],
    source: 'built-in',
  }))
}

// One of the member's own worlds (from worlds_mine) as a registry entry; `convert` turns a file
// path into a URL the page may load (convertFileSrc).
export function fromMine(item, convert) {
  const sep = item.folder.includes('\\') ? '\\' : '/'
  const entry = fromManifest({ ...item.manifest, slug: item.slug }, {
    idPrefix: 'my',
    source: 'yours',
    previewUrl: item.preview ? convert(item.preview) : '',
    urlOf: (file) => convert(`${item.folder}${sep}pieces${sep}${file}`),
  })
  return { ...entry, have: item.have, exports: item.exports, painted: item.painted, folder: item.folder, slug: item.slug }
}

// `mine` = the member's own created worlds (already entries, see fromMine).
export function allWorlds(mine = []) {
  return [...builtIn3d(), PROCEDURAL, SPACE, ...mine, ...builtIn2d(), MY_PICTURES]
}

export const findWorld = (list, id) => list.find((w) => w.id === id) ?? null
