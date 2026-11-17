// The start-up screen's driver. The screen itself is plain HTML in index.html, painted before any
// script runs; this turns its quip, status line and bar, and takes it away when Castle is ready.
//
//   loaderExpect(6)                                                     how many pieces to expect (the bar)
//   loaderBegin('settings', 'Reading your settings', 'settings.json')   a piece of start-up begins
//   loaderEnd('settings')                                               ... and ends
//   loaderDone()                                                        Castle is ready (or gave up waiting)
//
// The bar is the pieces ended out of the pieces expected. The status line shows the newest piece
// still running. The screen stays up at least MIN_MS so the words can be read, and at most MAX_MS so
// a slow piece can never keep the member out.
import { QUIPS } from './quips.js'

export const MIN_MS = 1400
export const MAX_MS = 12000
const QUIP_MS = 2600
const ACCENT_KEY = 'castle-loader-accent'

// ---------- pure parts, tested ----------

// The next quip: random, never the one just shown (when there is another to show).
export function nextQuip(list, last, random = Math.random) {
  if (!list.length) return ''
  if (list.length === 1) return list[0]
  let pick = list[Math.floor(random() * list.length)]
  while (pick === last) pick = list[Math.floor(random() * list.length)]
  return pick
}

// How full the bar is, as 4-100 (a sliver shows from the first moment).
export function percent(ended, total) {
  if (!total || total < 1) return 4
  return Math.max(4, Math.min(100, Math.round((ended / total) * 100)))
}

// How long to wait before the screen may go: what is left of the minimum, never negative.
export function holdFor(shownMs, minMs = MIN_MS) {
  return Math.max(0, minMs - shownMs)
}

// Quips from public/loader/quips.json: only strings, trimmed, not empty, not long, at most 60.
export function cleanQuips(raw) {
  if (!Array.isArray(raw)) return []
  return raw
    .filter((q) => typeof q === 'string')
    .map((q) => q.trim())
    .filter((q) => q && q.length <= 120)
    .slice(0, 60)
}

// ---------- the screen ----------

const root = typeof document === 'undefined' ? null : document.getElementById('castle-loader')
const el = (id) => (root ? document.getElementById(id) : null)
const now = () => (typeof performance === 'undefined' ? Date.now() : performance.now())
const started = now()

const running = new Map() // id -> { label, file }, in the order begun
let ended = 0
let expected = 0
let finished = false
let quips = [...QUIPS]
let lastQuip = ''
let quipTimer = 0
let failsafe = 0

function show() {
  if (!root || finished) return
  const newest = [...running.values()].pop()
  if (newest) {
    el('castle-loader-step').textContent = newest.label
    el('castle-loader-file').textContent = newest.file ?? ''
  }
  el('castle-loader-bar').style.width = `${percent(ended, Math.max(expected, ended + running.size))}%`
}

function turnQuip() {
  const node = el('castle-loader-quip')
  if (!node || finished) return
  lastQuip = nextQuip(quips, lastQuip)
  node.textContent = lastQuip
}

// Says how many pieces to expect, so the bar can be honest from the start.
export function loaderExpect(count) {
  expected = Math.max(expected, count)
  show()
}

export function loaderBegin(id, label, file = '') {
  if (!root || finished) return
  running.set(id, { label, file })
  show()
}

export function loaderEnd(id) {
  if (!root || finished) return
  if (running.delete(id)) ended += 1
  show()
}

// Castle is ready: the bar fills, then the screen fades once it has been up long enough.
export function loaderDone() {
  if (!root || finished) return
  finished = true
  clearInterval(quipTimer)
  clearTimeout(failsafe)
  const step = el('castle-loader-step')
  if (step) step.textContent = 'Ready'
  const file = el('castle-loader-file')
  if (file) file.textContent = ''
  const bar = el('castle-loader-bar')
  if (bar) bar.style.width = '100%'
  try {
    // the next start shows the theme's own colour from the first moment
    const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()
    if (accent) localStorage.setItem(ACCENT_KEY, accent)
  } catch {
    // storage may be blocked; the default colour shows
  }
  setTimeout(() => {
    root.classList.add('leaving')
    setTimeout(() => root.remove(), 520)
  }, holdFor(now() - started))
}

if (root) {
  // a window made for another screen (WORLD, the identify number) has no start-up screen
  if (window.__CASTLE_VIEW__) {
    root.remove()
  } else {
    try {
      const accent = localStorage.getItem(ACCENT_KEY)
      if (accent) root.style.setProperty('--loader-accent', accent)
    } catch {
      // the default colour shows
    }
    turnQuip()
    quipTimer = setInterval(turnQuip, QUIP_MS)
    failsafe = setTimeout(loaderDone, MAX_MS)
    // a build's own lines, if it ships any; a missing or odd file changes nothing
    fetch('/loader/quips.json')
      .then((r) => (r.ok ? r.json() : []))
      .then((raw) => {
        const extra = cleanQuips(raw)
        if (extra.length) quips = [...quips, ...extra]
      })
      .catch(() => {})
  }
}
