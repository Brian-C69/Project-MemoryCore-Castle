// What the terminal screen sends to the console by itself, not the member typing: answers to the
// console's questions (cursor position, device attributes, status) and the focus reports the CLI
// asked for. Windows' console waits for the cursor answer before it starts the CLI, and a session
// Castle starts on its own (a provider switch) begins before this screen knows it is running, so
// these always go through. The backend tells them apart the same way (session.rs).
const REPLY = /^\x1b\[(?:[?>]?[\d;]*[Rcn]|[IO])$/

export function isTerminalReply(data) {
  return typeof data === 'string' && data.length <= 32 && REPLY.test(data)
}
