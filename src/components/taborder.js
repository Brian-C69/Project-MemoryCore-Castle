// The top bar's tabs in the member's own order (Settings `tabOrder`, a list of tab ids).

// The tabs in the saved order. A tab the order does not name (a room added in a later release)
// keeps Castle's own order after the named ones; an id that is no tab any more is passed over.
export function orderTabs(tabs, order = []) {
  const rank = new Map((order ?? []).map((id, i) => [id, i]))
  const named = tabs.filter((t) => rank.has(t.id)).sort((a, b) => rank.get(a.id) - rank.get(b.id))
  return [...named, ...tabs.filter((t) => !rank.has(t.id))]
}

// The ids with `from` moved onto `to`: before it when moving left, after it when moving right -
// the place the dragged tab was dropped on. The same list back when nothing moves.
export function moveTab(ids, from, to) {
  const i = ids.indexOf(from)
  const j = ids.indexOf(to)
  if (from === to || i < 0 || j < 0) return ids
  const next = ids.filter((id) => id !== from)
  const k = next.indexOf(to)
  next.splice(i < j ? k + 1 : k, 0, from)
  return next
}

// The visible neighbour a tab trades places with on Alt+Left / Alt+Right (null at either end).
export function neighbourOf(visibleIds, id, step) {
  const i = visibleIds.indexOf(id)
  if (i < 0) return null
  return visibleIds[i + step] ?? null
}
