// The Sentinel's page logic, kept pure so it is tested: the shield's look from the status, and
// which buttons each finding shows.

export const STALE_MINUTES = 120
export const MAC_LINE = 'The Sentinel keeps watch on Windows for now.'

// -> { look: 'away' | 'unset' | 'quiet' | 'look' | 'alarm', line, stale }
export function shieldLook(status) {
  if (!status) return { look: 'unset', line: 'The Sentinel is waking up.', stale: false }
  if (!status.supported) return { look: 'away', line: MAC_LINE, stale: false }
  if (!status.on) return { look: 'unset', line: 'The Sentinel is off. Turn it on in Settings > Sentinel.', stale: false }
  if (!status.installed) return { look: 'unset', line: 'Run the set-up line in Settings > Sentinel to let it keep watch.', stale: false }
  const patrol = status.patrol
  const stale = !patrol || patrol.minutesAgo > STALE_MINUTES
  const open = status.open || { alarm: 0, look: 0, note: 0 }
  let look = 'quiet'
  if (status.unread > 0 || open.alarm > 0 || patrol?.status === 'alarm') look = 'alarm'
  else if (!status.current || patrol?.status === 'look' || open.look > 0) look = 'look'
  let line = patrol?.line || 'No patrol yet. It runs within the hour, or press Patrol now.'
  if (!status.current) line = status.setupLine
  else if (status.problem) line = status.problem
  if (status.patrolling) line = 'On patrol now…'
  return { look, line, stale }
}

// The buttons a finding shows: Fix (only with a move), This is normal, Undo (only once fixed by a move).
export function findingButtons(finding) {
  if (!finding) return []
  if (finding.state === 'open') {
    const out = []
    if (finding.fixMove) out.push('fix')
    out.push('accept')
    return out
  }
  if (finding.state === 'fixed' && finding.undoable) return ['undo']
  return []
}

// Words for a move, as the member reads them.
const MOVES = {
  'firewall-on': 'Turn the firewall on',
  'protection-on': 'Turn protection on',
  'scan-quick': 'Run a quick scan',
  'signatures-update': 'Update the antivirus',
  'block-address': 'Block this address for a day',
  'block-port': 'Block this port',
  'disable-task': 'Switch this task off',
  'disable-startup': 'Stop it starting with Windows',
  'stop-process': 'Stop this program',
}
export const moveLabel = (move) => MOVES[move] || 'Fix'
