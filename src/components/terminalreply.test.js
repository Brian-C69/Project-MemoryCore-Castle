import { describe, expect, it } from 'vitest'
import { isTerminalReply } from './terminalreply.js'

describe('terminal replies', () => {
  it('passes the answers and reports the console asked for', () => {
    // the bytes Castle's screen (xterm.js) sent back to real Codex and Claude Code queries, plus the same
    // shapes ending in BEL and as a DCS version reply (not captured, but the same kind of reply)
    for (const reply of [
      '\x1b[1;1R', '\x1b[24;80R', '\x1b[?1;2c', '\x1b[>0;276;0c', '\x1b[0n', '\x1b[I', '\x1b[O',
      '\x1b[?0u', '\x1b[?2026;2$y', '\x1b[?1;2$y',
      '\x1b]10;rgb:eeee/f3f3/ffff\x1b\\', '\x1b]11;rgb:0000/0000/0000\x1b\\', '\x1b]4;1;rgb:cccc/0000/0000\x1b\\',
      '\x1b]11;rgb:0000/0000/0000\x07', '\x1bP>|xterm.js(5.5.0)\x1b\\',
    ]) {
      expect(isTerminalReply(reply)).toBe(true)
    }
  })

  it('never treats typing as a reply', () => {
    for (const typed of [
      'hello', '\r', '\x7f', '\t', '\x1b', '\x1b[A', '\x1b[1;5C', '\x1b[3~', '\x1b[Z', '\x1bOP', '\x1b[200~paste\x1b[201~',
      '\x1b[1;1Rmore', '\x1b[', 'R', '\x1b[1;1H', '\x1b[97;5u', '\x1b[<0;10;5M', '\x1b]\x1b\\', '\x1b]11;rgb:0/0/0\x1b\\x',
      '\x1b[?2026;2$u', '\x1bb', '\x1b]hello\x07', null,
    ]) {
      expect(isTerminalReply(typed)).toBe(false)
    }
  })
})
