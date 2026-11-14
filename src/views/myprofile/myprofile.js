// MY PROFILE's pure helpers: the deltas on the tiles, which months glow on the timeline, and the
// scores and notes grouped for the three cards.

export const CARDS = [
  { grp: 'think', title: 'How I Think', tone: 'gold' },
  { grp: 'values', title: 'Values', tone: 'gold' },
  { grp: 'shadow', title: 'Shadow', tone: 'violet' },
]

// A tile that moved this far, or a new type, makes the month's dot glow.
export const GLOW_MOVE = 10

const SHORT = ['JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC']
const LONG = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']

const monthIndex = (month) => Number(String(month ?? '').slice(5, 7)) - 1

// '2026-03' -> 'MAR'
export function monthShort(month) {
  return SHORT[monthIndex(month)] ?? String(month ?? '')
}

// '2026-03' -> 'March 2026'
export function monthLong(month) {
  const name = LONG[monthIndex(month)]
  return name ? `${name} ${String(month).slice(0, 4)}` : String(month ?? '')
}

// The year under a dot: on the first month and wherever the year turns.
export function yearMark(timeline, i) {
  const year = String(timeline[i]?.month ?? '').slice(0, 4)
  return i === 0 || String(timeline[i - 1]?.month ?? '').slice(0, 4) !== year ? year : ''
}

const tileMap = (point) => new Map((point?.headline ?? []).map((s) => [s.label, s.score]))

// How far each tile moved since the month before: { Growth: 6, Focus: -4 }; a tile with nothing to
// compare against is left out.
export function tileDeltas(timeline, month) {
  const i = timeline.findIndex((p) => p.month === month)
  if (i <= 0) return {}
  const now = tileMap(timeline[i])
  const before = tileMap(timeline[i - 1])
  const out = {}
  for (const [label, score] of now) if (before.has(label)) out[label] = score - before.get(label)
  return out
}

// 6 -> '+6', -4 -> '-4', 0 -> '0'
export function deltaText(n) {
  if (n == null) return ''
  return n > 0 ? `+${n}` : String(n)
}

// The months whose dot glows: the type changed from the month before, or a tile moved by GLOW_MOVE
// or more. The first month never glows (there is nothing before it).
export function glowing(timeline) {
  const out = new Set()
  timeline.forEach((p, i) => {
    if (i === 0) return
    const typeChanged = Boolean(p.mbti && timeline[i - 1].mbti && p.mbti !== timeline[i - 1].mbti)
    const moved = Object.values(tileDeltas(timeline, p.month)).some((d) => Math.abs(d) >= GLOW_MOVE)
    if (typeChanged || moved) out.add(p.month)
  })
  return out
}

// A reading's scores and notes by group, each in the order it was kept.
export function grouped(reading) {
  const scores = { headline: [], think: [], values: [], shadow: [] }
  const notes = { think: [], values: [], shadow: [], patterns: [], changes: [] }
  for (const s of reading?.scores ?? []) scores[s.grp]?.push(s)
  for (const n of reading?.notes ?? []) notes[n.grp]?.push(n.line)
  return { scores, notes }
}

// A bar's width, never past the track.
export function barWidth(score) {
  return `${Math.max(0, Math.min(100, Number(score) || 0))}%`
}

// The scan's line while it reads: 'Reading March 2026… (3 of 9)'.
export function progressLine(p) {
  if (!p) return ''
  if (p.state === 'reading') return `Reading ${monthLong(p.month)}… (${p.done + 1} of ${p.total})`
  if (p.state === 'skipped') return `${monthLong(p.month)} was passed over: ${p.problem ?? 'it did not come back right'}`
  return `${monthLong(p.month)} read (${p.done} of ${p.total})`
}
