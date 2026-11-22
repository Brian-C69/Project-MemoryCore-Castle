// CALENDAR phase 2: weeks, member events, repeats and dragging. Pure, like calendar.js.
import { addDays, dayTitle, iso, parseIso } from './calendar.js'

const pad = (n) => String(n).padStart(2, '0')
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

export function weekOf(text, weekStart = 1) {
  const date = parseIso(text) || new Date()
  const back = (date.getDay() - weekStart + 7) % 7
  const first = addDays(iso(date), -back)
  const days = Array.from({ length: 7 }, (_, i) => addDays(first, i))
  return { from: days[0], to: days[6], days }
}

export function weekTitle(days) {
  const a = parseIso(days[0])
  const b = parseIso(days[6])
  const left = a.getMonth() === b.getMonth() ? `${a.getDate()}` : `${a.getDate()} ${MONTHS[a.getMonth()]}`
  return `${left} - ${b.getDate()} ${MONTHS[b.getMonth()]} ${b.getFullYear()}`
}

export function minutes(time) {
  const m = /^(\d{1,2}):(\d{2})$/.exec(time || '')
  return m ? Number(m[1]) * 60 + Number(m[2]) : null
}

export function clock(total) {
  const t = Math.max(0, Math.min(24 * 60 - 1, Math.round(total)))
  return `${pad(Math.floor(t / 60))}:${pad(t % 60)}`
}

// Timed events of one day become blocks: top and height in minutes, side by side when they overlap.
export function layoutDay(items) {
  const blocks = (items || [])
    .filter((i) => i.kind === 'event' && !i.allDay && minutes(i.time) !== null)
    .map((item) => {
      const top = minutes(item.time)
      const end = minutes(item.endTime)
      return { item, top, height: Math.max(30, end !== null && end > top ? end - top : 60) }
    })
    .sort((a, b) => a.top - b.top || b.height - a.height)
  const lanesEnd = []
  for (const block of blocks) {
    let lane = lanesEnd.findIndex((end) => end <= block.top)
    if (lane < 0) lane = lanesEnd.length
    lanesEnd[lane] = block.top + block.height
    block.lane = lane
  }
  // every block shares the widest set of lanes among the blocks it overlaps
  for (const block of blocks) {
    let most = block.lane + 1
    for (const other of blocks) if (other.top < block.top + block.height && block.top < other.top + other.height) most = Math.max(most, other.lane + 1)
    block.lanes = most
  }
  return blocks
}

export function colourVar(name) {
  if (name === 'soft') return 'color-mix(in srgb, var(--accent) 55%, var(--text))'
  if (name === 'plain') return 'var(--text)'
  return 'var(--accent)'
}
export const COLOURS = [{ value: 'accent', label: 'Accent' }, { value: 'soft', label: 'Soft' }, { value: 'plain', label: 'Plain' }]

const DAY_LABELS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
export const WEEKDAY_CHIPS = DAY_LABELS.map((label, i) => ({ value: i + 1, label }))

export function repeatText(rule) {
  if (!rule) return ''
  const unit = rule.every
  const every = (rule.interval || 1) > 1 ? `every ${rule.interval} ${unit}s` : `every ${unit}`
  const days = unit === 'week' && rule.weekdays?.length ? ` on ${rule.weekdays.map((d) => DAY_LABELS[d - 1]).filter(Boolean).join(', ')}` : ''
  const end = rule.until ? `, until ${rule.until}` : rule.count ? `, ${rule.count} times` : ''
  return `Repeats ${every}${days}${end}`
}

export function emptyForm(date, time = '') {
  return { id: 0, title: '', note: '', date, time, allDay: !time, endDate: '', endTime: '', colour: 'accent', repeatOn: false, every: 'week', interval: 1, weekdays: [], endsKind: 'never', until: '', count: 10, remindBeforeMin: null, automationJobId: null, automationLeadMin: 0 }
}

export function formFromQuick(q, fallbackDate) {
  const f = emptyForm(q.date || fallbackDate, q.time || '')
  f.title = q.title || ''
  f.remindBeforeMin = q.remindMin ?? null
  if (q.repeat) {
    f.repeatOn = true
    f.every = q.repeat.every
    f.interval = q.repeat.interval || 1
    f.weekdays = q.repeat.weekdays || []
  }
  return f
}

