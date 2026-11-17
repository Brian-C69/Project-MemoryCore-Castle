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
import { version } from '../../package.json'
import { QUIPS } from './quips.js'
import { forStorage, fromStorage, linesFor, loaderDesign } from './design.js'

export const MIN_MS = 1400
export const MAX_MS = 12000
const QUIP_MS = 2600
const DESIGN_KEY = 'castle-loader-design'

// ---------- pure parts, tested ----------

// The next quip: random, never the one just shown (when there is another to show).
export function nextQuip(list, last, random = Math.random) {
  if (!list.length) return ''
  if (list.length === 1) return list[0]
  let pick = list[Math.floor(random() * list.length)]
  while (pick === last) pick = list[Math.floor(random() * list.length)]
  return pick
}

// The version as the screen shows it: v0.2.14, or nothing when there is none to show.
export function versionLabel(v) {
  return typeof v === 'string' && /^[0-9][0-9A-Za-z.+-]{0,30}$/.test(v.trim()) ? `v${v.trim()}` : ''
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
  setTimeout(() => {
    root.classList.add('leaving')
    setTimeout(() => root.remove(), 520)
  }, holdFor(now() - started))
}

// The theme in use asks for its own start-up screen: remembered now, shown from the next start's
// first moment (the screen is up before the theme is chosen, so it cannot be shown any sooner).
export function loaderRemember(theme) {
  try {
    const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()
    localStorage.setItem(DESIGN_KEY, JSON.stringify(forStorage(loaderDesign(theme), accent)))
  } catch {
    // storage may be blocked or full; the next start shows Castle's own screen
  }
}

// Puts a remembered design on the screen: colour, title, picture and where the words sit.
function dress(design) {
  if (!design) return
  if (design.accent) root.style.setProperty('--loader-accent', design.accent)
  if (design.brand) document.querySelector('#castle-loader .brand').textContent = design.brand
  if (design.position !== 'bottom') root.classList.add(`pos-${design.position}`)
  if (design.imageUrl) {
    // a quote or backslash cannot end the url early
    const safe = design.imageUrl.replace(/["\\\n\r]/g, (c) => encodeURIComponent(c))
    document.querySelector('#castle-loader .art').style.backgroundImage = `url("${safe}")`
  }
}

if (root) {
  // a window made for another screen (WORLD, the identify number) has no start-up screen
  if (window.__CASTLE_VIEW__) {
    root.remove()
  } else {
    let design = null
    try {
      design = fromStorage(localStorage.getItem(DESIGN_KEY))
    } catch {
      // storage blocked: Castle's own screen shows
    }
    dress(design)
    el('castle-loader-version').textContent = versionLabel(version)
    quips = linesFor(quips, design)
    turnQuip()
    quipTimer = setInterval(turnQuip, QUIP_MS)
    failsafe = setTimeout(loaderDone, MAX_MS)
    // a build's own lines, if it ships any; a missing or odd file changes nothing
    fetch('/loader/quips.json')
      .then((r) => (r.ok ? r.json() : []))
      .then((raw) => {
        const extra = cleanQuips(raw)
        if (extra.length) quips = linesFor([...QUIPS, ...extra], design)
      })
      .catch(() => {})
  }
}
