// Asking the companion from a room: the request goes into the main session exactly as if the
// member typed it in Chat (several lines go in as one paste, so nothing is sent early).
import { terminalState, terminalWrite } from './castle.js'
import { imageRequest } from './castle.js'
import { typeLine } from '../components/typeline.js'

export async function askImage(lines) {
  const outputLine = lines.find(line => line.startsWith('Save it as '))
  const output = outputLine ? outputLine.slice(11).replace(/\.$/, '') : null
  const references = lines.filter(line => line.startsWith('Use this existing image as')).map(line => JSON.parse(line.slice(line.indexOf(': ') + 2).split('. Preserve')[0]))
  return imageRequest(lines.filter(line => line && !line.startsWith('Tell me the cost first')).join('\n'), output, references, 'appearance')
}

export async function askCompanion(lines, companion = 'your companion') {
  const state = await terminalState().catch(() => null)
  if (!state?.running) throw new Error(`Wake ${companion} first, then ask again.`)
  const text = Array.isArray(lines) ? lines.join('\n') : String(lines)
  // Codex reads an Enter that arrives together with the text as part of the paste and leaves the
  // text in its box unsent; typeLine's pause before Enter is the same fix the quick actions use.
  await typeLine(terminalWrite, `\u001b[200~${text}\u001b[201~`)
}
