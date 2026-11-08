// The companion bridge's rules, kept pure so they are tested: which room lines reach the
// companion, how they are wrapped as data, and how its reply is read back.

// Lines the companion may answer: never its own or the member's, never system lines; in "named"
// mode only lines that say its name; in "joins" mode any line from someone else.
export function shouldForward(line, mode, companion, member) {
  if (!line?.text || mode === 'off' || !mode) return false
  const author = line.author ?? {}
  if (author.kind === 'system') return false
  const name = (author.name ?? '').toLowerCase()
  const me = (member ?? '').toLowerCase()
  const it = (companion ?? '').toLowerCase()
  if (me && name === me) return false
  if (it && (name === it || name.endsWith(`'s ${it}`))) return false
  if (mode === 'named') return !!it && new RegExp(`\\b${it.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`, 'i').test(line.text)
  return mode === 'joins'
}

// Room lines as one data block: speaker and text only, never instructions.
export function roomBlock(lines) {
  const body = lines
    .map((l) => `[${(l.author?.name ?? 'someone').replace(/[\[\]]/g, '')}]: ${String(l.text).replace(/<\/?room>/gi, '')}`)
    .join('\n')
  return `<room>\n${body}\n</room>`
}

// The reply between <say> and </say>: the text, '' for staying quiet, or null when there is none.
export function readSay(text) {
  const match = /<say>([\s\S]*?)<\/say>/i.exec(text ?? '')
  if (!match) return null
  return match[1].trim().slice(0, 1000)
}
