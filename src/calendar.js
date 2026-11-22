// CALENDAR: pure helpers (no Vue, no Tauri) so the grid and the lists can be tested.
// Dates travel as 'YYYY-MM-DD' strings in local time; they are never turned into UTC.

export const KINDS = [
  { id: 'holiday', label: 'Holidays' },
  { id: 'event', label: 'Events' },
  { id: 'reminder', label: 'Reminders' },
  { id: 'task', label: 'Tasks' },
  { id: 'automation', label: 'Automations' },
  { id: 'diary', label: 'Diary' },
]

export const SHOWN_KINDS = KINDS

const pad = (n) => String(n).padStart(2, '0')

export function iso(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

export function parseIso(text) {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(text || '')
  if (!m) return null
  const date = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]))
  return iso(date) === text ? date : null
}

export function addDays(text, n) {
  const date = parseIso(text)
  if (!date) return text
  date.setDate(date.getDate() + n)
  return iso(date)
}

// weekStart: 1 = Monday, 0 = Sunday.
export function weekdayNames(weekStart = 1) {
  const names = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
  return [...names.slice(weekStart), ...names.slice(0, weekStart)]
}

// The weeks a month page shows: every week that touches the month, seven cells each.
export function monthGrid(year, month, weekStart = 1) {
  const first = new Date(year, month, 1)
  const offset = (first.getDay() - weekStart + 7) % 7
  const last = new Date(year, month + 1, 0).getDate()
  const weeks = Math.ceil((offset + last) / 7)
  const out = []
  for (let w = 0; w < weeks; w++) {
    const row = []
    for (let d = 0; d < 7; d++) {
      const date = new Date(year, month, 1 - offset + w * 7 + d)
      row.push({ date: iso(date), day: date.getDate(), inMonth: date.getMonth() === month })
    }
    out.push(row)
  }
  return out
}

export function gridRange(weeks) {
  return { from: weeks[0][0].date, to: weeks[weeks.length - 1][6].date }
}

export function monthTitle(year, month) {
  const names = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']
  return `${names[month]} ${year}`
}

export function shiftMonth(year, month, by) {
  const date = new Date(year, month + by, 1)
  return { year: date.getFullYear(), month: date.getMonth() }
}

export function filterItems(items, on) {
  return (items || []).filter((i) => on?.[i.kind] !== false)
}

export function groupByDay(items) {
  const map = {}
  for (const item of items || []) (map[item.date] ||= []).push(item)
  return map
}

// A month cell shows up to `max` items and a "+n more" for the rest.
export function cellItems(items, max = 3) {
  const list = items || []
  if (list.length <= max) return { shown: list, more: 0 }
  return { shown: list.slice(0, max - 1), more: list.length - (max - 1) }
}

// The agenda: the days that have something, from `from` for `days` days, in order.
export function agendaDays(items, from, days = 30) {
  const by = groupByDay(items)
  const to = addDays(from, days - 1)
  return Object.keys(by)
    .filter((d) => d >= from && d <= to)
    .sort()
    .map((date) => ({ date, items: by[date] }))
}

export function dayTitle(text) {
  const date = parseIso(text)
  if (!date) return text
  const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
  return `${days[date.getDay()]} ${date.getDate()} ${months[date.getMonth()]}`
}

export function itemLine(item) {
  return item.time ? `${item.time} ${item.title}` : item.title
}

// The small mark in front of an item: what it is, and how it went.
export function itemMark(item) {
  if (item.kind === 'automation') return item.state === 'failed' ? '!' : item.state === 'planned' ? '>' : '~'
  if (item.kind === 'task') return item.state === 'done' ? 'x' : '-'
  if (item.kind === 'reminder') return item.state === 'done' ? 'x' : '*'
  if (item.kind === 'diary') return 'o'
  if (item.kind === 'holiday') return '#'
  if (item.state === 'proposed') return '?'
  return '+'
}

// Where an item opens: the room that owns it.
export function ownerRoom(item) {
  return ['home', 'projects', 'automations', 'diary'].includes(item.owner) ? item.owner : null
}

// "3 hours ago" for a local 'YYYY-MM-DD HH:MM' stamp.
export function agoText(stamp, now = new Date()) {
  const m = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})/.exec(stamp || '')
  if (!m) return 'not yet'
  const then = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]))
  const minutes = Math.max(0, Math.round((now - then) / 60000))
  const plural = (n, unit) => `${n} ${unit}${n === 1 ? '' : 's'} ago`
  if (minutes < 1) return 'just now'
  if (minutes < 60) return plural(minutes, 'minute')
  if (minutes < 1440) return plural(Math.floor(minutes / 60), 'hour')
  return plural(Math.floor(minutes / 1440), 'day')
}
