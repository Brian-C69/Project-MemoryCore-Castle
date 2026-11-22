export const EVENTS = [
  { value: 'backup_done', label: 'Memory backup finished' },
  { value: 'backup_failed', label: 'Memory backup failed' },
  { value: 'provider_switched', label: 'Companion provider switched' },
  { value: 'diary_saved', label: 'Diary entry saved' },
  { value: 'task_done', label: 'Project task ticked' },
  { value: 'reminder_due', label: 'Reminder due today' },
  { value: 'calendar_event', label: 'Calendar event reaches its time' },
]
export function eventTrigger() { return { event: 'backup_failed' } }
export function triggerSummary(t = {}, kind = '') {
  if (kind === 'event' || (t.event && !t.folder)) return `When: ${EVENTS.find(e => e.value === t.event)?.label || 'a Castle event'}`
  if (kind === 'webhook') return 'Web request to its private link'
  if (kind === 'file' || t.folder !== undefined) return `${t.event === 'changed' ? 'File appears or changes' : 'File appears'} in ${t.folder || 'a folder'} (${t.glob || '*'})`
  if (t.platform) return `${t.platform === 'any' ? 'Any approved chat' : ({telegram:'Telegram',discord:'Discord',whatsapp:'WhatsApp'})[t.platform] || t.platform}: ${({ contains: 'contains', starts_with: 'starts with', regex: 'matches regex' })[t.match] || 'matches'} “${t.text || ''}”`
  if (t.cron) return `Cron: ${t.cron}`
  if (t.every_minutes) return `Every ${t.every_minutes} minutes`
  return t.at ? `Once: ${t.at.replace('T', ' ')}` : 'Choose a schedule'
}
export function pill(status) { return ({ ok: 'OK', failed: 'Failed', running: 'Running', timed_out: 'Timed out', skipped: 'Skipped', cancelled: 'Cancelled', delivery_failed: 'Delivery failed', delivery_unknown: 'Delivery unconfirmed', delivery_sending: 'Sending', unknown: 'Unconfirmed', sending: 'Sending' })[status] || 'Never run' }
export function nextRun(value) { return value || 'Switched off' }
export function newJob() { return { name: '', note: '', enabled: false, trigger: { cron: '0 8 * * 1-5' }, action: { mode: 'think', prompt: 'What is open and due today, in five lines?\n{{work_log}}\n{{reminders}}\n{{today}}' }, delivery: [{ kind: 'home' }], policy: { timeout_s: 120, max_output_kb: 64, missed: 'skip', retries: 0, rate_per_hour: 6 } } }

export function commandFields(argsText, envText) {
  let args, env
  try { args = JSON.parse(argsText || '[]') } catch { throw new Error('Arguments must be a JSON list of text values.') }
  try { env = JSON.parse(envText || '{}') } catch { throw new Error('Environment must be a JSON object of text values.') }
  if (!Array.isArray(args) || args.some(v => typeof v !== 'string')) throw new Error('Each argument must be text.')
  if (!env || Array.isArray(env) || typeof env !== 'object' || Object.values(env).some(v => typeof v !== 'string')) throw new Error('Environment names and values must be text.')
  return { args, env }
}
export function actionSummary(action = {}) { return action.kind === 'http' ? 'HTTP: ' + (action.method || 'GET') + ' ' + (action.url || '') : action.kind === 'castle' ? (CASTLE_ACTIONS.find(a => a.value === action.action)?.label || 'Castle action') : action.kind === 'command' ? 'Command: ' + (action.program || '').split(/[\\/]/).pop() : 'Companion briefing' }

export const CASTLE_ACTIONS = [
  { value: 'check_database', label: 'Check database' },
  { value: 'backup_memory', label: 'Back up memory locally' },
  { value: 'export_markdown', label: 'Export to Markdown' },
  { value: 'add_reminder', label: 'Add reminder' },
  { value: 'add_task', label: 'Add project task' },
  { value: 'wake_companion', label: 'Wake companion' },
  { value: 'save_diary_prompt', label: 'Request diary save' },
  { value: 'notify', label: 'HOME notification' },
]
export const DESTINATIONS = [{ kind: 'home', label: 'HOME' }, { kind: 'telegram', label: 'Telegram' }, { kind: 'discord', label: 'Discord' }, { kind: 'whatsapp', label: 'WhatsApp' }]
export function castleAction(action = 'check_database') { return { kind: 'castle', action, title: '', due: '', project: '', text: '', folder: '', allow_tools: false } }
export function toggleDestination(list = [], kind, on) { const rest = list.filter(d => d.kind !== kind); return on ? [...rest, { kind }] : rest }
export function runStatus(run) { const statuses = (run.delivered || []).map(d => d.status); return statuses.includes('failed') ? 'delivery_failed' : statuses.includes('unknown') ? 'delivery_unknown' : statuses.includes('sending') ? 'delivery_sending' : run.status }

export function fileTrigger() { return { folder: '', glob: '*', event: 'created', settle_s: 5 } }
export function webhookTrigger() { return {} }
export function triggerDefaults(kind) { return kind === 'message' ? messageTrigger() : kind === 'file' ? fileTrigger() : kind === 'webhook' ? webhookTrigger() : kind === 'event' ? eventTrigger() : { cron: '0 8 * * 1-5' } }
export function listeningText(job) { const on = Boolean(job.enabled); const kind = job.triggerKind; if (kind === 'message') return on ? 'Listening for messages' : 'Switched off'; if (kind === 'file') return on ? 'Watching the folder' : 'Switched off'; if (kind === 'webhook') return on ? 'Waiting for requests' : 'Switched off'; if (kind === 'event') return on ? 'Waiting for the event' : 'Switched off'; return 'Next: ' + nextRun(job.nextRun) }
// One line of HOME's AUTOMATIONS card: the job, how it went, and its words without markdown marks.
export function noticeLine(n = {}) {
  const bad = ['failed', 'timed_out', 'delivery_failed'].includes(n.status)
  const text = String(n.summary || '').replace(/\*\*|__|`/g, '').replace(/\s+/g, ' ').trim()
  return { name: n.name || 'A job', state: pill(n.status), tone: bad ? 'bad' : 'ok', text }
}
export function messageTrigger() { return { platform: "any", match: "contains", text: "invoice", case: false, also_chat: false } }

export function requestBody(method, text) { if (method === 'GET') return null; try { return JSON.parse(text || 'null') } catch { throw new Error('The HTTP body must be valid JSON.') } }
export function savedTrigger(kind, trigger, scheduleType, value) { return ['message','file','event'].includes(kind) ? trigger : kind === 'webhook' ? {} : { [scheduleType]: scheduleType === 'every_minutes' ? Number(value) : value } }
