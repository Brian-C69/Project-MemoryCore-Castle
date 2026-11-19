// Chat's mic: one click (or ⌘⇧M / Ctrl+Shift+M) and Claude Code's own voice mode records, as if
// Space were held in the Log; a second click stops it. The words Claude Code wrote into its prompt
// are then read off the Log screen, moved into Chat's box for the member to look over, and the
// prompt is emptied, so the later Send does not type them twice. Esc cancels: the prompt is emptied
// and nothing comes to the box. Pure apart from the timers; tested.
//
// How Claude Code tells a held Space (2.1.x, hold mode): five Spaces, each within 120 ms of the one
// before, start the recording (the first two land in the prompt and are taken back); once it
// records, the recording ends 200 ms after the last Space. A keyboard's auto-repeat does exactly
// this, and so does writing a Space every 40 ms into the session while the mic is on.

/** How often the mic types a Space while on: well inside Claude Code's 120 ms and 200 ms. */
export const HOLD_EVERY_MS = 40
/** Claude Code's "delete to the start of the line" (one screen line of the prompt per press). */
export const CLEAR_LINE = '\u0015'
/** How long the words may take to arrive after the stop before Castle stops waiting. */
export const SETTLE_LIMIT_MS = 20000
/** A mic left on stops by itself after this long (Claude Code's own tap mode caps at 2 minutes too). */
export const MAX_RECORDING_MS = 2 * 60 * 1000
// ---------- The mic's shortcut (Settings > Chat > Mic) ----------
//
// Saved as { code, shift, alt }: the key by its KeyboardEvent.code (KeyA..KeyZ, Digit0..Digit9),
// with ⌘ on a Mac (Ctrl elsewhere) always held, plus Shift and/or Alt as chosen. Nothing saved
// means ⌘⇧M (Ctrl+Shift+M).

/** The shortcut when the member has not chosen one: ⌘⇧M on a Mac, Ctrl+Shift+M elsewhere. */
export const DEFAULT_MIC_SHORTCUT = Object.freeze({ code: 'KeyM', shift: true, alt: false })

const KEY_CODE = /^(?:Key[A-Z]|Digit[0-9])$/

/** The key's own character for a code: 'KeyM' -> 'M', 'Digit3' -> '3'. */
const keyOf = (code) => code.replace(/^(?:Key|Digit)/, '')

/** A saved shortcut made safe to use: an unknown or missing one is the default. */
export function micShortcutOf(saved) {
  if (!saved || typeof saved !== 'object' || !KEY_CODE.test(String(saved.code ?? ''))) return { ...DEFAULT_MIC_SHORTCUT }
  return { code: saved.code, shift: !!saved.shift, alt: !!saved.alt }
}

/** True when two shortcuts are the same keys. */
export function sameMicShortcut(a, b) {
  const x = micShortcutOf(a)
  const y = micShortcutOf(b)
  return x.code === y.code && x.shift === y.shift && x.alt === y.alt
}

/** The shortcut in shortcutLabel's words: 'Ctrl+Shift+M' (shortcutLabel makes it ⌘⇧M on a Mac). */
export function micShortcutText(shortcut) {
  const s = micShortcutOf(shortcut)
  return ['Ctrl', s.alt && 'Alt', s.shift && 'Shift', keyOf(s.code)].filter(Boolean).join('+')
}

/** The default shortcut's words, as before the shortcut could be changed. */
export const MIC_SHORTCUT = micShortcutText(DEFAULT_MIC_SHORTCUT)

/** The keys a shortcut must never take, with what they already do there. Keyed 'shift+alt+code'. */
const TAKEN_MAC = {
  'KeyC': 'Copy', 'KeyV': 'Paste', 'KeyX': 'Cut', 'KeyA': 'Select all', 'KeyZ': 'Undo',
  'KeyQ': 'Quit Castle', 'KeyW': 'Close the window', 'KeyH': 'Hide Castle', 'KeyM': 'Minimise',
  'shift+KeyZ': 'Redo', 'shift+KeyQ': 'Log out of the Mac', 'shift+Digit3': 'a screenshot',
  'shift+Digit4': 'a screenshot', 'shift+Digit5': 'a screenshot', 'alt+KeyH': 'Hide the other apps',
  'alt+KeyM': 'Minimise every window',
}
const TAKEN_OTHER = {
  'KeyC': 'Copy', 'KeyV': 'Paste', 'KeyX': 'Cut', 'KeyA': 'Select all', 'KeyZ': 'Undo', 'KeyY': 'Redo',
  'shift+KeyZ': 'Redo',
}

/** What a shortcut already does on this computer ('Copy', 'Undo'...), or '' when it is free. */
export function micShortcutTaken(shortcut, mac) {
  const s = micShortcutOf(shortcut)
  const name = [s.shift && 'shift', s.alt && 'alt', s.code].filter(Boolean).join('+')
  return (mac ? TAKEN_MAC : TAKEN_OTHER)[name] ?? ''
}

const MODIFIERS = new Set(['Meta', 'Control', 'Shift', 'Alt', 'AltGraph', 'CapsLock', 'Fn', 'OS'])

/**
 * Reads a key press made while Settings waits for a new shortcut. Returns null while only
 * modifiers are held (the member is still pressing), { cancel: true } for Esc, { problem, keys }
 * when the keys cannot be the mic's, or { shortcut, keys } for a good one. `label` turns
 * 'Ctrl+Shift+M' words into this computer's (shortcutLabel).
 */
