// The DIARY room's three designs read the same shelves ([{ name: '2026-11' | 'Earlier', pages:
// [{ title, file, date }] }]). Everything here is pure, so it is tested without a window.

export const DIARY_LOOKS = [
  { id: 'list', label: 'List', line: 'Every page by month. Best for finding one by its title.' },
  { id: 'calendar', label: 'Calendar', line: 'A month at a glance. Days with a page are marked.' },
  { id: 'book', label: 'Book', line: 'Two pages side by side, read in order like a journal.' },
]

export function lookOf(value) {
  return DIARY_LOOKS.some((l) => l.id === value) ? value : 'list'
}

const isMonth = (name) => /^\d{4}-\d{2}$/.test(name)
const isDate = (value) => /^\d{4}-\d{2}-\d{2}$/.test(value ?? '')

// Every page with a date, and those without.
function split(shelves) {
  const dated = []
  const undated = []
  for (const shelf of shelves ?? []) {
    for (const page of shelf.pages ?? []) (isDate(page.date) ? dated : undated).push(page)
  }
  return { dated, undated }
}

// The months that hold a page, oldest first: ['2026-09', '2026-10', '2026-11'].
export function monthsOf(shelves) {
  const months = new Set(split(shelves).dated.map((p) => p.date.slice(0, 7)))
  return [...months].filter(isMonth).sort()
}

// The pages without a date (older numbered files), for the calendar's "Earlier" list.
export function undatedOf(shelves) {
  return split(shelves).undated
}

// 'YYYY-MM' moved by whole months: stepMonth('2026-01', -1) -> '2025-12'.
export function stepMonth(month, by) {
  const [y, m] = month.split('-').map(Number)
  const index = y * 12 + (m - 1) + by
  return `${Math.floor(index / 12)}-${String((index % 12) + 1).padStart(2, '0')}`
}

// A month as weeks, Monday first. Each cell is null (outside the month) or
// { day, date, pages } with that day's pages in the order they were written.
export function calendarWeeks(month, shelves) {
  const [y, m] = month.split('-').map(Number)
  const days = new Date(y, m, 0).getDate()
  // Monday = 0 ... Sunday = 6
  const lead = (new Date(y, m - 1, 1).getDay() + 6) % 7
  const byDay = new Map()
  for (const page of split(shelves).dated) {
    if (page.date.slice(0, 7) !== month) continue
    const list = byDay.get(page.date) ?? []
    list.push(page)
    byDay.set(page.date, list)
  }
  const cells = Array(lead).fill(null)
  for (let day = 1; day <= days; day += 1) {
    const date = `${month}-${String(day).padStart(2, '0')}`
    // the shelves are newest first; a day reads oldest first
    cells.push({ day, date, pages: [...(byDay.get(date) ?? [])].reverse() })
  }
  while (cells.length % 7) cells.push(null)
  const weeks = []
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7))
  return weeks
}

// The month to show first: the wanted page's month, else the newest month with a page, else today's.
export function startMonth(shelves, wantedDate, today) {
  if (isDate(wantedDate)) return wantedDate.slice(0, 7)
  const months = monthsOf(shelves)
  return months[months.length - 1] ?? (today ?? '').slice(0, 7)
}

// The diary as a book: every page oldest first, the ones without a date before the rest.
export function bookPages(shelves) {
  const { dated, undated } = split(shelves)
  // within the shelves a day's later page comes first: reversing the whole gives writing order
  const inOrder = [...dated].reverse().sort((a, b) => a.date.localeCompare(b.date))
  return [...[...undated].reverse(), ...inOrder]
}

// The left page's index of the spread that holds `index` (spreads are 0-1, 2-3, ...).
export function spreadStart(index) {
  return Math.max(0, index - (index % 2))
}

// The spread the book opens at: the wanted page's, else the last one (the newest pages).
export function openSpread(pages, wantedFile) {
  const wanted = pages.findIndex((p) => p.file === wantedFile)
  return spreadStart(wanted >= 0 ? wanted : Math.max(0, pages.length - 1))
}

// The two pages of a spread: { left, right } (right is null on a last single page).
export function spreadAt(pages, start) {
  return { left: pages[start] ?? null, right: pages[start + 1] ?? null }
}

