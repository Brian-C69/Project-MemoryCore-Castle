// What the terminal screen sends to the console by itself, not the member typing: answers to the
// console's questions and the focus reports the CLI asked for. Windows' console waits for the
// cursor answer before it starts the CLI, and a session Castle starts on its own (a provider
// switch) begins before this screen knows it is running, so these always go through. The backend
// tells them apart the same way (is_terminal_reply in session.rs); keep the two in step.
const REPLY = new RegExp(
  [
    '^\\x1b\\[[IO]$', // focus in / out
    '^\\x1b\\[[?>]?[\\d;]*[Rcn]$', // cursor position, device attributes, status
    '^\\x1b\\[\\?[\\d;]*u$', // keyboard mode
    '^\\x1b\\[\\?[\\d;]*\\$y$', // mode reports
    '^\\x1b[\\]P][\\d>][^\\x1b\\x07]*(?:\\x1b\\\\|\\x07)$', // colour (OSC) and version (DCS) replies
  ].join('|'),
)

export function isTerminalReply(data) {
  return typeof data === 'string' && data.length <= 128 && REPLY.test(data)
}
