// Asking the companion from a room: the request goes into the main session exactly as if the
// member typed it in Chat (several lines go in as one paste, so nothing is sent early; Enter
// follows after a short pause, which Codex needs). Pictures never come this way: APPEARANCE and
// STUDIO queue them, and castle-look.ps1 writes the prompt without an AI.
import { terminalState, terminalWrite } from './castle.js'
import { typeLine } from '../components/typeline.js'

export async function askCompanion(lines, companion = 'your companion') {
  const state = await terminalState().catch(() => null)
  if (!state?.running) throw new Error(`Wake ${companion} first, then ask again.`)
  const text = Array.isArray(lines) ? lines.filter(Boolean).join('\n') : String(lines)
  await typeLine(terminalWrite, `\u001b[200~${text}\u001b[201~`)
}
