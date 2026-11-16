// The MARKET room's small decisions, kept pure for the tests: money and price words, who opened a
// position, the funding countdown, and quantities from a share of what is available.

export const INTERVALS = ['1m', '5m', '15m', '1H', '4H', '1D']
export const QUICK_MONEY = [1000, 10000, 100000]
export const PAPER_MAX = 1_000_000_000

/** 10182.4 -> "USD 10,182.40". */
export function usd(n) {
  const v = Number(n) || 0
  return `USD ${v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
}

/** A signed amount: "+USD 182.40" or "-USD 7.96". */
export function signedUsd(n) {
  const v = Number(n) || 0
  return `${v >= 0 ? '+' : '-'}${usd(Math.abs(v))}`
}

/** How many decimals a tick size shows (0.1 -> 1, 0.0001 -> 4, 1 -> 0). */
export function decimalsOf(step) {
  const s = Number(step)
  if (!Number.isFinite(s) || s <= 0 || s >= 1) return 0
  return Math.min(10, Math.max(0, Math.round(-Math.log10(s))))
}

/** A price shown on its tick: 67432.18 with tick 0.1 -> "67,432.2". */
export function price(n, tick = 0.01) {
  const v = Number(n)
  if (!Number.isFinite(v)) return '—'
  const d = decimalsOf(tick)
  return v.toLocaleString('en-US', { minimumFractionDigits: d, maximumFractionDigits: d })
}

/** 'gain' | 'loss' | '' for colouring a figure. */
export function tone(n) {
  const v = Number(n)
  if (!Number.isFinite(v) || Math.abs(v) < 1e-9) return ''
  return v > 0 ? 'gain' : 'loss'
}

/** Who opened a position or placed an order, in the member's words. */
export function openedBy(source, companion = 'Companion') {
  if (source === 'COMPANION_AUTO') return `${companion} (on its own)`
  if (source === 'COMPANION_ASKED') return companion
  return 'You'
}

export const sideLabel = (side) => (side === 'SHORT' ? 'Short' : 'Long')

/** The funding countdown: "02:14:09", or "" when unknown or past. */
export function countdown(nextMs, now = Date.now()) {
  const left = Math.floor((Number(nextMs) - now) / 1000)
  if (!Number.isFinite(left) || left <= 0) return ''
  const h = Math.floor(left / 3600)
  const m = Math.floor((left % 3600) / 60)
  const s = left % 60
  return [h, m, s].map((x) => String(x).padStart(2, '0')).join(':')
}

/** A quantity down to the symbol's step. */
export function toStep(qty, step) {
  const q = Number(qty)
  const s = Number(step)
  if (!Number.isFinite(q) || q <= 0) return 0
  if (!Number.isFinite(s) || s <= 0) return q
  const d = decimalsOf(s)
  return Number((Math.floor(q / s + 1e-9) * s).toFixed(d))
}

/**
 * The quantity a share of the available money buys at a price and leverage, down to the step:
 * (available * share * leverage) / price, leaving a little for the fees.
 */
export function qtyForShare(available, share, priceNow, leverage, step) {
  const a = Number(available)
  const p = Number(priceNow)
  const l = Number(leverage) || 1
  if (!(a > 0) || !(p > 0) || !(share > 0)) return 0
  const spend = a * Math.min(1, share) * 0.995
  return toStep((spend * l) / p, step)
}

/** A typed paper-money amount: a number above 0 and at most a billion, or null. */
export function parsePaperMoney(text) {
  const clean = String(text ?? '').replace(/usd|\$|,/gi, '').trim()
  if (!/^\d+(\.\d{1,2})?$/.test(clean)) return null
  const n = Number(clean)
  return n > 0 && n <= PAPER_MAX ? n : null
}

/** A fresh client id for one click, so a double click never places two orders. */
export function clientId(prefix = 'member') {
  const rand = Math.random().toString(36).slice(2, 10)
  return `${prefix}-${Date.now().toString(36)}-${rand}`
}
