// FILES, its small decisions, pure and tested: size words, the rows the tree shows (opened
// folders, the filter), a file's language for the reader's label, the opened folders remembered
// per project while Castle runs, and a file's whole path for the chat.

/** 912 -> "912 B", 2150 -> "2.1 KB", 3_400_000 -> "3.2 MB"; nothing for a folder. */
export function sizeWords(bytes) {
  if (bytes === null || bytes === undefined) return ''
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

const LANGUAGES = {
  js: 'JavaScript', mjs: 'JavaScript', cjs: 'JavaScript', ts: 'TypeScript', vue: 'Vue', jsx: 'JSX', tsx: 'TSX',
  php: 'PHP', java: 'Java', rs: 'Rust', py: 'Python', go: 'Go', rb: 'Ruby', cs: 'C#', kt: 'Kotlin',
  css: 'CSS', scss: 'SCSS', html: 'HTML', json: 'JSON', md: 'Markdown', yml: 'YAML', yaml: 'YAML',
  toml: 'TOML', xml: 'XML', sql: 'SQL', sh: 'Shell', ps1: 'PowerShell', txt: 'Text',
}

/** "app.js" -> "JavaScript"; unknown or no extension -> "". */
export function languageOf(name) {
  const lower = String(name ?? '').toLowerCase()
  const dot = lower.lastIndexOf('.')
  return dot > 0 ? (LANGUAGES[lower.slice(dot + 1)] ?? '') : ''
}

/**
 * The rows the tree shows, depth first: each opened folder's level under it. `levels` maps a
 * folder's rel ('' = the top) to its entries; `opened` holds the opened folders' rels. With a
 * filter, a row shows when its name matches or something under it (already read) does.
 */
export function visibleRows(levels, opened, filter = '') {
  const wanted = String(filter ?? '').trim().toLowerCase()
  const rows = []
  const walk = (rel, depth) => {
    let any = false
    for (const entry of levels[rel] ?? []) {
      const start = rows.length
      rows.push({ entry, depth })
      let under = false
      if (entry.dir && (opened.has(entry.rel) || wanted)) under = walk(entry.rel, depth + 1)
      const hit = !wanted || entry.name.toLowerCase().includes(wanted)
      if (!hit && !under) rows.splice(start) // nothing here or below matches: drop it and its rows
      else any = true
    }
    return any
  }
  walk('', 0)
  return rows
}

// The opened folders per project, kept while Castle runs.
const OPENED = new Map()

/** The opened set for a project folder (the same Set every time, so changes stay). */
export function openedFor(folder) {
  if (!OPENED.has(folder)) OPENED.set(folder, new Set())
  return OPENED.get(folder)
}

/** The project's file as a Windows path: folder + rel with backslashes. */
export function absPath(folder, rel) {
  return `${String(folder).replace(/[\\/]+$/, '')}\\${String(rel).replace(/^[\\/]+/, '').replaceAll('/', '\\')}`
}

/** A text file's lines for the reader (Windows line ends and the last empty line left out). */
export function textLines(text) {
  return String(text ?? '').replace(/\r/g, '').replace(/\n$/, '').split('\n')
}

/** The reader's foot: "2.1 KB · 40 lines · JavaScript". */
export function fileFacts(shown, name) {
  if (!shown) return ''
  const parts = [sizeWords(shown.size)]
  if (shown.kind === 'text') {
    const count = textLines(shown.text).length
    parts.push(`${count} ${count === 1 ? 'line' : 'lines'}`)
  }
  const language = languageOf(name)
  if (language) parts.push(language)
  return parts.filter(Boolean).join(' · ')
}
