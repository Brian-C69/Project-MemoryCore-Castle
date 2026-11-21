import { afterEach, describe, expect, it, vi } from 'vitest'

vi.mock('../api/castle.js', () => ({ dbInstall: vi.fn(), onDbInstall: vi.fn(async () => () => {}) }))

import { installState, runInstall, runningInstall } from './useDbInstall.js'

function deferred() {
  let resolve
  let reject
  const promise = new Promise((res, rej) => ((resolve = res), (reject = rej)))
  return { promise, resolve, reject }
}

afterEach(async () => {
  await runningInstall()?.catch(() => {})
})

describe('the database install kept for the whole run', () => {
  it('shows an install as running until it finishes', async () => {
    const job = deferred()
    const first = runInstall(() => job.promise)
    expect(installState.running).toBe(true)
    expect(installState.line).toBe('Downloading…')
    job.resolve('done')
    await first
    expect(installState.running).toBe(false)
    expect(installState.line).toBe('')
    expect(runningInstall()).toBe(null)
  })

  it('a second press joins the install already running instead of starting another', async () => {
    const job = deferred()
    const start = vi.fn(() => job.promise)
    const first = runInstall(start)
    const second = runInstall(start)
    expect(second).toBe(first)
    expect(runningInstall()).toBe(first)
    job.resolve('done')
    await first
    expect(start).toHaveBeenCalledTimes(1)
  })

  it('lets a reopened panel wait on the same install and hear the same failure', async () => {
    const job = deferred()
    const first = runInstall(() => job.promise)
    const follower = runningInstall()
    job.reject('MariaDB could not be downloaded.')
    await expect(first).rejects.toBe('MariaDB could not be downloaded.')
    await expect(follower).rejects.toBe('MariaDB could not be downloaded.')
    expect(installState.running).toBe(false)
  })

  it('keeps why the last install failed for a panel that opens afterwards, and clears it on the next try', async () => {
    await expect(runInstall(() => Promise.reject('MariaDB could not be downloaded.'))).rejects.toBe('MariaDB could not be downloaded.')
    expect(installState.error).toBe('MariaDB could not be downloaded.')
    const job = deferred()
    const next = runInstall(() => job.promise)
    expect(installState.error).toBe('')
    job.resolve('ok')
    await next
    expect(installState.error).toBe('')
  })

  it('starts a fresh install after the last one failed', async () => {
    const failed = runInstall(() => Promise.reject('no internet'))
    await expect(failed).rejects.toBe('no internet')
    const start = vi.fn(() => Promise.resolve('ok'))
    await runInstall(start)
    expect(start).toHaveBeenCalledTimes(1)
  })
})
