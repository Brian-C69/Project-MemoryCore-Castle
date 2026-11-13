// The Source panel's small decisions, plain and tested: what a file's letter means, how long ago a
// commit was, the order of a commit's badges, a diff's lines, which paths a commit of the ticked
// files takes, which files stay ticked after a fresh read, what the push question says, and which
// card shows first (Start git here, Connect to GitHub, or the normal view).

const WORDS = { M: 'modified', A: 'added', D: 'deleted', R: 'renamed', C: 'copied', U: 'conflict', '??': 'new', T: 'type changed' }

// How many commits one page of the log holds (the core's page size).
export const LOG_PAGE = 50

// The words the "Ask {companion} to commit" button types into the session.
export const ASK_TO_COMMIT = 'Please commit the current changes with a clear message.'

/** "M" / " M" / "MM" / "??" -> { letter, word, tone } for the file row. */
export function codeOf(code) {
  const text = String(code ?? '').trim()
  if (text === '??') return { letter: '??', word: WORDS['??'], tone: 'new' }
  if (/U/.test(text) || text === 'AA' || text === 'DD') return { letter: 'U', word: WORDS.U, tone: 'conflict' }
  const letter = text.replace(/[^MADRCT]/g, '')[0] ?? 'M'
  const tone = { A: 'new', D: 'gone', R: 'moved', C: 'new' }[letter] ?? 'changed'
  return { letter, word: WORDS[letter] ?? 'changed', tone }
}

/** "2 minutes ago", "3 hours ago", "yesterday", "5 days ago", else the date. */
export function ago(iso, now = Date.now()) {
  const at = new Date(iso).getTime()
  if (!Number.isFinite(at)) return ''
  const seconds = Math.max(0, Math.round((now - at) / 1000))
  const plural = (n, unit) => `${n} ${unit}${n === 1 ? '' : 's'} ago`
  if (seconds < 60) return 'just now'
  if (seconds < 3600) return plural(Math.floor(seconds / 60), 'minute')
  if (seconds < 86400) return plural(Math.floor(seconds / 3600), 'hour')
  const days = Math.floor(seconds / 86400)
  if (days === 1) return 'yesterday'
  if (days < 30) return plural(days, 'day')
  return new Date(at).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })
}

/** A commit's badges in the usual order: local branches, then remotes, then tags. */
export function badges(refs) {
  const kind = (ref) => (ref.startsWith('tag: ') ? 2 : ref.includes('/') ? 1 : 0)
  return [...(refs ?? [])]
    .filter((ref) => ref && ref !== 'HEAD' && !ref.endsWith('/HEAD'))
    .map((ref) => ({ text: ref.replace(/^tag: /, ''), kind: ['branch', 'remote', 'tag'][kind(ref)], order: kind(ref) }))
    .sort((a, b) => a.order - b.order)
}

/** A diff's lines, each with its kind for colour: add, del, hunk, meta or ctx. */
export function diffLines(text) {
  return String(text ?? '')
    .replace(/\r/g, '')
    .split('\n')
    .map((line) => {
      if (/^(diff --git|index |--- |\+\+\+ |new file|deleted file|similarity|rename (from|to))/.test(line)) return { kind: 'meta', text: line }
      if (line.startsWith('@@')) return { kind: 'hunk', text: line }
      if (line.startsWith('+')) return { kind: 'add', text: line }
      if (line.startsWith('-')) return { kind: 'del', text: line }
      return { kind: 'ctx', text: line }
    })
}

/** The paths a commit of the ticked files takes: a rename brings its old path with it. */
export function commitPaths(files, ticked) {
  const out = []
  for (const file of files ?? []) {
    if (!ticked.has(file.path)) continue
    out.push(file.path)
    if (file.from) out.push(file.from)
  }
  return out
}

/**
 * The ticks after a fresh read: files that are gone lose their tick, files seen for the first
 * time come ticked, and a file the member unticked stays unticked.
 */
export function nextTicks(oldFiles, ticked, freshFiles) {
  const known = new Set((oldFiles ?? []).map((file) => file.path))
  const fresh = freshFiles ?? []
  const next = new Set([...(ticked ?? [])].filter((path) => fresh.some((file) => file.path === path)))
  for (const file of fresh) if (!known.has(file.path)) next.add(file.path)
  return next
}

/**
 * The graph after a fresh read of the newest page, keeping the older commits already shown below
 * it. Null when the newest page no longer meets them (history was rewritten), so the graph starts
 * over from the fresh page.
 */
export function withNewestPage(first, shown) {
  const lastFresh = first?.at(-1)?.hash
  const at = (shown ?? []).findIndex((commit) => commit.hash === lastFresh)
  if (at < 0) return null
  const seen = new Set(first.map((commit) => commit.hash))
  return [...first, ...shown.slice(at + 1).filter((commit) => !seen.has(commit.hash))]
}

/** The one push question: "Push 2 commits to origin/main?" (a first push names origin/<branch>). */
export function pushQuestion(status) {
  if (!status) return 'Push?'
  const target = status.upstream || `origin/${status.branch}`
  const ahead = status.ahead || 0
  if (!status.upstream) return `Push ${status.branch} to ${target} for the first time?`
  return `Push ${ahead} commit${ahead === 1 ? '' : 's'} to ${target}?`
}

/** Push has something to do: a branch with no upstream yet, or commits ahead of it. */
export function canPush(status) {
  if (!status || !status.branch || status.branch === 'HEAD') return false
  return !status.upstream || status.ahead > 0
}

/** git's own words when the folder is not a repository ("fatal: not a git repository ..."). */
export function isNotRepo(problem) {
  return /not a git repository/i.test(String(problem ?? ''))
}

/** Some commit in the graph carries an origin/... badge (the repo already has an origin). */
function hasOriginRef(commits) {
  return (commits ?? []).some((commit) => (commit.refs ?? []).some((ref) => /^origin\//.test(ref)))
}

/**
 * Which view the Source panel shows: 'init' (the folder is not a git repo: Start git here),
 * 'connect' (a repo with no origin: the Connect to GitHub card), 'normal' (everything else,
 * including still reading). `remote` is dev_git_remote's answer and `remoteRead` says it came
 * back; a remote it cannot read as GitHub's is still a remote when the branch tracks one or a
 * commit carries an origin/ badge, so those never ask to connect.
 */
export function sourceCard({ status, problem, remote, remoteRead, commits }) {
  if (!status && isNotRepo(problem)) return 'init'
  if (!status || !remoteRead || remote) return 'normal'
  if (status.upstream || hasOriginRef(commits)) return 'normal'
  return 'connect'
}
