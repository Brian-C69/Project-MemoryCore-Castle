import { ref } from 'vue'
import { calendarHome, calendarRange } from '../api/castle.js'
import { gridRange, iso, monthGrid } from '../calendar.js'
// What the room shows the moment it opens: this month's window, read behind the start-up loader.
export const calendarSeed = ref(null)
// HOME's CALENDAR card: the reminders raised lately and the next few events.
export const calendarCard = ref(null)
export async function refreshCalendarCard() {
  try { calendarCard.value = await calendarHome() } catch (e) { calendarCard.value = { notices: [], more: 0, next: [], problem: e?.message || String(e) } }
  return calendarCard.value
}
let pending = null
export function prepareCalendar() {
  if (pending) return pending
  refreshCalendarCard()
  const now = new Date()
  const range = gridRange(monthGrid(now.getFullYear(), now.getMonth(), 1))
  pending = calendarRange(range.from, range.to)
    .then((items) => { calendarSeed.value = { ...range, day: iso(now), items }; return calendarSeed.value })
    .finally(() => { pending = null })
  return pending
}
