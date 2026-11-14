// HOME's widgets as the member arranges them: which show, how big, and where (per theme). The
// theme gives every widget its own place; the member's place, when there is one, replaces the
// theme's anchors with a top-left corner in percent of HOME and keeps the theme's width.

export const WIDGETS = [
  { id: 'clock', name: 'Clock', about: 'The time and the date.' },
  { id: 'gauges', name: 'Gauges', about: "Your computer's CPU and memory." },
  { id: 'usage', name: 'Usage', about: 'How much of your Claude Code and Codex limits is used.' },
  { id: 'today', name: 'Today', about: 'Your reminders for today.' },
  { id: 'projects', name: 'Projects', about: 'Your active projects.' },
  { id: 'diary', name: 'Diary', about: 'The newest diary entry.' },
  { id: 'growth', name: 'Growth', about: 'How your memory has grown.' },
  { id: 'backup', name: 'Backup', about: 'When your memory was last backed up.' },
  { id: 'database', name: 'Database', about: 'Your memory database and its engine.' },
]

export const SIZES = { s: 0.85, m: 1, l: 1.15 }

// Is this widget shown? USAGE has its own switch (it also stops Castle asking the CLIs).
export function isOn(id, settings = {}) {
  if (id === 'usage') return settings.showUsageCard !== false
  return !(settings.widgetsOff ?? []).includes(id)
}

// The settings change for switching a widget on or off.
export function switchPatch(id, on, settings = {}) {
  if (id === 'usage') return { showUsageCard: on }
  const off = new Set(settings.widgetsOff ?? [])
  if (on) off.delete(id)
  else off.add(id)
  return { widgetsOff: [...off].sort() }
}

// A widget's size: 's', 'm' or 'l'.
export function sizeOf(id, settings = {}) {
  const size = settings.widgetSizes?.[id]
  return size in SIZES ? size : 'm'
}

// The settings change for a widget's size ('m' is the default and is not kept).
export function sizePatch(id, size, settings = {}) {
  const sizes = { ...(settings.widgetSizes ?? {}) }
  if (size === 'm' || !(size in SIZES)) delete sizes[id]
  else sizes[id] = size
  return { widgetSizes: sizes }
}

// Where a widget sits: the member's corner (percent) with the theme's width and height, or the
// theme's own place untouched.
export function placeFor(themePlace = {}, memberPlace = null) {
  if (!memberPlace) return themePlace
  const place = { left: `${memberPlace.left}%`, top: `${memberPlace.top}%` }
  if (themePlace.width != null) place.width = themePlace.width
  if (themePlace.height != null) place.height = themePlace.height
  return place
}

// The member's place for a widget in a theme, or null.
export function memberPlace(settings = {}, themeId, id) {
  return settings.widgetPlaces?.[themeId]?.[id] ?? null
}

const round = (n) => Math.round(n * 10) / 10

// A dropped widget's corner in percent, kept wholly on HOME: `box` is where the widget now is
// (left, top, width, height in pixels, relative to HOME), `home` is HOME's width and height.
export function dropPlace(box, home) {
  if (!home?.width || !home?.height) return { left: 0, top: 0 }
  const maxLeft = Math.max(0, home.width - box.width)
  const maxTop = Math.max(0, home.height - box.height)
  const left = Math.min(Math.max(0, box.left), maxLeft)
  const top = Math.min(Math.max(0, box.top), maxTop)
  return { left: round((left / home.width) * 100), top: round((top / home.height) * 100) }
}

// The settings change for one widget put down in a theme.
export function placePatch(settings = {}, themeId, id, place) {
  const all = { ...(settings.widgetPlaces ?? {}) }
  all[themeId] = { ...(all[themeId] ?? {}), [id]: place }
  return { widgetPlaces: all }
}

// The settings change for "Back to the theme's places": only this theme's places go.
export function resetPatch(settings = {}, themeId) {
  const all = { ...(settings.widgetPlaces ?? {}) }
  delete all[themeId]
  return { widgetPlaces: all }
}
