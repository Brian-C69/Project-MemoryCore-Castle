import { describe, expect, it } from 'vitest'
import { sendLatest } from './sendlatest.js'

// a send that waits until the test lets it finish (or fail)
function gate() {
  const pending = []
  const send = (value) => (newer) =>
    new Promise((resolve, reject) => pending.push({ value, newer, resolve, reject }))
  return { pending, send }
}
const tick = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('one send at a time, always the latest', () => {
  it('skips the stale choices made while a send is out and sends the newest once', async () => {
    const sent = []
    let current = 'a'
    const { pending, send } = gate()
    const run = sendLatest((newer) => {
      sent.push(current)
      return send(current)(newer)
    })
    const first = run()
    current = 'b'
    run()
    current = 'c'
    run()
    expect(pending[0].newer()).toBe(true) // the first send can tell newer choices are waiting
    pending[0].resolve()
    await tick()
    pending[1].resolve()
    await first
    expect(sent).toEqual(['a', 'c'])
  })

  it('still sends the newest choice when the send before it fails', async () => {
    const sent = []
    const failures = []
    let current = 'a'
    const { pending, send } = gate()
    const run = sendLatest(
      (newer) => {
        sent.push(current)
        return send(current)(newer)
      },
      (reason) => failures.push(reason),
    )
    const first = run()
    current = 'b'
    run()
    pending[0].reject('busy')
    await tick()
    pending[1].resolve()
    await first
    expect(sent).toEqual(['a', 'b'])
    expect(failures).toEqual([]) // the newer send went through: the old failure is not shown
  })

  it('reports a failure when nothing newer is waiting, and runs again afterwards', async () => {
    const failures = []
    const run = sendLatest(
      () => Promise.reject('no screens'),
      (reason) => failures.push(reason),
    )
    await run()
    expect(failures).toEqual(['no screens'])
    await run()
    expect(failures).toEqual(['no screens', 'no screens'])
  })
})
