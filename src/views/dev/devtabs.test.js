import { describe, expect, it } from 'vitest'
import {
  devRoom,
  devSlotOf,
  folderName,
  groupByPhase,
  itemMark,
  nextActive,
  openPlans,
  orderTabs,
  pasted,
  planProgress,
  recentProjects,
  samePath,
  sessionView,
  tabForFolder,
} from './devtabs.js'

const tab = (slot, folder, running = false) => ({ slot, folder, name: folderName(folder), running, status: { ready: true, message: '', fix: null } })

describe('DEV tabs', () => {
  it('orders the open tabs by slot', () => {
    expect(orderTabs([tab(3, 'C:\\c'), tab(0, 'C:\\a'), tab(1, 'C:\\b')]).map((t) => t.slot)).toEqual([0, 1, 3])
    expect(orderTabs(null)).toEqual([])
  })

  it('compares folders the Windows way', () => {
    expect(samePath('D:/Work/Site/', 'd:\\work\\site')).toBe(true)
    expect(samePath('D:\\Work\\Site', 'D:\\Work\\Sites')).toBe(false)
    expect(samePath('', '')).toBe(false)
    expect(tabForFolder([tab(2, 'D:\\Work\\Site')], 'd:/work/site').slot).toBe(2)
  })

  it('names a folder by its last part', () => {
    expect(folderName('D:\\Work\\My Site\\')).toBe('My Site')
    expect(folderName('')).toBe('')
  })

  it('keeps the current tab, or moves to its nearest neighbour when it closed', () => {
    const tabs = [tab(0, 'a'), tab(2, 'b'), tab(5, 'c')]
    expect(nextActive(tabs, 2)).toBe(2)
    expect(nextActive(tabs, 3)).toBe(5)
    expect(nextActive(tabs, 7)).toBe(5)
    expect(nextActive(tabs, null)).toBe(0)
    expect(nextActive([], 2)).toBe(null)
  })

  it('shows the tab of the folder just opened', () => {
    const tabs = [tab(0, 'D:\\a'), tab(1, 'D:\\b')]
    expect(nextActive(tabs, 0, 'd:/b')).toBe(1)
    expect(nextActive(tabs, 0, 'D:\\missing')).toBe(0)
  })

  it('lists recent projects without the memory folder, marking the open ones', () => {
    const projects = [
      { folder: 'D:\\Memory', name: 'Memory' },
      { folder: 'D:\\Site', name: 'Site' },
      { folder: 'D:\\Shop', name: '' },
    ]
    expect(recentProjects(projects, [tab(4, 'D:\\Site')], 'd:/memory/')).toEqual([
      { folder: 'D:\\Site', name: 'Site', slot: 4 },
      { folder: 'D:\\Shop', name: 'Shop', slot: null },
    ])
  })

  it('reads a closed tab as asleep', () => {
    expect(sessionView(null).running).toBe(false)
    expect(sessionView(tab(1, 'a', true))).toEqual({ running: true, status: { ready: true, message: '', fix: null } })
  })

  it('names a tab’s chat room and reads the slot back, leaving other rooms alone', () => {
    expect(devRoom(3)).toBe('dev:3')
    expect(devSlotOf('dev:3')).toBe(3)
    expect(devSlotOf('dev:0')).toBe(0)
    expect(devSlotOf('dev:8')).toBe(null)
    expect(devSlotOf('story')).toBe(null)
    expect(devSlotOf(undefined)).toBe(null)
    expect(devSlotOf('dev:1x')).toBe(null)
  })

  it('sends a message as one bracketed paste', () => {
    expect(pasted('two\nlines')).toBe('\u001b[200~two\nlines\u001b[201~')
  })
})

describe('DEV plans', () => {
  const items = [
    { ordinal: 3, kind: 'task', text: 'Build the card', state: 'done' },
    { ordinal: 1, kind: 'task', text: 'Read the brief', state: 'carried' },
    { ordinal: 2, kind: 'phase', text: 'Phase 1 - Cards', state: 'open' },
    { ordinal: 4, kind: 'seal', text: 'Seal', state: 'open' },
    { ordinal: 5, kind: 'phase', text: 'Phase 2 - Polish', state: 'open' },
    { ordinal: 6, kind: 'task', text: 'Polish', state: 'flagged' },
  ]

  it('groups the boxes under their phase rows, in order', () => {
    const groups = groupByPhase(items)
    expect(groups.map((g) => g.heading)).toEqual([null, 'Phase 1 - Cards', 'Phase 2 - Polish'])
    expect(groups[0].items.map((i) => i.text)).toEqual(['Read the brief'])
    expect(groups[1].items.map((i) => i.text)).toEqual(['Build the card', 'Seal'])
    expect(groups[2].items.map((i) => i.text)).toEqual(['Polish'])
  })

  it('keeps an empty phase as a heading, and no empty headless group', () => {
    expect(groupByPhase([{ ordinal: 1, kind: 'phase', text: 'Phase 1', state: 'open' }])).toEqual([{ heading: 'Phase 1', state: 'open', items: [] }])
    expect(groupByPhase([])).toEqual([])
  })

  it('marks done with a tick, carried with a dash, flagged with a flag, open with nothing', () => {
    expect(itemMark('done').mark).toBe('✓')
    expect(itemMark('carried').mark).toBe('–')
    expect(itemMark('flagged').mark).toBe('⚑')
    expect(itemMark('open').mark).toBe('')
    expect(itemMark('something new').mark).toBe('')
  })

  it('counts boxes done, not phase rows', () => {
    expect(planProgress(items)).toBe('1 / 4')
    expect(planProgress([])).toBe('')
  })

  it('lists only the open plans', () => {
    const plans = [{ id: 1, status: 'open' }, { id: 2, status: 'done' }, { id: 3, status: 'Completed' }, { id: 4, status: 'active' }]
    expect(openPlans(plans).map((p) => p.id)).toEqual([1, 4])
  })
})
