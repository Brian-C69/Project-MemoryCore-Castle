// The room as a chat: whose line is the member's own, who has read it (the ticks), and the
// letter drawn in a face circle when a speaker has no picture.

const AI_KINDS = ['assistant', 'member_assistant']
export const isAi = (author) => AI_KINDS.includes(author?.kind)

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
