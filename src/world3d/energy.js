// How the companion's crystal shines with the weekly limit left on the member's CLI (0..1, or
// null when unknown): full glow at full quota, softer as it is used, dim, warm and slowly pulsing
// under a fifth, a faint ember when none is left. Unknown keeps the normal glow.
export const LOW = 0.2
export const EMBER = 0.12
export const WARM = '#ffb070'

export function energyLook(energy) {
  if (energy === null || energy === undefined || Number.isNaN(Number(energy))) return { factor: 1, warm: 0, pulse: 0 }
  const e = Math.min(1, Math.max(0, Number(energy)))
  const low = e < LOW
  return {
    factor: EMBER + (1 - EMBER) * e,
    warm: low ? (LOW - e) / LOW : 0,
    pulse: low ? 0.25 : 0,
  }
}

// The quota as the WORLD room says it: "72% left (resets Thu)".
export function quotaWords(quota, now = new Date()) {
  if (!quota) return ''
  const left = Math.round(quota.leftPercent)
  if (!quota.resetsAt) return `${left}% left`
  const reset = new Date(quota.resetsAt)
  const soon = reset - now < 24 * 3600 * 1000
  const when = soon ? reset.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' }) : reset.toLocaleDateString('en-US', { weekday: 'short' })
  return `${left}% left (resets ${when})`
}
