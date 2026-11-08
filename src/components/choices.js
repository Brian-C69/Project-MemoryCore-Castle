// Reads a numbered choice menu from the bottom of the Log screen, the way Claude Code shows a
// permission prompt or a question:
//
//   Do you want to proceed?              "Finish the system" ... Which is it?
//   ❯ 1. Yes                               1. Plan it with me now
//     2. Yes, and don't ask again             I open plan mode: scope, stack ...   <- a description
//     3. No, and tell Claude what to do    ❯ 4. just put in the weekly board
//                                          ─────────────────────────────
//                                            5. Chat about this
//                                          Enter to select · Tab/Arrow keys to navigate · Esc to cancel
//
// The companion's ordinary numbered lists have no selection marker, so a menu needs one on an option line.
// This reads Claude Code's screen, which can change; when nothing is read, no buttons appear.

const BORDER = /[│┃║╭╮╰╯─━┌┐└┘]/g
const OPTION = /^(?:[❯›>]\s*)?(\d{1,2})[.)]\s+(.+?)\s*$/
const MARKED = /^[❯›>]\s*\d/
// The menu's last option must be this close to the bottom, counted in non-empty lines.
const NEAR_BOTTOM = 8
// How far up a menu (with its descriptions) may reach.
const MENU_ROWS = 40
// The tab row of a multi-question form, never the question.
const TABS = /[←→]|✓\s*Submit/

function clean(line) {
  return line.replace(BORDER, ' ').trim()
}

export function readChoices(lines) {
  const rows = lines.map(clean).filter((line) => line.length > 0)
  if (rows.length === 0) return null

  // The last option line near the bottom.
  let end = -1
  for (let i = rows.length - 1; i >= Math.max(0, rows.length - NEAR_BOTTOM); i -= 1) {
    if (OPTION.test(rows[i])) {
      end = i
      break
    }
  }
  if (end < 0) return null

  // Walk up for the options counting down to 1; lines between them are their descriptions.
  const found = []
  let expected = Number(rows[end].match(OPTION)[1])
  for (let i = end; i >= 0 && i >= end - MENU_ROWS && expected >= 1; i -= 1) {
    const match = rows[i].match(OPTION)
    if (!match) continue
    if (Number(match[1]) !== expected) return null
    found.unshift({ index: i, number: expected, label: match[2] })
    expected -= 1
  }
  if (expected !== 0 || found.length < 2) return null

  const first = found[0].index
  if (!rows.slice(first, end + 1).some((row) => MARKED.test(row))) return null

  const options = found.map((option, n) => {
    const next = n + 1 < found.length ? found[n + 1].index : end + 1
    const hint = n + 1 < found.length ? rows.slice(option.index + 1, next).join(' ').trim() : ''
    return {
      number: option.number,
      label: option.label.replace(/\s*\(esc\)\s*$/i, '').trim(),
      ...(hint ? { hint } : {}),
    }
  })

  // The question: the line just above option 1 when it ends in "?" (not a form's tab row) -
  // its last sentence, since a long question wraps over lines above it.
  const asked = first > 0 && !TABS.test(rows[first - 1]) ? rows[first - 1] : ''
  const question = asked.endsWith('?') ? asked.slice(asked.lastIndexOf('. ') + 1).trim() : ''

  return { question, options }
}