// Turning by spreads, stopping at both ends: turn(2, +1, 5) -> 4, turn(4, +1, 5) -> 4.
export function turn(start, by, total) {
  const last = spreadStart(Math.max(0, total - 1))
  return Math.min(last, Math.max(0, start + by * 2))
}

// "pages 3-4 of 12", "page 13 of 13".
export function spreadLabel(start, total) {
  if (!total) return ''
  return start + 1 < total ? `pages ${start + 1}-${start + 2} of ${total}` : `page ${start + 1} of ${total}`
}

// ---------- the reading page ----------

// Which reading page each way of browsing uses (Kiyo, Nov 10 2026: Book A, Calendar B, List C).
export const PAGE_DESIGNS = { book: 'letter', calendar: 'journal', list: 'notebook' }

// A diary entry as plain paragraphs, or null when it holds markdown structure (headings,
// lists, tables, code) and should be shown by the markdown reader instead.
export function plainParagraphs(text) {
  const body = String(text ?? '').replace(/\r/g, '').trim()
  if (!body) return []
  if (/^\s*(#{1,6}\s|[-*+]\s|\d+\.\s|\||```|>)/m.test(body)) return null
  return body.split(/\n\s*\n/).map((p) => p.replace(/\n/g, ' ').trim()).filter(Boolean)
}

// "a 2 minute read" at an easy reading pace; never less than one.
export function readMinutes(text) {
  const words = String(text ?? '').trim().split(/\s+/).filter(Boolean).length
  return Math.max(1, Math.round(words / 200))
}

// The sentence lifted out as a pull quote: the first one spoken in the first person that is
// neither too short nor too long, looked for after the opening paragraph. Null when none fits.
export function pullQuote(paragraphs) {
  for (const paragraph of (paragraphs ?? []).slice(1)) {
    const sentences = paragraph.match(/[^.!?]+[.!?]+/g) ?? []
    const found = sentences.map((s) => s.trim()).find((s) => /^I\b/.test(s) && s.length >= 30 && s.length <= 120)
    if (found) return found
  }
  return null
}

// { day: '5', month: 'NOV', weekday: 'Thursday' } for a date; null without one.
export function dateParts(date, locale = 'en-US') {
  if (!isDate(date)) return null
  const d = new Date(`${date}T00:00:00`)
  return {
    day: String(d.getDate()),
    month: d.toLocaleDateString(locale, { month: 'short' }).toUpperCase(),
    weekday: d.toLocaleDateString(locale, { weekday: 'long' }),
  }
}

// A small constellation that belongs to one date: five points in a 0..1 box, always the same
// for the same date, different between dates.
export function emblemPoints(date) {
  let seed = 0
  for (const c of String(date ?? '')) seed = (seed * 31 + c.charCodeAt(0)) >>> 0
  const next = () => {
    seed = (seed * 1664525 + 1013904223) >>> 0
    return seed / 4294967296
  }
  return Array.from({ length: 5 }, (_, i) => ({
    // spread left to right so the joining line reads as a constellation, not a scribble
    x: Math.round((0.1 + i * 0.2 + (next() - 0.5) * 0.12) * 1000) / 1000,
    y: Math.round((0.15 + next() * 0.7) * 1000) / 1000,
  }))
}

// The entries around one page, oldest first: up to `span` either side, the page itself marked.
export function neighbours(pages, file, span = 2) {
  const at = (pages ?? []).findIndex((p) => p.file === file)
  if (at < 0) return { around: [], previous: null, next: null }
  const from = Math.max(0, at - span)
  return {
    around: pages.slice(from, at + span + 1).map((p) => ({ ...p, here: p.file === file })),
    previous: pages[at - 1] ?? null,
    next: pages[at + 1] ?? null,
  }
}

// "Nov 3" for a neighbouring entry's label.
export function shortDate(date, locale = 'en-US') {
  if (!isDate(date)) return ''
  return new Date(`${date}T00:00:00`).toLocaleDateString(locale, { month: 'short', day: 'numeric' })
}

// "Tuesday, November 10" for a page's date; '' for a page without one.
export function longDate(date, locale = 'en-US') {
  if (!isDate(date)) return ''
  return new Date(`${date}T00:00:00`).toLocaleDateString(locale, { weekday: 'long', month: 'long', day: 'numeric' })
}
