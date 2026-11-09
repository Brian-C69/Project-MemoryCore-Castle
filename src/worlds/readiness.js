// Is a world ready to show? A model world is ready when every one of its pieces is there. A piece
// counts when it answers OK and is not an HTML page (a dev server answers a missing file with its
// index page). The member's own worlds arrive with their count already read from the folder.
const cache = new Map()

export async function pieceThere(url, fetcher = globalThis.fetch) {
  try {
    const answer = await fetcher(url, { method: 'HEAD', cache: 'no-store' })
    if (!answer.ok) return false
    const type = answer.headers?.get?.('content-type') ?? ''
    return !type.includes('text/html')
  } catch {
    return false
  }
}

export async function readiness(world, fetcher = globalThis.fetch) {
  if (!world) return { have: 0, total: 0, ready: false }
  if (world.engine !== 'glb') return { have: 0, total: 0, ready: true }
  if (typeof world.have === 'number') {
    const total = world.pieces.length
    return { have: world.have, total, ready: total > 0 && world.have >= total }
  }
  const there = await Promise.all(world.pieces.map((p) => pieceThere(p.url, fetcher)))
  const have = there.filter(Boolean).length
  const total = world.pieces.length
  return { have, total, ready: total > 0 && have === total }
}

// Kept for the visit; `fresh` asks again (after "Look for screens" or a prepare).
export async function readinessOf(world, { fresh = false, fetcher } = {}) {
  if (!world) return { have: 0, total: 0, ready: false }
  if (!fresh && world.source !== 'yours' && cache.has(world.id)) return cache.get(world.id)
  const result = await readiness(world, fetcher)
  cache.set(world.id, result)
  return result
}

export function statusLine(world, state) {
  if (!world) return ''
  if (world.engine === 'procedural') return 'ready · built in'
  if (world.engine === 'slideshow') return `ready · ${world.pictures?.length ?? 0} pictures`
  if (world.engine === 'mine') return 'your pictures'
  if (!state) return 'looking…'
  if (state.ready) return 'ready'
  return `waiting · ${state.have}/${state.total} · make in Tripo`
}
