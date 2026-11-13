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
