// The room as a chat: whose line is the member's own, who has read it (the ticks), and the
// letter drawn in a face circle when a speaker has no picture.

const AI_KINDS = ['assistant', 'member_assistant']
export const isAi = (author) => AI_KINDS.includes(author?.kind)

// Presence is not proof that this Castle still admits its companion: revoked keys can
// leave rows behind. Never interpret an unknown/stale status as online.
export function roomPresenceState(row, { me = null, companion = null, enabled = false, verified = false, fresh = false } = {}) {
  const owner = String(row.owner || row.ownerName || String(row.name ?? '').match(/^(.+)['\u2019]s\s+(.+)$/)?.[1] || '').trim()
  const own = isAi(row) && ((companion?.name && row.name === companion.name && row.kind === companion.kind) || (me?.name && owner === me.name))
  if (own && !enabled) return 'disabled'
  if (own && !verified) return 'unknown'
  if (!fresh) return 'unknown'
  return ['online', 'here', 'active', 'idle', 'away', 'offline'].includes(row.status) ? row.status : 'unknown'
}

// The service can retain several presence rows for revoked/recreated companion keys.
// Merge only the same raw companion identity, never the shortened display name.
export function roomMembers(rows = []) {
  const result = [], positions = new Map()
  const rank = (status) => ({ online: 3, here: 3, active: 3, idle: 2, away: 1, offline: 0 })[status] ?? 0
  for (const row of rows) {
    if (!isAi(row)) { result.push(row); continue }
    const key = JSON.stringify([row.kind, row.name])
    if (!positions.has(key)) { positions.set(key, result.length); result.push({ ...row }); continue }
    const index = positions.get(key), previous = result[index]
    const preferred = rank(row.status) > rank(previous.status) || (rank(row.status) === rank(previous.status) && Number(row.seen ?? 0) > Number(previous.seen ?? 0)) ? row : previous
    const other = preferred === row ? previous : row
    result[index] = { ...other, ...preferred, avatar: preferred.avatar || other.avatar, seen: Math.max(Number(previous.seen ?? 0), Number(row.seen ?? 0)) }
  }
  return result
}

// Display only: never rewrite the server identity used for ownership, avatars or replies.
export function speakerPresentation(author, hostName = '') {
  const raw = String(author?.name ?? '').trim()
  if (!isAi(author)) return { name: raw, owner: '', title: '' }
  const pair = raw.match(/^(.+)['’]s\s+(.+)$/)
  const name = String(author?.assistantName || pair?.[2] || raw).trim()
  const owner = String(author?.owner || author?.ownerName || pair?.[1] || (author?.kind === 'assistant' ? hostName : '')).trim()
  return { name, owner, title: owner ? `Master: ${owner}` : 'Companion' }
}

// The member's own line: one Castle posted for them (by id), or one matching their /me name and
// kind. A line from someone else who happens to share the name is never theirs by name alone
// unless the kind matches too.
export function isMine(line, myIds = [], me = null) {
  if (!line || line.author?.kind === 'system') return false
  if (myIds.includes(line.id)) return true
  return !!me?.name && line.author?.name === me.name && line.author?.kind === me.kind
}

// Who has read a line: every speaker whose seen reached its id, the member excluded.
export function readers(line, seen = [], me = null) {
  if (!line?.id) return []
  return seen
    .filter((s) => (s.seen ?? 0) >= line.id)
    .filter((s) => !(me && s.name === me.name && s.kind === me.kind))
    .map((s) => s.name)
}

// ✓ once the line is in the room, ✓✓ once someone else has read it.
export function ticks(line, seen, me) {
  if (!line?.id) return { mark: '', read: false, title: 'Sending…' }
  const who = readers(line, seen, me)
  return who.length ? { mark: '✓✓', read: true, title: `Read by ${who.join(', ')}` } : { mark: '✓', read: false, title: 'In the room' }
}

// The letter in a face circle without a picture: ✦ for an AI, else the first letter of the name.
export function initial(author) {
  if (isAi(author)) return '✦'
  const name = String(author?.name ?? '').trim()
  return name ? name[0].toUpperCase() : '?'
}

// Local calendar days, rather than elapsed 24-hour periods (including DST changes).
export function dayMarker(at, now = new Date()) {
  if (!at) return null
  const d = new Date(at)
  if (Number.isNaN(d.getTime())) return null
  const stamp = (x) => Date.UTC(x.getFullYear(), x.getMonth(), x.getDate())
  const age = (stamp(now) - stamp(d)) / 86400000
  const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  const date = d.toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' })
  const label = age === 0 ? 'Today' : age === 1 ? 'Yesterday' : age > 1 && age < 7 ? d.toLocaleDateString('en-GB', { weekday: 'long' }) : date
  return { key, label, date }
}

export function datedLines(lines, now = new Date()) {
  let previous = null
  return lines.flatMap((line) => {
    const day = dayMarker(line.at, now)
    const separator = day && day.key !== previous ? [{ kind: 'day', ...day }] : []
    if (day) previous = day.key
    return [...separator, { kind: 'message', key: `line-${line.id}`, line }]
  })
}

// A failed image is retryable; concurrent messages share one request per avatar token.
export function avatarLoader(fetchFace, onFace, { now = Date.now, retryMs = 10000 } = {}) {
  const loaded = new Set(), pending = new Map(), failed = new Map()
  return async (token) => {
    if (!token || loaded.has(token)) return
    if (pending.has(token)) return pending.get(token)
    if (failed.has(token) && now() - failed.get(token) < retryMs) return
    const request = Promise.resolve().then(() => fetchFace(token)).then((face) => {
      if (!face) throw new Error('Empty avatar')
      onFace(token, face)
      loaded.add(token)
      failed.delete(token)
    }).catch(() => { failed.set(token, now()) }).finally(() => pending.delete(token))
    pending.set(token, request)
    return request
  }
}

// A room file (a picture or a voice line) is fetched once per id however many lines ask at once, and one
// that failed is asked for again after a wait instead of staying "loading" for good. `onFile(id, data)`
// is called as each one arrives, in whatever order they finish: the page must add it to its files as they
// are THEN (not as they were when the request started), or loads that finish together wipe each other.
// `onFail` lets the page ask again after the wait even when no new line arrives.
export function fileLoader(fetchFile, onFile, { now = Date.now, retryMs = 10000, onFail = () => {} } = {}) {
  const loaded = new Set(), pending = new Map(), failed = new Map()
  return async (id) => {
    if (!id || loaded.has(id)) return
    if (pending.has(id)) return pending.get(id)
    if (failed.has(id) && now() - failed.get(id) < retryMs) return
    const request = Promise.resolve().then(() => fetchFile(id)).then((data) => {
      if (!data) throw new Error('Empty file')
      onFile(id, data)
      loaded.add(id)
      failed.delete(id)
    }).catch(() => {
      failed.set(id, now())
      onFail(id)
    }).finally(() => pending.delete(id))
    pending.set(id, request)
    return request
  }
}

export function currentAvatar(author, presence = []) {
  if (author?.avatar) return author.avatar
  const matches = presence.filter((p) => author?.name && p.name === author.name && p.kind === author.kind)
  return matches.length === 1 ? matches[0].avatar : null
}

// ---------- reactions and replies (shown only when the room lists them in /me's features) ----------
export const REACTIONS = ['👍', '❤️', '👏', '😂', '😮', '😢']

/** What the room can do, from /me: { replies, reactions, customReactions }. */
export function roomFeatures(me) {
  const f = Array.isArray(me?.features) ? me.features : []
  return { replies: f.includes('replies'), reactions: f.includes('reactions'), customReactions: f.includes('customReactions') }
}

/** A line's chips: the six first in their order, then any other emoji the room sent, as it sent them. */
export function reactionChips(line) {
  const list = (Array.isArray(line?.reactions) ? line.reactions : []).filter((r) => r && typeof r.emoji === 'string' && r.count > 0)
  const six = REACTIONS.map((e) => list.find((r) => r.emoji === e)).filter(Boolean)
  return [...six, ...list.filter((r) => !REACTIONS.includes(r.emoji))]
}

/** The picker's emoji beyond the six (any other single emoji can be typed or pasted). */
export const PICKER = [
  '🔥', '🎉', '🙏', '💜', '✨', '😍', '🥰', '🤔', '👀', '💯', '🥳', '😭',
  '🙌', '😅', '🤣', '😎', '🤗', '😴', '🤯', '😡', '👌', '✅', '💪', '🫶',
  '🌸', '⭐', '🌙', '☕', '🍰', '🎵', '💡', '🚀', '🐱', '🐶', '🌈', '🍀',
]

const PICTOGRAPH = /\p{Extended_Pictographic}|\p{Regional_Indicator}|⃣/u

/** Whether `text` is exactly one emoji (a flag, a keycap, a skin tone or a joined one included), at most 32 bytes. */
export function isOneEmoji(text) {
  const s = String(text ?? '').trim()
  if (!s || new TextEncoder().encode(s).length > 32 || !PICTOGRAPH.test(s) || /[A-Za-z\s]/.test(s)) return false
  const graphemes = [...new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(s)]
  return graphemes.length === 1
}

/** Whether pressing `emoji` on `line` adds it (true) or takes the member's own back (false). */
export function toggleReaction(line, emoji) {
  return !(line?.reactions ?? []).some((r) => r.emoji === emoji && r.mine)
}

/** The quote a reply shows: the original's name and its first words, and whether it is loaded. */
export function replySnippet(line, lines = []) {
  const reply = line?.replyTo
  if (!reply || typeof reply !== 'object') return null
  const original = lines.find((l) => l.id === reply.id)
  const text = String(reply.text ?? '').replace(/\s+/g, ' ').trim()
  return { id: reply.id, name: reply.name || 'someone', text: text || 'a removed message', loaded: Boolean(original), gone: !text }
}

/** The strip above the composer while replying: "Name: the start of the line". */
export function replyingLine(line, name) {
  const text = String(line?.text ?? '').replace(/\s+/g, ' ').trim()
  const short = text.length > 80 ? `${text.slice(0, 80)}…` : text || 'a picture or a voice line'
  return `${name}: ${short}`
}
