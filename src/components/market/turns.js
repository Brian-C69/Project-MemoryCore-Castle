// The companion's turns as the room shows them: chat lines, action chips and lesson rows. Pure.

/** A turn's actions as saved ({action, result, ok}[]), or [] when it has none. */
export function actionsOf(turn) {
  try {
    const list = JSON.parse(turn?.actionsJson ?? '[]')
    return Array.isArray(list) ? list : []
  } catch {
    return []
  }
}

/** One action as a chip: "Buy ETHUSDT 0.05", "Sell BTCUSDT 0.01 @ 68,000", "Close SOLUSDT". */
export function chipOf(entry) {
  const a = entry?.action ?? {}
  if (a.type === 'close_position') return `Close ${a.symbol}`
  const verb = a.side === 'SHORT' ? 'Sell' : 'Buy'
  const at = a.orderType === 'LIMIT' && a.price ? ` @ ${Number(a.price).toLocaleString('en-US')}` : ''
  return `${verb} ${a.symbol} ${a.qty}${at}`
}

/** 'buy' | 'sell' | 'close', for the chip's colour. */
export function chipKind(entry) {
  const a = entry?.action ?? {}
  if (a.type === 'close_position') return 'close'
  return a.side === 'SHORT' ? 'sell' : 'buy'
}

/** The tag on a trade-log card. */
export const tagOf = (turn) => (turn?.triggerKind === 'AUTO' ? 'On its own' : 'You asked')

/** The chat panel's lines, oldest first, from the newest turns: the member's words, then the reply. */
export function chatLines(turns, max = 20) {
  const lines = []
  for (const t of [...(turns ?? [])].reverse()) {
    if (t.message) lines.push({ id: `${t.id}-m`, who: 'member', text: t.message, at: t.createdAt })
    if (t.reply) lines.push({ id: `${t.id}-r`, who: 'companion', text: t.reply, at: t.createdAt, failed: t.status === 'FAILED' })
  }
  return lines.slice(-max)
}

/** The newest action that filled, for the LAST TRADE card. */
export function lastTrade(turns) {
  for (const t of turns ?? []) {
    const done = actionsOf(t).find((e) => e.ok)
    if (done) return { chip: chipOf(done), kind: chipKind(done), result: done.result, at: t.createdAt }
  }
  return null
}

/** "+1.80%" / "-0.90%". */
export function percent(n) {
  const v = Number(n) || 0
  return `${v >= 0 ? '+' : ''}${v.toFixed(2)}%`
}

/** A turn's state in words when it did not finish: null for a finished turn. */
export function unfinishedOf(turn) {
  if (turn?.status === 'DONE') return null
  if (turn?.status === 'RUNNING') return 'Thinking…'
  if (turn?.status === 'STOPPED') return 'Stopped'
  return 'Did not finish'
}

/** The Trade log's counters for the turns shown: turns, actions that went through, actions refused, and turns with no trade. */
export function turnCounts(turns) {
  const counts = { turns: 0, done: 0, refused: 0, noTrade: 0 }
  for (const t of turns ?? []) {
    counts.turns++
    const actions = actionsOf(t)
    if (!actions.length && t.status === 'DONE') counts.noTrade++
    for (const a of actions) a.ok ? counts.done++ : counts.refused++
  }
  return counts
}
