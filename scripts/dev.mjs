// Starts Vite on whatever port is free, then runs `tauri dev` pointed at that port.
// Arguments after the script name go to Castle itself, e.g. `node scripts/dev.mjs --autostart`.
import { createServer } from 'vite'
import cli from '@tauri-apps/cli'

const server = await createServer()
await server.listen()
server.printUrls()

const devUrl = server.resolvedUrls.local[0]
const config = JSON.stringify({ build: { devUrl } })
const appArgs = process.argv.slice(2)

try {
  // After the first `--` come cargo's arguments; after the second, Castle's own.
  await cli.run(['dev', '--config', config, ...(appArgs.length ? ['--', '--', ...appArgs] : [])], 'tauri')
} finally {
  await server.close()
}
