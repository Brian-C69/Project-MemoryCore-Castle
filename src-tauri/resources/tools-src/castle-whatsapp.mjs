// MemoryCore in your pocket, WhatsApp: a small local sidecar Castle spawns, speaking Baileys
// (a WhatsApp Web client) on one side and a plain localhost HTTP door on the other, so the Rust
// side never needs to know anything about WhatsApp's own protocol - only this door's shape.
//
// Env vars Castle sets when it starts this: WA_AUTH_DIR (where the paired session is kept, so a
// restart does not ask for the QR again), WA_PORT (the local port this door listens on).
//
// This file is a SOURCE, not what Castle ships. It is bundled into one dependency-free file with
// esbuild (Baileys needs real npm packages; bundling means Castle ships one script, the same way
// it ships castle-image.mjs, with nothing to npm install on the member's machine):
//
//   npm install @whiskeysockets/baileys
//   npx esbuild castle-whatsapp.mjs --bundle --platform=node --format=cjs --outfile=../tools/castle-whatsapp.cjs
//
// (--format=cjs, not esm: some of Baileys' own dependencies use a dynamic require() for Node's
// builtin modules, which only resolves correctly in a CommonJS bundle.)

import http from 'node:http'
import { makeWASocket, useMultiFileAuthState, DisconnectReason, isJidGroup, isJidBroadcast, isJidStatusBroadcast } from '@whiskeysockets/baileys'

// v1: one-to-one chats only, no groups/broadcasts/status updates
const isOneToOne = (jid) => !!jid && !isJidGroup(jid) && !isJidBroadcast(jid) && !isJidStatusBroadcast(jid)

const AUTH_DIR = process.env.WA_AUTH_DIR ?? './castle-whatsapp-auth'
const PORT = Number(process.env.WA_PORT ?? 51847)

let sock = null
let state = null
let saveCreds = null
let latestQr = null
let connected = false
let lastProblem = null
const incoming = []

async function connect() {
  const auth = await useMultiFileAuthState(AUTH_DIR)
  state = auth.state
  saveCreds = auth.saveCreds
  sock = makeWASocket({ auth: state, printQRInTerminal: false, syncFullHistory: false })

  sock.ev.on('creds.update', saveCreds)

  sock.ev.on('connection.update', (update) => {
    if (update.qr) latestQr = update.qr
    if (update.connection === 'open') {
      connected = true
      latestQr = null
      lastProblem = null
    }
    if (update.connection === 'close') {
      connected = false
      const code = update.lastDisconnect?.error?.output?.statusCode
      if (code === DisconnectReason.loggedOut) {
        lastProblem = 'Logged out on the phone. Scan a new QR to pair again.'
      } else {
        lastProblem = 'Reconnecting…'
        setTimeout(connect, 3000)
      }
    }
  })

  sock.ev.on('messages.upsert', ({ messages }) => {
    for (const m of messages) {
      if (m.key.fromMe) continue
      const chatId = m.key.remoteJid
      if (!isOneToOne(chatId)) continue
      const text = (m.message?.conversation ?? m.message?.extendedTextMessage?.text ?? '').trim()
      if (!text) continue
      incoming.push({ id: m.key.id, chatId, name: m.pushName ?? 'Someone', text })
    }
  })
}

function json(res, code, body) {
  res.writeHead(code, { 'Content-Type': 'application/json' })
  res.end(JSON.stringify(body))
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let body = ''
    req.on('data', (chunk) => { body += chunk })
    req.on('end', () => resolve(body))
    req.on('error', reject)
  })
}

const server = http.createServer(async (req, res) => {
  try {
    // local-only by design: this door is bound to 127.0.0.1 below, never the network
    if (req.method === 'GET' && req.url === '/status') {
      return json(res, 200, { paired: !!state?.creds?.registered, connected, qr: latestQr, problem: lastProblem })
    }
    if (req.method === 'GET' && req.url === '/incoming') {
      const drained = incoming.splice(0, incoming.length)
      return json(res, 200, { messages: drained })
    }
    if (req.method === 'POST' && req.url === '/send') {
      const { to, text } = JSON.parse(await readBody(req))
      await sock.sendMessage(to, { text })
      return json(res, 200, { ok: true })
    }
    json(res, 404, { error: 'not found' })
  } catch (error) {
    json(res, 500, { error: String(error?.message ?? error) })
  }
})

server.listen(PORT, '127.0.0.1', () => {
  console.log(`castle-whatsapp listening on 127.0.0.1:${PORT}`)
})

connect().catch((error) => {
  console.error('castle-whatsapp could not start', error)
  process.exit(1)
})
