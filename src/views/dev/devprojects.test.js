import { describe, expect, it } from 'vitest'
import { linkOf, ownerWords, programOf, serversOf } from './devprojects.js'

const servers = [
  { port: 5174, pid: 1, program: 'node.exe', slot: 0, startedBy: 'Lyra (its tab)' },
  { port: 3306, pid: 2, program: 'mysqld.exe', slot: null, startedBy: '' },
  { port: 5173, pid: 3, program: 'node.exe', slot: 0, startedBy: 'you (its folder)' },
  { port: 8000, pid: 4, program: 'python.exe', slot: null, startedBy: '' },
  { port: 4000, pid: 5, program: 'node.exe', slot: 2, startedBy: 'Lyra (its tab)' },
]
const tabs = [{ slot: 0, name: 'order-tracker' }]

describe("DEV's live servers", () => {
  it("gives a tab only its own servers, by port", () => {
    expect(serversOf(servers, 0).map((s) => s.port)).toEqual([5173, 5174])
    expect(serversOf(servers, 1)).toEqual([])
    expect(serversOf(null, 0)).toEqual([])
  })

  it('names who a server belongs to', () => {
    expect(ownerWords(servers[0], tabs)).toBe('order-tracker')
    expect(ownerWords(servers[0], [{ slot: 0, name: 'order-tracker', label: 'order-tracker · 2' }])).toBe('order-tracker · 2')
    expect(ownerWords(servers[4], tabs)).toBe('Tab 3')
    expect(ownerWords(servers[1], tabs)).toBe('-')
    expect(ownerWords(servers[3], tabs)).toBe('(not a DEV project)')
  })

  it('opens only the local address and names the program plainly', () => {
    expect(linkOf(servers[0])).toBe('http://localhost:5174')
    expect(programOf(servers[1])).toBe('mysqld')
    expect(programOf({})).toBe('')
  })
})
