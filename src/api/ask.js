// Asking the companion from a room: the request goes into the main session exactly as if the
// member typed it in Chat (several lines go in as one paste, so nothing is sent early).
import { terminalState, terminalWrite } from './castle.js'

export async function askCompanion(lines, companion = 'your companion') {
  const state = await terminalState().catch(() => null)
  if (!state?.running) throw new Error(`Wake ${companion} first, then ask again.`)
  const text = Array.isArray(lines) ? lines.join('\n') : String(lines)
  await terminalWrite(`\u001b[200~${text}\u001b[201~`)
  await terminalWrite('\r')
}