export function toEvent(f) {
  const allDay = Boolean(f.allDay)
  const time = allDay ? '00:00' : f.time || '09:00'
  const event = { id: f.id || 0, title: f.title, note: f.note || '', startsAt: `${f.date} ${time}`, allDay, colour: f.colour || 'accent', remindBeforeMin: f.remindBeforeMin ?? null, automationJobId: f.automationJobId || null, automationLeadMin: f.automationJobId ? Number(f.automationLeadMin) || 0 : null }
  if (allDay) event.endsAt = f.endDate && f.endDate !== f.date ? `${f.endDate} 00:00` : null
  else event.endsAt = f.endTime ? `${f.endDate || f.date} ${f.endTime}` : null
  event.repeat = f.repeatOn
    ? {
        every: f.every,
        interval: Number(f.interval) || 1,
        weekdays: f.every === 'week' ? [...(f.weekdays || [])].sort() : [],
        until: f.endsKind === 'until' && f.until ? f.until : null,
        count: f.endsKind === 'count' ? Number(f.count) || 1 : null,
      }
    : null
  return event
}

export function toForm(e) {
  const [date, time] = (e.startsAt || '').split(' ')
  const f = emptyForm(date, e.allDay ? '' : time)
  f.id = e.id
  f.title = e.title
  f.note = e.note || ''
  f.allDay = Boolean(e.allDay)
  f.colour = e.colour || 'accent'
  f.remindBeforeMin = e.remindBeforeMin ?? null
  f.automationJobId = e.automationJobId ?? null
  f.automationLeadMin = e.automationLeadMin ?? 0
  f.state = e.state || 'active'
  f.createdBy = e.createdBy || 'member'
  if (e.endsAt) {
    const [endDate, endTime] = e.endsAt.split(' ')
    f.endDate = endDate === date ? '' : endDate
    f.endTime = e.allDay ? '' : endTime
  }
  if (e.repeat) {
    f.repeatOn = true
    f.every = e.repeat.every
    f.interval = e.repeat.interval || 1
    f.weekdays = e.repeat.weekdays || []
    if (e.repeat.until) {
      f.endsKind = 'until'
      f.until = e.repeat.until
    } else if (e.repeat.count) {
      f.endsKind = 'count'
      f.count = e.repeat.count
    }
  }
  return f
}

// Where a dragged event lands: the new day keeping its time of day, or the time it was dropped on.
export function movedStart(item, date, time) {
  const at = time ?? (item.allDay ? '00:00' : item.time || '00:00')
  return `${date} ${at}`
}

// Only an event's first piece can be dragged, and only events.
export function canDrag(item) {
  return item.kind === 'event' && item.first !== false && item.state !== 'proposed'
}

// One line for a suggestion waiting for a yes: what, when, and how often.
export function proposalLine(e) {
  const [date, time] = (e.startsAt || '').split(' ')
  const when = `${dayTitle(date)}${e.allDay ? '' : ` ${time}`}`
  const again = e.repeat ? ` - ${repeatText(e.repeat).toLowerCase()}` : ''
  return `${e.title} - ${when}${again}`
}

// ---- phase 3: reminders ----

export function leadText(min) {
  const plural = (n, unit) => (n === 1 ? `1 ${unit}` : `${n} ${unit}s`)
  if (min >= 10080 && min % 10080 === 0) return plural(min / 10080, 'week')
  if (min >= 1440 && min % 1440 === 0) return plural(min / 1440, 'day')
  if (min >= 60 && min % 60 === 0) return plural(min / 60, 'hour')
  return plural(min, 'minute')
}

const LEADS = [0, 5, 10, 15, 30, 60, 120, 1440, 2880, 10080]

// The "Remind me" choices as strings (a GlassSelect value): 'none', or the minutes before. A lead that
// came from quick add ("remind 45m") and is not in the list is added so it can be seen and kept.
export function remindOptions(current) {
  const leads = current != null && !LEADS.includes(current) ? [...LEADS, current].sort((a, b) => a - b) : LEADS
  return [{ value: 'none', label: 'No reminder' }, ...leads.map((m) => ({ value: String(m), label: m === 0 ? 'At the time' : `${leadText(m)} before` }))]
}
export const remindValue = (min) => (min == null ? 'none' : String(min))
export const remindMinutes = (value) => (value === 'none' || value == null ? null : Number(value))

// ---- phase 5: an event that starts an automation ----

// The jobs an event may start, as GlassSelect options ('none' or the job's id as text).
export function jobOptions(jobs, current) {
  const list = [...(jobs || [])]
  // a job that is no longer eligible (switched off the trigger, archived) stays visible so it can be seen and replaced
  if (current != null && !list.some((j) => j.id === current)) list.push({ id: current, name: 'An automation that no longer waits for calendar events', enabled: false })
  return [{ value: 'none', label: 'No automation' }, ...list.map((j) => ({ value: String(j.id), label: j.name + (j.enabled ? '' : ' (switched off)') }))]
}
export const jobValue = (id) => (id == null ? 'none' : String(id))
export const jobId = (value) => (value === 'none' || value == null ? null : Number(value))

// When it starts: the same choices as a reminder, without "No reminder".
export const linkLeadOptions = (current) => remindOptions(current).slice(1)
