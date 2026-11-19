// Chat's mic: hold the button and Claude Code's own voice mode records, as if Space were held in
// the Log. On release the words Claude Code wrote into its prompt are read off the Log screen,
// moved into Chat's box for the member to look over, and the prompt is emptied, so the later Send
// does not type them twice. Pure apart from the timers; tested.
//
// How Claude Code tells a held Space (2.1.x, hold mode): five Spaces, each within 120 ms of the one
// before, start the recording (the first two land in the prompt and are taken back); once it
// records, the recording ends 200 ms after the last Space. A keyboard's auto-repeat does exactly
// this, and so does writing a Space every 40 ms into the session.

/** How often a held button types a Space: well inside Claude Code's 120 ms and 200 ms. */
export const HOLD_EVERY_MS = 40
/** Claude Code's "delete to the start of the line" (one screen line of the prompt per press). */
export const CLEAR_LINE = '\u0015'
/** How long the words may take to arrive after release before Castle stops waiting. */
export const SETTLE_LIMIT_MS = 20000

// Claude Code's prompt box: a rule (or the older rounded box) above and below, the first line
// starts with its ❯ (or >), the lines after it are indented to line up.
const RULE = /^\s*(?:[╭╰]─|─{12,})/
const FIRST = /^\s*│?\s*[❯>]\s?(.*)$/
// Claude Code's own words while it records or writes the words down (its footer)
const WORKING = /keep holding…|listening…|Transcribing…/
// What Claude Code says when nothing could be written down
const PROBLEMS = [
  /No speech detected\./,
  /No audio detected from microphone\.[^\n]*/,
  /Microphone access is denied\.[^\n]*/,
  /voice mode isn't ready yet[^\n]*/,
  /Voice connection failed\.[^\n]*/,
]

const unboxed = (line) => String(line ?? '').replace(/\s*│\s*$/, '')

/**
 * The prompt Claude Code shows at the bottom of the screen (lines newest last, its dim placeholder
 * already left out): { text, lines } with the screen lines joined as one text, or null when no
 * prompt box can be seen (a question is up, or the session is starting).
 */
export function readPrompt(screen) {
  const lines = (screen ?? []).map(unboxed)
  for (let i = lines.length - 1; i > 0; i -= 1) {
    const first = FIRST.exec(lines[i])
    if (!first || !RULE.test(lines[i - 1])) continue
    const parts = [first[1]]
    let end = i + 1
    while (end < lines.length && !RULE.test(lines[end])) {
      parts.push(lines[end].replace(/^\s*│?/, ''))
      end += 1
    }
    // a prompt box is closed by its lower rule; without it this was not the box
    if (end >= lines.length) continue
    const text = parts.map((p) => p.trim()).filter(Boolean).join(' ').replace(/\s+/g, ' ').trim()
    return { text, lines: end - i }
  }
  return null
}

/** True while Claude Code is still recording or writing the words down. */
export const voiceWorking = (screen) => (screen ?? []).slice(-12).some((line) => WORKING.test(String(line ?? '')))

/** Claude Code's message when it could not write anything down, or ''. */
export function voiceProblem(screen) {
  const text = (screen ?? []).slice(-12).join('\n')
  for (const pattern of PROBLEMS) {
    const found = pattern.exec(text)
    if (found) return found[0].trim()
  }
  return ''
}

/**
 * The mic for one session. `write(data)` types into it; `readScreen(count)` gives the last screen
 * lines as shown, `readTyped(count)` the same without dim text (Claude Code's placeholder in an
 * empty prompt is dim, its "listening…" too); `pause` and `every` are the timers (replaced in tests).
 * start() begins typing Spaces; stop() ends it and resolves { text, note } once the words settled:
 * the words for Chat's box ('' for none) and, when something went wrong, a short line about it.
 */
export function voiceHold({ write, readScreen, readTyped, pause = (ms) => new Promise((done) => setTimeout(done, ms)), every = setInterval, stopEvery = clearInterval }) {
  let timer = null
  let held = false

  function start() {
    if (held) return false
    held = true
    write(' ').catch(() => {})
    timer = every(() => write(' ').catch(() => {}), HOLD_EVERY_MS)
    return true
  }

  // Waits until Claude Code is done (no "listening…" / "Transcribing…" and the prompt the same
  // twice running), then reads the prompt.
  async function settled() {
    await pause(400)
    let last = null
    for (let waited = 400; waited < SETTLE_LIMIT_MS; waited += 150) {
      const screen = readScreen(40)
      const working = voiceWorking(screen)
      const prompt = readPrompt(readTyped(40))
      if (!working && prompt && last && prompt.text === last.text) return { prompt, screen }
      last = working ? null : prompt
      await pause(150)
    }
    return { prompt: null, screen: readScreen(40) }
  }

  // Empties the prompt a screen line at a time, reading it again after each round.
  async function clearPrompt(prompt) {
    let left = prompt
    for (let round = 0; round < 3 && left; round += 1) {
      await write(CLEAR_LINE.repeat(left.lines + 1))
      await pause(200)
      left = readPrompt(readTyped(40))
      if (left && !left.text) return true
    }
    return !left || !left.text
  }

  async function stop() {
    if (!held) return { text: '', note: '' }
    held = false
    stopEvery(timer)
    timer = null
    const { prompt, screen } = await settled()
    if (!prompt) return { text: '', note: 'Your words did not come back from the Log - open Log to see them.' }
    if (!prompt.text) {
      // a hold too short to record leaves its Spaces in the prompt
      await write(CLEAR_LINE).catch(() => {})
      return { text: '', note: voiceProblem(screen) }
    }
    let emptied = false
    try {
      emptied = await clearPrompt(prompt)
    } catch {
      emptied = false
    }
    return { text: prompt.text, note: emptied ? '' : 'The words are still in the Log too - empty its prompt before you send.' }
  }

  return { start, stop, held: () => held }
}
