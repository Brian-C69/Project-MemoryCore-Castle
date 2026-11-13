import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'

const tree = { dirty: false }
vi.mock('../../api/castle.js', () => ({ devGitDirty: async () => tree.dirty }))
const { useSaveClose } = await import('./useSaveClose.js')

function setup({ canSave = true, running = true } = {}) {
  const said = []
  const closed = { count: 0 }
  const busy = ref(false)
  const close = useSaveClose({
    slot: () => 0,
    say: (t) => said.push(t),
    busy,
    running: ref(running),
    canSave: () => canSave,
    close: () => (closed.count += 1),
  })
  return { ...close, said, closed, busy }
}

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('Close session saves first', () => {
  it('closes at once with a clean tree, or when she is asleep', async () => {
    tree.dirty = false
    const c = setup()
    await c.requestClose()
    expect(c.closed.count).toBe(1)
    expect(c.said).toEqual([])
    tree.dirty = true
    const asleep = setup({ running: false })
    await asleep.requestClose()
    expect(asleep.closed.count).toBe(1)
  })

  it('types save project with changes and closes once clean and quiet twice', async () => {
    tree.dirty = true
    const c = setup()
    await c.requestClose()
    expect(c.said).toEqual(['save project'])
    expect(c.saving.value.note).toMatch(/Saving the project/)
    c.busy.value = true
    tree.dirty = false
    await vi.advanceTimersByTimeAsync(3000)
    expect(c.closed.count).toBe(0) // still working
    c.busy.value = false
    await vi.advanceTimersByTimeAsync(3000)
    expect(c.closed.count).toBe(0) // one quiet look
    await vi.advanceTimersByTimeAsync(3000)
    expect(c.closed.count).toBe(1)
    expect(c.saving.value).toBe(null)
  })

  it('leaves the session open after ten minutes, and closes on the member word', async () => {
    tree.dirty = true
    const c = setup()
    await c.requestClose()
    await vi.advanceTimersByTimeAsync(10 * 60 * 1000 + 3000)
    expect(c.saving.value.stuck).toBe(true)
    expect(c.closed.count).toBe(0)
    c.closeAnyway()
    expect(c.closed.count).toBe(1)
  })

  it('asks instead when the memory keeps no projects', async () => {
    tree.dirty = true
    const c = setup({ canSave: false })
    await c.requestClose()
    expect(c.said).toEqual([])
    expect(c.saving.value.ask).toBe(true)
    expect(c.closed.count).toBe(0)
    c.dismiss()
    expect(c.saving.value).toBe(null)
  })
})
