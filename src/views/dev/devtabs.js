// The DEV room's pure rules: which tab shows, how the project list reads, and how a plan's
// boxes are grouped and marked. No calls to the core here.

export const MAX_TABS = 8

// A folder compared the Windows way: either slash, no trailing slash, any case.
export function samePath(a, b) {
  const norm = (p) => String(p ?? '').replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase()
  return !!a && !!b && norm(a) === norm(b)
}

// The last part of a folder, for a tab with no name.
export function folderName(folder) {
  const parts = String(folder ?? '').split(/[\\/]+/).filter(Boolean)
  return parts[parts.length - 1] ?? ''
}

// The open tabs in slot order (0 first).
export function orderTabs(tabs) {
  return [...(tabs ?? [])].filter((t) => Number.isInteger(t?.slot)).sort((a, b) => a.slot - b.slot)
}

export function tabForSlot(tabs, slot) {
  return (tabs ?? []).find((t) => t.slot === slot) ?? null
}

export function tabForFolder(tabs, folder) {
  return (tabs ?? []).find((t) => samePath(t.folder, folder)) ?? null
}

// The recent projects, without the memory folder (it has its own session at HOME), each marked
// with the slot of its open tab (or null).
export function recentProjects(projects, tabs, memoryFolder) {
  return (projects ?? [])
    .filter((p) => p?.folder && !samePath(p.folder, memoryFolder))
    .map((p) => {
      const tab = tabForFolder(tabs, p.folder)
      return { folder: p.folder, name: p.name || folderName(p.folder), slot: tab ? tab.slot : null }
    })
}

// Which tab shows after the tabs changed: the one asked for (by folder), else the current one if it
// is still open, else its nearest neighbour (the next slot up, then the one below), else none.
export function nextActive(tabs, current, wantedFolder = null) {
  const list = orderTabs(tabs)
  if (wantedFolder) {
    const wanted = tabForFolder(list, wantedFolder)
    if (wanted) return wanted.slot
  }
  if (!list.length) return null
  if (current === null || current === undefined) return list[0].slot
  if (list.some((t) => t.slot === current)) return current
  const after = list.find((t) => t.slot > current)
  if (after) return after.slot
  return list[list.length - 1].slot
}

// What a session panel shows for a tab; a tab that is no longer open reads as asleep.
export function sessionView(tab) {
  if (!tab) return { running: false, status: { ready: false, message: 'This tab is closed.', fix: null } }
  return {
    running: !!tab.running,
    status: { ready: !!tab.status?.ready, message: tab.status?.message ?? '', fix: tab.status?.fix ?? null },
  }
}

// The chat room name of a DEV tab ('dev:3'), and the slot back from it (null for any other room).
export function devRoom(slot) {
  return `dev:${slot}`
}

export function devSlotOf(room) {
  const match = /^dev:([0-9]+)$/.exec(typeof room === 'string' ? room : '')
  if (!match) return null
  const slot = Number(match[1])
  return slot < MAX_TABS ? slot : null
}

// A message typed into a DEV session: one bracketed paste, so a new line never sends it early.
export function pasted(text) {
  return `\u001b[200~${text}\u001b[201~`
}

// ---------- Plans ----------

const CLOSED_PLAN = new Set(['done', 'closed', 'complete', 'completed', 'finished', 'archived', 'superseded'])

export function openPlans(plans) {
  return (plans ?? []).filter((p) => !CLOSED_PLAN.has(String(p?.status ?? '').toLowerCase()))
}

// A plan's boxes in order, cut into groups at each phase row: [{ heading, items }]. Boxes before the
// first phase row sit in a group with no heading.
export function groupByPhase(items) {
  const sorted = [...(items ?? [])].sort((a, b) => (a.ordinal ?? 0) - (b.ordinal ?? 0))
  const groups = []
  let group = null
  for (const item of sorted) {
    if (item.kind === 'phase') {
      group = { heading: item.text, state: item.state ?? 'open', items: [] }
      groups.push(group)
      continue
    }
    if (!group) {
      group = { heading: null, state: 'open', items: [] }
      groups.push(group)
    }
    group.items.push(item)
  }
  return groups.filter((g) => g.heading !== null || g.items.length)
}

const MARKS = {
  done: { mark: '✓', label: 'Done' },
  carried: { mark: '–', label: 'Carried to a later plan' },
  flagged: { mark: '⚑', label: 'Flagged' },
  open: { mark: '', label: 'Open' },
}

// The mark in an item's box, and what it means (for its tooltip).
export function itemMark(state) {
  return MARKS[state] ?? MARKS.open
}

// "3 / 7": the boxes done out of all boxes (phase rows are headings, not boxes).
export function planProgress(items) {
  const boxes = (items ?? []).filter((i) => i.kind !== 'phase')
  const done = boxes.filter((i) => i.state === 'done').length
  return boxes.length ? `${done} / ${boxes.length}` : ''
}