export function readMicShortcut(event, mac, label = (text) => text) {
  if (!event) return null
  if (event.key === 'Escape') return { cancel: true }
  if (MODIFIERS.has(event.key)) return null
  const mod = mac ? '⌘' : 'Ctrl'
  const held = mac ? !!event.metaKey : !!event.ctrlKey
  const other = mac ? !!event.ctrlKey : !!event.metaKey
  const named = KEY_CODE.test(event.code ?? '') ? keyOf(event.code) : String(event.key ?? '').length === 1 ? String(event.key).toUpperCase() : String(event.key ?? '')
  const pressed = label([held && 'Ctrl', event.altKey && 'Alt', event.shiftKey && 'Shift', named].filter(Boolean).join('+'))
  if (!KEY_CODE.test(event.code ?? '')) return { problem: `${pressed} can't be the mic's key. Pick a letter or a digit, with ${mod}.`, keys: pressed }
  if (other) return { problem: mac ? 'Leave Control out: on a Mac the mic uses ⌘.' : 'Leave the Windows key out: the mic uses Ctrl.', keys: pressed }
  if (!held) return { problem: `${pressed} would go off while you type. Hold ${mod} with it.`, keys: pressed }
  const shortcut = { code: event.code, shift: !!event.shiftKey, alt: !!event.altKey }
  const taken = micShortcutTaken(shortcut, mac)
  if (taken) return { problem: `${pressed} is already ${taken}. Pick another key, or add ${mac ? '⇧' : 'Shift'}.`, keys: pressed }
  return { shortcut, keys: pressed }
}

/**
 * What Chat's 🎙 shows: null for no button, else { ready, hint, shortcut }. Nothing shows until both
 * Claude Code's voice (`voice`: undefined until read, null when it cannot be read, else
 * { enabled, mode }) and the member's shortcut (`shortcut`: undefined until read) are known, so the
 * button never shows ⌘⇧M first and the member's own keys a moment later.
 */
export function micOffer({ running, claude, voice, shortcut }) {
  if (!running || !claude || voice === undefined || shortcut === undefined) return null
  const keys = micShortcutOf(shortcut)
  if (voice === null) return { ready: true, shortcut: keys, hint: 'Talk - turn voice on first with /voice in the Log' }
  if (!voice.enabled) return null
  if (voice.mode !== 'hold') return { ready: false, shortcut: keys, hint: "Chat's mic needs voice in hold mode: type /voice hold in the Log" }
  return { ready: true, shortcut: keys, hint: 'Talk: click to start, click again to stop. Your words come into the box to look over before Send.' }
}

/**
 * True for the mic's shortcut only: its key with ⌘ on a Mac (Ctrl elsewhere) and exactly its
 * Shift and Alt, nothing else held. Every other key (⌘X, ⌘⇧X, Ctrl+M, a plain M) is left alone.
 * The key counts by its code, or by its letter on a layout that puts the letter elsewhere.
 */
export function isMicShortcut(event, mac, shortcut = DEFAULT_MIC_SHORTCUT) {
  if (!event) return false
  const s = micShortcutOf(shortcut)
  if (!!event.shiftKey !== s.shift || !!event.altKey !== s.alt) return false
  const code = event.code === s.code
  const key = String(event.key ?? '').toLowerCase() === keyOf(s.code).toLowerCase()
  if (!code && !key) return false
  return mac ? !!event.metaKey && !event.ctrlKey : !!event.ctrlKey && !event.metaKey
}

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
 * empty prompt is dim, its "listening…" too); `pause`, `every` and `after` are the timers (replaced
 * in tests).
 * start({ onCap }) begins typing Spaces; `onCap` is called if it is still on after
 * MAX_RECORDING_MS (the caller then stops it as usual). stop() ends it and resolves { text, note }
 * once the words settled: the words for Chat's box ('' for none) and, when something went wrong, a
 * short line about it. cancel() ends it the same way but empties the prompt and brings no words.
 */
export function voiceMic({
  write,
  readScreen,
  readTyped,
  pause = (ms) => new Promise((done) => setTimeout(done, ms)),
  every = setInterval,
  stopEvery = clearInterval,
  after = setTimeout,
  stopAfter = clearTimeout,
}) {
  let timer = null
  let cap = null
  let on = false

  function start({ onCap } = {}) {
    if (on) return false
    on = true
    write(' ').catch(() => {})
    timer = every(() => write(' ').catch(() => {}), HOLD_EVERY_MS)
    cap = after(() => {
      cap = null
      if (on) onCap?.()
    }, MAX_RECORDING_MS)
    return true
  }

  // No more Spaces: Claude Code ends the recording 200 ms later.
  function quiet() {
    on = false
    stopEvery(timer)
    timer = null
    if (cap !== null) stopAfter(cap)
    cap = null
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
    if (!on) return { text: '', note: '' }
    quiet()
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

  // Esc: Claude Code still writes the words into its prompt, so wait for them, then take them away.
  async function cancel() {
    if (!on) return { text: '', note: '' }
    quiet()
    const { prompt } = await settled()
    if (!prompt) return { text: '', note: 'Cancelled, but the Log could not be read - open Log to empty its prompt.' }
    let emptied = false
    try {
      emptied = prompt.text ? await clearPrompt(prompt) : (await write(CLEAR_LINE), true)
    } catch {
      emptied = false
    }
    return { text: '', note: emptied ? 'Cancelled.' : 'Cancelled, but the words are still in the Log - empty its prompt before you send.' }
  }

  return { start, stop, cancel, on: () => on }
}
