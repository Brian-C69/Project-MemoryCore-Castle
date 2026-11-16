// HOME's credit widgets: the words and the bar for each service, kept pure for the tests.
// A credit comes from `credits_read` (src-tauri/src/credits.rs).

export const CREDIT_WIDGETS = ['credit-openai', 'credit-seedance', 'credit-suno', 'credit-elevenlabs']

const NAMES = { openai: 'OPENAI', seedance: 'SEEDANCE', suno: 'SUNO', elevenlabs: 'ELEVENLABS' }
const PAGES = { openai: 'OpenAI', seedance: 'BytePlus' }
/** A bar turns amber below this much left (or, for Seedance, above this much of the budget spent). */
const LOW = 0.15

/** 'credit-openai' -> 'openai' (also the key's name in Settings > Keys). */
export const providerOf = (widgetId) => String(widgetId).replace(/^credit-/, '')

export const isCreditWidget = (id) => CREDIT_WIDGETS.includes(id)

export const nameOf = (provider) => NAMES[provider] ?? String(provider).toUpperCase()

/** The member's own month, for the Seedance count: "2026-11" and its first moment in ms. */
export function monthNow(now = new Date()) {
  const month = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`
  return { month, start: new Date(now.getFullYear(), now.getMonth(), 1).getTime() }
}

/** "just now", "8 min ago", "3 h ago", "2 d ago". */
export function agoLabel(at, now = Date.now()) {
  if (!at) return ''
  const minutes = Math.max(0, Math.floor((now - at) / 60000))
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes} min ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 48) return `${hours} h ago`
  return `${Math.floor(hours / 24)} d ago`
}

const usd = (n) => `USD ${Number(n).toFixed(2)}`
const count = (n) => Math.round(Number(n)).toLocaleString('en-US')

/** The big figure: "USD 18.22", "~ USD 17.47", "768", "19,412", or "—" when unknown. */
export function valueLine(credit) {
  if (!credit || credit.value == null) return '—'
  if (credit.provider === 'openai' || credit.provider === 'seedance') return `${credit.estimate ? '~ ' : ''}${usd(credit.value)}`
  return count(credit.value)
}

function resetLabel(at) {
  if (!at) return ''
  return new Date(at).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
}

/** The one dim line under the figure. */
export function dimLine(credit, now = Date.now()) {
  if (!credit) return ''
  const page = PAGES[credit.provider]
  const state = credit.state?.state
  if (page && state === 'signed-out' && credit.value == null) return `Sign in to ${page} to read it`
  if (page && state === 'signed-out') return `signed out of ${page} - sign in again`
  if (page && state === 'no-answer' && (!credit.at || credit.state.at > credit.at)) return `${page}'s page did not answer ${agoLabel(credit.state.at, now)}`
  if (credit.note) return credit.note
  switch (credit.provider) {
    case 'openai': {
      if (credit.value == null) return 'Sign in, or set your balance'
      const when = credit.source === 'hand' ? `set ${agoLabel(credit.at, now)}` : `from OpenAI · ${agoLabel(credit.at, now)}`
      return credit.callsSince ? `${when} · ${callsSince(credit)} since` : when
    }
    case 'seedance': {
      const since = credit.callsSince ? ` · ${callsSince(credit)}` : ''
      if (credit.base == null) return `this month · counted here${since}`
      const when = credit.source === 'hand' ? `set ${agoLabel(credit.at, now)}` : `from BytePlus · ${agoLabel(credit.at, now)}`
      return `this month · ${when}${since}`
    }
    case 'suno':
      return credit.value == null ? 'Checking…' : 'credits'
    case 'elevenlabs': {
      if (credit.value == null) return 'Checking…'
      const reset = resetLabel(credit.resetsAt)
      return `left of ${count(credit.limit)} characters${reset ? ` · resets ${reset}` : ''}`
    }
    default:
      return ''
  }
}

function callsSince(credit) {
  const n = credit.callsSince ?? 0
  const word = credit.provider === 'seedance' ? 'render' : 'call'
  return `${n} ${word}${n === 1 ? '' : 's'}`
}

/**
 * The bar, only where there is an honest whole: { fill: 0..1, low: bool }, or null for no bar.
 * OpenAI, Suno and ElevenLabs fill with what is left; Seedance fills with the month's spend of its budget.
 */
export function barOf(credit) {
  if (!credit || credit.value == null || !credit.whole || credit.whole <= 0) return null
  const fill = Math.min(1, Math.max(0, credit.value / credit.whole))
  const low = credit.provider === 'seedance' ? fill > 1 - LOW : fill < LOW
  return { fill, low }
}

/** True when the widget offers Sign in: a page-read service not reading right now. */
export function wantsSignIn(credit) {
  if (!credit || !PAGES[credit.provider]) return false
  return credit.state?.state !== 'signed-in'
}

/** A typed amount: a number of USD with at most two decimals, or null. */
export function parseUsd(text) {
  const clean = String(text ?? '').replace(/usd|\$|,/gi, '').trim()
  if (!/^\d+(\.\d{1,2})?$/.test(clean)) return null
  return Number(clean)
}
