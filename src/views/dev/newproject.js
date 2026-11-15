// The New project sheet's small decisions, plain and tested: the name a clone link gives, what a
// GitHub repo name may be, which GitHub line shows, and when the sheet may make the project.
// The Source panel's Connect to GitHub card uses the same name and GitHub rules.

// GitHub's own limit for a repository name.
const NAME_MAX = 100

/** The repo name a link names: "https://github.com/o/my-app.git" -> "my-app" (also git@host:o/r.git). */
export function nameFromLink(link) {
  const text = String(link ?? '').trim().replace(/[?#].*$/, '')
  const match = /[/:]([^/:\s]+?)(?:\.git)?\/*$/.exec(text)
  return match ? match[1] : ''
}

/** A GitHub repo name: letters, digits, "-", "_" and ".", not starting with ".", at most 100. */
export function validRepoName(name) {
  const text = String(name ?? '')
  return text.length > 0 && text.length <= NAME_MAX && /^[A-Za-z0-9_.-]+$/.test(text) && !text.startsWith('.')
}

/** Any text made into a valid repo name: "My App (old)" -> "My-App-old"; '' when nothing is left. */
export function cleanRepoName(text) {
  return String(text ?? '')
    .trim()
    .replace(/[^A-Za-z0-9_.-]+/g, '-')
    .replace(/-{2,}/g, '-')
    .replace(/^[.-]+/, '')
    .replace(/-+$/, '')
    .slice(0, NAME_MAX)
}

/**
 * Which GitHub line shows for dev_github's answer: 'asking' (not read yet), 'install' (no GitHub
 * CLI), 'signin' (installed, signed out) or 'ready'.
 */
export function githubLine(github) {
  if (!github) return 'asking'
  if (!github.installed) return 'install'
  if (!github.signedIn) return 'signin'
  return 'ready'
}

/** Where the new project lands, as the sheet shows it: <root>\<kind>\<name> (no kind: <root>\<name>). */
export function landingFolder(root, kind, name) {
  if (!root) return ''
  const base = String(root).replace(/[\\/]+$/, '')
  return [base, kind, String(name ?? '').trim() || '<name>'].filter(Boolean).join('\\')
}

/**
 * The sheet may make the project: a projects folder is set, nothing is running, the name is a
 * valid repo name, and then a link (clone) or a signed-in owner (create).
 */
export function canSubmit({ mode, root, link, name, owner, github, working }) {
  if (working || !root || !validRepoName(String(name ?? '').trim())) return false
  if (mode === 'link') return Boolean(String(link ?? '').trim())
  if (mode === 'create') return githubLine(github) === 'ready' && Boolean(owner)
  return false
}

// ---------- the stack (optional) ----------
// What a member may ask their companion to set up in a new project, one table so it grows. The
// member's companion is a plain companion (not Kiyo's Home), so what it is told is a plain sentence.
export const OTHER = '__other__'
export const STACK = [
  { id: 'frontend', label: 'Frontend', role: 'frontend', options: ['Vue.js', 'Nuxt', 'React', 'Next.js', 'Angular', 'Svelte / SvelteKit', 'plain HTML + JS'] },
  { id: 'backend', label: 'Backend', role: 'backend', options: ['Laravel', 'Django', 'FastAPI', 'Flask', 'Node (Express)', 'NestJS', 'Java Spring Boot', 'ASP.NET Core', 'Go', 'Ruby on Rails'] },
  { id: 'database', label: 'Database', role: 'database', options: ['MySQL', 'PostgreSQL', 'SQLite', 'MongoDB'] },
  { id: 'hosting', label: 'Hosting', role: 'hosting', options: ['DO App Platform', 'Hostinger', 'Vercel', 'Netlify', 'Render', 'Railway', 'AWS', 'a VPS of their own'] },
  { id: 'mobile', label: 'Mobile', role: 'mobile app', options: ['Flutter', 'React Native', 'Android (Kotlin)'] },
]

/** One choice as the sentence will say it: the pick, the member's own words for Other..., or ''. */
export function stackChoice(pick, other = '') {
  if (pick === OTHER) return String(other ?? '').replace(/\s+/g, ' ').trim().slice(0, 60).trim()
  return pick ? String(pick) : ''
}

/** The chosen parts in table order: [{ id, role, name }]. */
function chosen(picks = {}, others = {}) {
  return STACK.map((s) => ({ id: s.id, role: s.role, name: stackChoice(picks[s.id], others[s.id]) })).filter((c) => c.name)
}

/** "Next.js + NestJS + PostgreSQL on Vercel" (hosting after "on"), or '' when nothing is chosen. */
export function stackPreview(picks = {}, others = {}) {
  const parts = chosen(picks, others)
  const hosting = parts.find((c) => c.id === 'hosting')
  const rest = parts.filter((c) => c.id !== 'hosting').map((c) => c.name).join(' + ')
  if (!rest && !hosting) return ''
  if (!rest) return `hosted on ${hosting.name}`
  return hosting ? `${rest} on ${hosting.name}` : rest
}

/** The one sentence typed into the new project's session, or '' when nothing is chosen. */
export function setupSentence(picks = {}, others = {}) {
  const parts = chosen(picks, others)
  if (!parts.length) return ''
  const hosting = parts.find((c) => c.id === 'hosting')
  const built = parts.filter((c) => c.id !== 'hosting').map((c) => `${c.name} ${c.role}`)
  const what = built.length ? `Set up a new project: ${built.join(', ')}${hosting ? `, deployed on ${hosting.name}` : ''}.` : `Set up a new project, deployed on ${hosting.name}.`
  const steps = hosting ? 'set up the environment, scaffold the project, then deployment' : 'set up the environment, then scaffold the project'
  return `${what} Start with a plan: ${steps}. Ask me before anything costs money or goes online.`
}
