export function runnerStatus(h, now = Date.now()) {
 if (!h) return { label: 'Checking runner', tone: 'muted', detail: 'Waiting for the backend heartbeat.' }
 let mode = h.mode
 if (!['off','paused','starting'].includes(mode) && h.lastHeartbeat != null && (now < h.lastHeartbeat || now - h.lastHeartbeat > 45000)) mode = 'stale'
 const states = {
  running: ['Runner active', 'ok', 'While this computer is awake, jobs keep running with Castle hidden, minimized or Windows locked.'],
  saving: ['Saving results', 'warn', `${h.pendingResults || 0} completed result(s) waiting to save. Actions are not repeated.`],
  paused: ['Paused', 'muted', 'New runs are paused. Missed schedules use their policy when resumed.'],
  off: ['Off', 'muted', 'Switch on the AUTOMATIONS room to start jobs.'],
  starting: ['Starting runner', 'muted', 'Waiting for the first successful check.'],
  stale: ['Runner not responding', 'bad', 'The last heartbeat is overdue. Check Castle before relying on new jobs.'],
  database_unavailable: ['Database unavailable', 'bad', 'Jobs wait for the memory database. Check Settings > Database.'],
  attention: ['Runner needs attention', 'bad', h.problem || 'Open the job history for details.'],
 }
 const [label, tone, detail] = states[mode] || states.starting
 return { label, tone, detail }
}
export function heartbeatTime(value) { return value == null ? 'Not yet' : new Date(value).toLocaleTimeString() }
