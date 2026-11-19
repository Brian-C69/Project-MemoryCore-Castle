// STUDIO's workflows, the pure parts: which blocks the form draws, what a run sends, and what its
// buttons say. The Rust core checks and runs the file (src-tauri/src/workflows.rs); this only shapes
// the page.

export const ICONS = {
  picture: '🖼',
  people: '👥',
  person: '👤',
  poster: '📜',
  star: '★',
  heart: '♥',
  spark: '✦',
  brush: '🖌',
}
export const iconOf = (workflow) => ICONS[workflow?.icon] ?? ICONS.spark

const INPUTS = ['picture', 'character', 'text', 'chips']

const blocksOf = (workflow) => (Array.isArray(workflow?.blocks) ? workflow.blocks : [])
export const createOf = (workflow) => blocksOf(workflow).find((b) => b.type === 'create') ?? null

/** The blocks the form draws, in the file's order: every input and every Write-prompt button. */
export function fieldsOf(workflow) {
  return blocksOf(workflow)
    .filter((b) => INPUTS.includes(b.type) || b.type === 'describe')
    .map((b) => ({ ...b, label: (b.label ?? '').trim() || b.id }))
}

/** A fresh set of answers: empty words and pictures, each choice on its first option. */
export function emptyValues(workflow) {
  const values = {}
  for (const b of blocksOf(workflow)) {
    if (!INPUTS.includes(b.type)) continue
    values[b.id] = { text: b.type === 'chips' ? (b.options?.[0] ?? '') : '', name: '', picture: '' }
  }
  return values
}

/** The `{{names}}` the picture step's prompt uses, as block ids. */
export function usedIds(workflow) {
  const template = createOf(workflow)?.prompt ?? ''
  return [...template.matchAll(/\{\{\s*([^}]+?)\s*\}\}/g)].map((m) => m[1].split('.')[0])
}

/** The first thing still to fill before the picture can be made, in the member's words; '' when ready. */
export function missing(workflow, values) {
  const create = createOf(workflow)
  if (!create) return 'This workflow has no picture step.'
  const byId = Object.fromEntries(blocksOf(workflow).map((b) => [b.id, b]))
  const label = (id) => (byId[id]?.label ?? '').trim() || id
  for (const id of usedIds(workflow)) {
    if (byId[id]?.type === 'text' && !values[id]?.text?.trim()) return `Fill in ${label(id)}.`
  }
  for (const id of create.references ?? []) {
    if (!values[id]?.picture) return `Add a picture to ${label(id)}.`
  }
  return ''
}

/**
 * The picture button: { kind, label }. kind is 'checking' (the sign-in check is still answering),
 * 'key' (no way to draw: the button opens Keys), 'busy', 'missing' (something to fill first) or 'ready'.
 * `way` is the Pictures power: 'ready' (the key), 'builtin' (Codex), 'prompt' (none).
 */
export function createAction(workflow, values, way, loaded = true, busy = false) {
  if (!loaded) return { kind: 'checking', label: 'Checking…' }
  if (way !== 'ready' && way !== 'builtin') return { kind: 'key', label: 'Add the OpenAI key' }
  if (busy) return { kind: 'busy', label: 'Working…' }
  const why = missing(workflow, values)
  return why ? { kind: 'missing', label: 'Create picture', why } : { kind: 'ready', label: 'Create picture' }
}

/** Write prompt from picture: ready when a way is open and its picture is chosen. */
export function describeAction(block, values, way, busy = false) {
  if (way !== 'ready' && way !== 'builtin') return { kind: 'key', label: block.label }
  if (busy) return { kind: 'busy', label: 'Reading the picture…' }
  if (!values[block.from]?.picture) return { kind: 'missing', label: block.label, why: 'Upload the picture first.' }
  return { kind: 'ready', label: block.label }
}

/** What a run sends: each block's words trimmed, empty parts left out. */
export function runValues(values) {
  const out = {}
  for (const [id, v] of Object.entries(values ?? {})) {
    const part = {}
    if (v.text?.trim()) part.text = v.text.trim()
    if (v.name?.trim()) part.name = v.name.trim()
    if (v.picture) part.picture = v.picture
    out[id] = part
  }
  return out
}
