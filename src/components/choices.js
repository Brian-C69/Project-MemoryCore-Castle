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
// "Tell Claude what to change", "No, and tell Claude what to do", "Type something".
const TELL = /\btell claude\b|^type something\b/i
// The tab row of a multi-question form, never the question.
const TABS = /[←→]|[✓✔]\s*Submit/
// One tab of that row: its box (☒ / ■ / ✔ = answered) and its name.
const TAB = /([☐☒□■✔])\s*([^☐☒□■✔✓←→]+)/g
const ANSWERED = /[☒■✔]/
// How far above option 1 the question's paragraph may reach.
const QUESTION_ROWS = 12
// A line this much shorter than the paragraph's longest ends a block of its own (a hard line
// break, "Bash command" / "cargo test"); longer lines are one sentence wrapped by the terminal.
const WRAPPED = 0.7

function clean(line) {
  return line.replace(BORDER, ' ').trim()
}

function readTabs(line) {
  const tabs = []
  for (const [, box, name] of line.replace(/[✓✔]\s*Submit/, '').matchAll(TAB)) {
    const text = name.trim()
    if (text) tabs.push({ name: text, done: ANSWERED.test(box) })
  }
  return tabs
}

// The paragraph above option 1 (to a blank line, a rule or a form's tab row), as blocks of wrapped
// lines, and the tab row when one sits just above it.
function paragraphAbove(all, firstRaw) {
  const lines = []
  let i = firstRaw - 1
  while (i >= 0 && !all[i]) i -= 1
  for (; i >= 0 && lines.length < QUESTION_ROWS && all[i]; i -= 1) {
    if (TABS.test(all[i])) break
    lines.unshift(all[i])
  }
  // the tab row: right there, or one blank (or rule) further up
  let tabs = []
  for (let seen = 0; i >= 0 && seen < 2; i -= 1) {
    if (!all[i]) continue
    seen += 1
    if (TABS.test(all[i])) {
      tabs = readTabs(all[i])
      break
    }
  }
  const longest = Math.max(0, ...lines.map((line) => line.length))
  const blocks = []
  let block = []
  lines.forEach((line, n) => {
    block.push(line)
    if (n < lines.length - 1 && line.length < longest * WRAPPED) {
      blocks.push(block.join(' '))
      block = []
    }
  })
  if (block.length) blocks.push(block.join(' '))
  return { blocks, tabs }
}

// The question is the sentence ending at the paragraph's last "?" (wrapped over lines or not);
// everything else in the paragraph is its context - a parenthesis after it, the lines before it.
function splitQuestion(blocks) {
  const at = blocks.findLastIndex((text) => text.includes('?'))
  if (at < 0) return { question: '', context: blocks.join(' · ') }
  const text = blocks[at]
  const end = text.lastIndexOf('?')
  const start = Math.max(text.lastIndexOf('. ', end), text.lastIndexOf('? ', end - 1), text.lastIndexOf('! ', end)) + 1
  const question = text.slice(start, end + 1).trim()
  const context = [...blocks.slice(0, at), text.slice(0, start).trim(), text.slice(end + 1).trim(), ...blocks.slice(at + 1)]
    .filter(Boolean)
    .join(' · ')
  return { question, context }
}

// The option of a card that opens a text field for the member's own words ("Tell Claude what to
// change", "Type something."), or undefined. While it is up, Chat's box IS that option.
export const tellOf = (menu) => menu?.options.find((option) => option.tell)
// A question's own "Type something." is the member's own answer, not a change to the plan.
export const isOwnAnswer = (option) => /^type something\b/i.test(option?.label ?? '')
// A "No, and tell..." keeps its button: a plain no must stay one click; other free-text options
// are answered from the box instead.
export const hidesTell = (option) => !!option.tell && !/^no\b/i.test(option.label)

