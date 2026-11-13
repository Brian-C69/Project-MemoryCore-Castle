// Claude Code's permission mode, read from the Log screen and changed with Claude Code's own key,
// and the context the session has used, read from Castle's status line. Pure, tested.
//
// Claude Code cycles manual -> accept edits -> plan -> auto with Shift+Tab, and its footer says
// which is on ("⏵⏵ accept edits on (shift+tab to cycle)", "⏸ plan mode on ..."). Manual has no
// words of its own: the prompt box shows with none of the others.

export const MODES = [
  { id: 'manual', label: 'Manual' },
  { id: 'accept', label: 'Accept edits' },
  { id: 'plan', label: 'Plan' },
  { id: 'auto', label: 'Auto' },
]

/** What the terminal receives for Shift+Tab. */
export const SHIFT_TAB = '\u001b[Z'

const SAYS = [
  ['accept', /accept edits on/i],
  ['plan', /plan mode on/i],
  ['auto', /(auto mode on|auto-accept on|bypass permissions on)/i],
]
// the prompt box or its hint line: the screen is at rest and readable
const PROMPT = /(\? for shortcuts|shift\+tab to cycle|^\s*[>❯]\s|^[\s│|]*[>❯]\s|─{12,})/i

/**
 * The mode the screen shows, from its last lines (newest last): 'manual' | 'accept' | 'plan' |
 * 'auto', or null when the footer can't be seen (starting, a question up, a long output).
 */
export function readMode(lines) {
  const tail = (lines ?? []).slice(-14)
  for (let i = tail.length - 1; i >= 0; i -= 1) {
    const line = String(tail[i] ?? '')
    const found = SAYS.find(([, pattern]) => pattern.test(line))
    if (found) return found[0]
  }
  return tail.some((line) => PROMPT.test(String(line ?? ''))) ? 'manual' : null
}

/** How many Shift+Tab presses from one mode to another along the cycle (0 to 3). */
export function pressesTo(from, to) {
  const order = MODES.map((m) => m.id)
  const a = order.indexOf(from)
  const b = order.indexOf(to)
  if (a < 0 || b < 0) return 1
  return (b - a + order.length) % order.length
}

// Castle's status line (castle-statusline.ps1): "<model> | 42% | 120.3k tokens".
const CONTEXT = /\|\s*(\d{1,3})%\s*\|\s*([\d.]+\s*[kKmM]?)\s*tokens/

/** The context as the status line says it: { percent, tokens } from the newest such line, or null. */
export function readContext(lines) {
  const tail = (lines ?? []).slice(-14)
  for (let i = tail.length - 1; i >= 0; i -= 1) {
    const m = CONTEXT.exec(String(tail[i] ?? ''))
    if (m) return { percent: Math.min(100, Number(m[1])), tokens: m[2].replace(/\s+/g, '') }
  }
  return null
}

/** Where the warm colour starts: the window is nearly full. */
export const WARM_FROM = 80

/** The meter's colour: neutral, then warm near the top. */
export function contextTone(percent) {
  return percent >= WARM_FROM ? 'warm' : 'neutral'
}
