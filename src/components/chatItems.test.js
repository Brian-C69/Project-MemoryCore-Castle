import { describe, expect, it } from 'vitest'
import { buildItems, isMarker, openPlanFile } from './chatItems.js'

const FILE = 'C:\\Users\\Lyra\\.claude\\plans\\hazy-noodling-meerkat.md'
const PLAN = '# Work Lv.24 - The Handover\n\n### Phase 1\n- [ ] one'
const at = (hh, mm) => `2026-11-11T${String(hh).padStart(2, '0')}:${String(mm).padStart(2, '0')}:00.000Z`
const e = (id, role, text, time) => ({ id, role, text, at: time })
// what the Chat read from the plan file while the question was up
const waiting = { path: FILE, text: PLAN, at: at(15, 34) }
const kinds = (items) => items.map((item) => (item.kind === 'plan' ? `plan:${item.id}` : item.entry?.text ?? item.kind))

describe('the plan cards in Chat', () => {
  it('lets the card go once the plan is answered', () => {
    const entries = [
      e('k1', 'member', 'plan the handover', at(15, 26)),
      e('pf1', 'plan-file', FILE, at(15, 27)),
      e('pf2', 'plan-file', FILE, at(15, 28)),
      e('w1', 'work', 'ExitPlanMode', at(15, 28)),
      e('p1', 'plan', `${PLAN}\n(edited)`, at(15, 34)),
      e('end', 'plan-end', FILE, at(15, 35)),
      e('a1', 'companion', 'built and sealed', at(16, 20)),
    ]
    expect(openPlanFile(entries)).toBeNull()
    expect(kinds(buildItems(entries, waiting))).toEqual(['plan the handover', 'plan:p1', 'built and sealed'])
  })

  it('shows the card read from the file at the bottom while the question is up', () => {
    const entries = [e('k1', 'member', 'plan it', at(15, 26)), e('pf1', 'plan-file', FILE, at(15, 27)), e('w1', 'work', 'ExitPlanMode', at(15, 28))]
    expect(openPlanFile(entries)).toBe(FILE)
    const items = buildItems(entries, waiting)
    expect(kinds(items)).toEqual(['plan it', `plan:plan-file-${FILE}`])
    expect(items[1].title).toBe('Work Lv.24 - The Handover')
  })

  it('settles the file card where the plan ended when the record never carried the plan', () => {
    const entries = [
      e('pf1', 'plan-file', FILE, at(15, 27)),
      e('w1', 'work', 'ExitPlanMode', at(15, 28)),
      e('end', 'plan-end', FILE, at(15, 35)),
      e('a1', 'companion', 'starting now', at(15, 36)),
    ]
    const items = buildItems(entries, waiting)
    expect(kinds(items)).toEqual([`plan:plan-file-${FILE}`, 'starting now'])
    expect(items[0].entry.at).toBe(at(15, 35))
  })

  it('keeps only the newest plan file waiting at the bottom', () => {
    const other = 'C:\\Users\\Lyra\\.claude\\plans\\calm-river.md'
    const entries = [e('pf1', 'plan-file', FILE, at(15, 27)), e('w1', 'work', 'ExitPlanMode', at(15, 28)), e('pf2', 'plan-file', other, at(15, 40))]
    expect(openPlanFile(entries)).toBe(other)
    expect(buildItems(entries, waiting)).toEqual([])
    const fresh = { path: other, text: '# Calm River', at: at(15, 41) }
    expect(kinds(buildItems(entries, fresh))).toEqual([`plan:plan-file-${other}`])
  })

  it('draws the same plan asked, then approved, as one card', () => {
    const entries = [e('p1', 'plan', PLAN, at(15, 28)), e('k1', 'member', 'yes', at(15, 29)), e('p2', 'plan', PLAN, at(15, 30))]
    expect(kinds(buildItems(entries))).toEqual(['plan:p1', 'yes'])
  })
})

describe('the markers', () => {
  it('are never drawn', () => {
    expect(isMarker({ role: 'plan-file' })).toBe(true)
    expect(isMarker({ role: 'plan-end' })).toBe(true)
    expect(isMarker({ role: 'member' })).toBe(false)
  })
})
