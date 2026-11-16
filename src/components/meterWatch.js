// The gauges keep watch: the small decisions of the gear and the trip card, kept pure for the tests.

export const MIN_LIMIT = 50
export const MAX_LIMIT = 99
export const DEFAULT_LIMIT = 90

/** A value typed in the gear, held to 50..99. A blank or a word keeps the fallback. */
export function boundLimit(value, fallback = DEFAULT_LIMIT) {
  const n = Math.round(Number(value))
  if (!Number.isFinite(n) || String(value).trim() === '') return fallback
  return Math.min(MAX_LIMIT, Math.max(MIN_LIMIT, n))
}

/** 850 -> "850 MB", 1434 -> "1.4 GB". */
export function sizeLabel(mb) {
  const n = Number(mb) || 0
  return n >= 1024 ? `${(n / 1024).toFixed(1)} GB` : `${Math.round(n)} MB`
}

const OWNERS = {
  castle: 'Castle',
  companion: 'your companion',
  database: 'memory database',
  project: 'your project',
  server: 'server',
  editor: 'editor',
  system: 'Windows',
}

/** The tag shown beside a protected program, or '' for an ordinary app. */
export function ownerTag(owner) {
  return OWNERS[owner] ?? ''
}

/** The line after a tidy: what came back, said honestly. */
export function freedLine(freedMb) {
  if (freedMb == null) return 'Tidying RAM is not available on this system.'
  return freedMb > 0 ? `Tidied ${sizeLabel(freedMb)} - some comes back as apps use it.` : 'Nothing more to tidy right now.'
}

/** The trip card's first line. */
export function tripHeadline(trip) {
  if (!trip) return ''
  const meter = trip.meter === 'memory' ? 'RAM' : 'CPU'
  const head = `${meter} at ${trip.value}%`
  if (trip.meter !== 'memory') return `${head} - here is what is using it`
  if (trip.freedMb == null) return `${head} - here is what is using it`
  return trip.freedMb > 0
    ? `${head} - tidied ${sizeLabel(trip.freedMb)} (some comes back as apps use it)`
    : `${head} - nothing more to tidy`
}

/** "1:07 AM" for a trip's time. */
export function timeLabel(at) {
  const d = new Date(Number(at) || 0)
  return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })
}

/** True while a gauge reads at or above its value, so its ring glows warm. */
export function isHot(value, limit, on) {
  return Boolean(on) && Number(value) >= Number(limit)
}
