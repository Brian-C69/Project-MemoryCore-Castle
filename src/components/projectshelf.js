// The PROJECTS room's shelf moves: from each shelf, what a project can do next. `ask` asks first
// (a pause's reason, a delete's yes); `to` is the shelf it goes to.
export const MOVES = {
  active: [
    { label: 'Pause', ask: 'pause', to: 'paused' },
    { label: 'Archive', to: 'archived' },
    { label: 'Delete…', ask: 'delete', to: 'deleted' },
  ],
  paused: [
    { label: 'Resume', to: 'active' },
    { label: 'Archive', to: 'archived' },
    { label: 'Delete…', ask: 'delete', to: 'deleted' },
  ],
  archived: [
    { label: 'Back to active', to: 'active' },
    { label: 'Delete…', ask: 'delete', to: 'deleted' },
  ],
  deleted: [{ label: 'Restore', to: 'active' }],
}

// The words on the project's chip.
export function stateLine(state) {
  switch (state?.shelf) {
    case 'paused':
      return 'Paused'
    case 'archived':
      return 'Archived'
    case 'deleted': {
      const left = state.daysLeft ?? 0
      return left > 1 ? `Deleted · gone for good in ${left} days` : left === 1 ? 'Deleted · gone for good tomorrow' : 'Deleted · goes today'
    }
    default:
      return 'Active'
  }
}

// What a move of a project with sub-projects asks first: null when there is nothing to ask
// (no sub-projects beside it on its shelf, or the move already asks its own question first).
export function kidsQuestion(state, to) {
  const n = state?.kidsBeside ?? 0
  if (!n || to === state.shelf) return null
  const verb = { paused: 'pause', archived: 'archive', deleted: 'delete', active: state.shelf === 'deleted' ? 'restore' : 'resume' }[to]
  const kids = n === 1 ? 'its sub-project' : `its ${n} sub-projects`
  return { verb, question: `Also ${verb} ${kids}?`, all: `${cap(verb)} all of them`, one: 'Only the parent' }
}

function cap(word) {
  return word.charAt(0).toUpperCase() + word.slice(1)
}

// "3 of 8 tasks done · 38%", or '' with no tasks.
export function progressLine(done, total) {
  if (!total) return ''
  return `${done} of ${total} tasks done · ${Math.round((done / total) * 100)}%`
}

// The PROJECTS list as rows to draw: each shelf's pages in tree order (the store sends a parent,
// then its sub-projects), with sub-projects hidden under a closed parent. A search keeps a
// parent whose sub-project matches, and opens it. `closed` holds the files of closed parents.
export function treeRows(shelves, query = '', closed = new Set()) {
  const q = query.trim().toLowerCase()
  const hit = (p) => p.title.toLowerCase().includes(q) || (p.date ?? '').includes(q)
  return shelves
    .map((shelf) => {
      const rows = []
      for (const page of shelf.pages) {
        const parent = page.tree?.parent
        const inTree = parent && shelf.pages.some((p) => p.file === parent)
        if (inTree) continue
        const kids = shelf.pages.filter((p) => p.tree?.parent === page.file)
        const kidHits = q ? kids.filter(hit) : kids
        if (q && !hit(page) && !kidHits.length) continue
        const open = q ? true : !closed.has(page.file)
        rows.push({ page, depth: 0, kids: kids.length, open })
        if (open) for (const k of q && !hit(page) ? kidHits : kids) rows.push({ page: k, depth: 1, kids: 0, open: false })
      }
      return { name: shelf.name, rows }
    })
    .filter((s) => s.rows.length)
}

// A task list after one task is dragged from index `from` to index `to`: the new order of ids.
export function reorder(ids, from, to) {
  if (from === to || from < 0 || to < 0 || from >= ids.length || to >= ids.length) return ids.slice()
  const out = ids.slice()
  const [moved] = out.splice(from, 1)
  out.splice(to, 0, moved)
  return out
}
