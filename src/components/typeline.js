// Types a line into the session and presses Enter, the way the quick-action buttons and the
// switch prompt do. Codex reads an Enter that arrives together with the text as part of a paste
// and leaves the text in its box unsent, so Enter follows after a short pause (Codex needs more
// than 50 ms; this leaves margin for a busy PC). Claude Code does not mind the wait.

export const ENTER = String.fromCharCode(13)
export const ENTER_DELAY_MS = 250

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

export async function typeLine(write, text, wait = sleep) {
  await write(text)
  await wait(ENTER_DELAY_MS)
  await write(ENTER)
}
