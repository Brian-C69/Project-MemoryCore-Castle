// Which computer Castle runs on, and the small differences that follow: the command key, the
// key names shown in hints, the words for the system's own parts. Safe to import where there is
// no page (vitest, Node): it then says "not a Mac", so the Windows wording stays the default.

/** True when this navigator says Mac (userAgent or platform). */
export function detectMac(nav) {
  if (!nav) return false
  const said = `${nav.userAgentData?.platform ?? ''} ${nav.platform ?? ''} ${nav.userAgent ?? ''}`
  return /mac/i.test(said)
}

// Only a page counts: Node (and so vitest) has a navigator of its own that says "MacIntel" on a
// Mac, which must not turn the tests' expected wording into the Mac's.
const pageNavigator = typeof window !== 'undefined' && typeof navigator !== 'undefined' ? navigator : undefined
export const isMac = detectMac(pageNavigator)

/** The command-ish modifier for this computer: ⌘ on a Mac, Ctrl elsewhere. */
export function modKey(event, mac = isMac) {
  return Boolean(mac ? event?.metaKey : event?.ctrlKey)
}

const MAC_KEYS = { ctrl: '⌘', alt: '⌥', shift: '⇧', enter: '↩', return: '↩', left: '←', right: '→', up: '↑', down: '↓' }

/** 'Ctrl+Enter' -> '⌘↩' on a Mac; unchanged elsewhere. */
export function shortcutLabel(text, mac = isMac) {
  if (!mac) return text
  return String(text)
    .split('+')
    .map((part) => MAC_KEYS[part.trim().toLowerCase()] ?? part.trim())
    .join('')
}

/** The Alt key's name: '⌥' on a Mac, 'Alt+' elsewhere (as in "Alt+←"). */
export const altLabel = isMac ? '⌥' : 'Alt+'

/** Pick the Mac or the Windows words. */
export function onMac(macText, otherText, mac = isMac) {
  return mac ? macText : otherText
}