// The folder-trust question Claude Code asks the first time it opens a folder (a new DEV tab sat
// silent in Chat because the question was only in the Log). Its options carry no numbers - "No, exit" / "❯ Yes, I trust this
// folder" - so each gets the keys that reach it from the marked one: arrows, then Enter.
const UP = '\x1b[A'
const DOWN = '\x1b[B'
function readTrust(all, raw, rows) {
  const at = rows.findLastIndex((row) => /^[❯›>]?\s*yes,?\s+i trust this folder\b/i.test(row))
  if (at < 0 || at < rows.length - NEAR_BOTTOM) return null
  // the option block: the rows around it that are short lines with no sentence end, up to 4
  let first = at
  let last = at
  while (first > 0 && at - first < 3 && /^[❯›>]?\s*(yes|no)\b/i.test(rows[first - 1])) first -= 1
  while (last < rows.length - 1 && last - at < 3 && /^[❯›>]?\s*(yes|no)\b/i.test(rows[last + 1])) last += 1
  const block = rows.slice(first, last + 1)
  const marked = block.findIndex((row) => MARKED_ANY.test(row))
  if (marked < 0 || block.length < 2) return null
  const options = block.map((row, n) => {
    const steps = n - marked
    const keys = (steps > 0 ? DOWN.repeat(steps) : UP.repeat(-steps)) + '\r'
    return { number: n + 1, label: row.replace(MARKED_ANY, '').trim(), keys }
  })
  // the question sits a few paragraphs up (the folder and "Security guide" come between)
  const asked = rows.slice(Math.max(0, first - QUESTION_ROWS), first).findLast((row) => row.includes('?'))
  const { question } = splitQuestion(asked ? [asked] : [])
  const folder = rows.slice(Math.max(0, first - QUESTION_ROWS), first).find((row) => /^[A-Za-z]:\\|^\//.test(row))
  return { question: question || 'Do you trust this folder?', ...(folder ? { context: folder } : {}), options }
}
const MARKED_ANY = /^[❯›>]\s*/

// A question that takes several answers: each option is a box,
// "[ ]" or "[✓]", a number toggles it, and an unnumbered "Submit" row under the boxes sends them -
// to the next tab of a form, or as the answer. The card had no way to reach that row.
const BOX = /^\[[ ✓✔xX×]\]\s/
const RIGHT = '\x1b[C'
function submitOf(options, markedAt, tabs) {
  const lastBox = options.findLastIndex((option) => BOX.test(option.label))
  if (lastBox < 0) return null
  // in a form the right arrow is the next tab (what Log takes); alone, the arrows walk from
  // the marked option to the row just under the last box, and Enter presses it
  if (tabs.length) return { label: 'Next →', keys: RIGHT }
  const steps = lastBox + 1 - Math.max(0, markedAt)
  return { label: 'Submit', keys: (steps > 0 ? DOWN.repeat(steps) : UP.repeat(-steps)) + '\r' }
}

export function readChoices(lines) {
  const all = lines.map(clean)
  const raw = []
  all.forEach((line, i) => line && raw.push(i))
  const rows = raw.map((i) => all[i])
  if (rows.length === 0) return null

  // The last option line near the bottom.
  let end = -1
  for (let i = rows.length - 1; i >= Math.max(0, rows.length - NEAR_BOTTOM); i -= 1) {
    if (OPTION.test(rows[i])) {
      end = i
      break
    }
  }
  if (end < 0) return readTrust(all, raw, rows)

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

  const multi = found.some((option) => BOX.test(option.label))
  const options = found.map((option, n) => {
    const next = n + 1 < found.length ? found[n + 1].index : end + 1
    let hint = n + 1 < found.length ? rows.slice(option.index + 1, next).join(' ').trim() : ''
    // the Submit row under the last box is a button of its own, never that option's description
    if (multi && BOX.test(option.label) && !BOX.test(found[n + 1]?.label ?? '')) hint = hint.replace(/\s*\bSubmit$/, '').trim()
    const label = option.label.replace(/\s*\(esc\)\s*$/i, '').trim()
    return {
      number: option.number,
      label,
      ...(hint ? { hint } : {}),
      // The option that opens a text field for the member's own words: Chat's box stands in for it.
      ...(TELL.test(label) ? { tell: true } : {}),
    }
  })

  // The question and its context, read from the whole paragraph above option 1 (a question
  // whose paragraph ends in ")" would otherwise be lost, and with it every word).
  const { blocks, tabs } = paragraphAbove(all, raw[first])
  const { question, context } = splitQuestion(blocks)

  const markedAt = found.findIndex((option) => MARKED.test(rows[option.index]))
  const submit = multi ? submitOf(options, markedAt, tabs) : null
  return { question, ...(context ? { context } : {}), ...(tabs.length ? { tabs } : {}), options, ...(submit ? { submit } : {}) }
}
