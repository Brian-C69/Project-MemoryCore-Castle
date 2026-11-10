import { describe, expect, it } from 'vitest'
import { isTerminalReply } from './terminalreply.js'

describe('terminal replies', () => {
  it('passes the answers and reports the console asked for', () => {
    for (const reply of ['\x1b[1;1R', '\x1b[24;80R', '\x1b[?1;2c', '\x1b[>0;10;1c', '\x1b[0n', '\x1b[I', '\x1b[O']) {
      expect(isTerminalReply(reply)).toBe(true)
    }
  })

  it('never treats typing as a reply', () => {
    for (const typed of ['hello', '\r', '\x1b[A', '\x1b[200~paste\x1b[201~', '\x1b[1;1Rmore', '\x1b[', 'R', '\x1b[1;1H', null]) {
      expect(isTerminalReply(typed)).toBe(false)
    }
  })
})
