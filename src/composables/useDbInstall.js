// The database engine's download, kept for the whole run of Castle instead of inside the panel.
// Settings > Database opens and closes, but the core keeps downloading; this holds the one running
// install and its progress so the panel can show it again when it reopens, and so a second press
// of Install joins the first instead of starting another download.
import { reactive } from 'vue'
import { dbInstall, onDbInstall } from '../api/castle.js'
import { progressLine } from '../components/database.js'

export const installState = reactive({ running: false, line: '' })

let current = null
let listening = false

async function listen() {
  if (listening) return
  listening = true
  try {
    await onDbInstall(({ done, total }) => {
      if (installState.running) installState.line = progressLine(done, total)
    })
  } catch {
    // outside Castle's window
    listening = false
  }
}

// Starts the install, or hands back the one already running.
export function runInstall(start = dbInstall) {
  if (current) return current
  installState.running = true
  installState.line = progressLine(0, 0)
  listen()
  current = Promise.resolve()
    .then(start)
    .finally(() => {
      current = null
      installState.running = false
      installState.line = ''
    })
  return current
}

// The install running right now, or null: for a panel that opens while it is going.
export function runningInstall() {
  return current
}
