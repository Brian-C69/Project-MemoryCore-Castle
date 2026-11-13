import { describe, expect, it, vi } from 'vitest'
import { avatarLoader, currentAvatar, datedLines, dayMarker, roomMembers, speakerPresentation } from './roomchat.js'

describe('room presence after companion keys change', () => {
  it('merges duplicate companion rows and keeps the active status, latest read and picture', () => {
    const rows = [{ name: "Bernard's Mira", kind: 'member_assistant', status: 'away', seen: 40, avatar: 'old' }, { name: "Bernard's Mira", kind: 'member_assistant', status: 'online', seen: 10, avatar: 'new' }]
    expect(roomMembers(rows)).toEqual([{ name: "Bernard's Mira", kind: 'member_assistant', status: 'online', seen: 40, avatar: 'new' }])
    expect(rows[0].status).toBe('away')
  })
  it('preserves different owners with the same companion name and does not merge humans', () => {
    const rows = [{ name: "Bernard's Mira", kind: 'member_assistant' }, { name: "Rin's Mira", kind: 'member_assistant' }, { name: 'Rin', kind: 'member' }, { name: 'Rin', kind: 'member' }]
    expect(roomMembers(rows)).toHaveLength(4)
  })
  it('collapses identical stale away entries and prefers the latest read on ties', () => {
    const rows = [{ name: "Bernard's Mira", kind: 'member_assistant', status: 'away', seen: 0, avatar: 'picture' }, { name: "Bernard's Mira", kind: 'member_assistant', status: 'away', seen: 0 }]
    expect(roomMembers(rows)).toHaveLength(1)
    expect(roomMembers(rows)[0].avatar).toBe('picture')
    expect(roomMembers([...rows, { ...rows[0], seen: 30, avatar: 'latest' }])[0].avatar).toBe('latest')
  })
})

describe('companions have their own display names', () => {
  it('shows the companion alone and retains the owner for hover without mutating identity', () => {
    const author = { name: "Brian.C's Mira", kind: 'member_assistant' }
    expect(speakerPresentation(author)).toEqual({ name: 'Mira', owner: 'Brian.C', title: 'Master: Brian.C' })
    expect(author.name).toBe("Brian.C's Mira")
    expect(speakerPresentation({ name: 'Rin’s Mika', kind: 'member_assistant' }).name).toBe('Mika')
  })
  it('gives the host companion the same hover treatment and leaves humans unchanged', () => {
    expect(speakerPresentation({ name: 'Alice', kind: 'assistant' }, 'Kiyo')).toEqual({ name: 'Alice', owner: 'Kiyo', title: 'Master: Kiyo' })
    expect(speakerPresentation({ name: "John's shop", kind: 'member' })).toEqual({ name: "John's shop", owner: '', title: '' })
    expect(speakerPresentation({ name: 'Unlinked', kind: 'member_assistant' }).owner).toBe('')
  })
  it('prefers explicit owner/name metadata to a formatted name', () => {
    expect(speakerPresentation({ name: "Old's Name", kind: 'member_assistant', ownerName: 'New owner', assistantName: 'New name' })).toEqual({ name: 'New name', owner: 'New owner', title: 'Master: New owner' })
  })
})

describe('room pictures', () => {
  it('retries transient failures, coalesces requests and keeps successful pictures', async () => {
    let now = 0
    const fetch = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValue('data:image/png;base64,ok')
    const show = vi.fn()
    const load = avatarLoader(fetch, show, { now: () => now, retryMs: 100 })
    await Promise.all([load('face'), load('face')])
    expect(fetch).toHaveBeenCalledTimes(1)
    await load('face')
    expect(fetch).toHaveBeenCalledTimes(1)
    now = 100
    await Promise.all([load('face'), load('face')])
    expect(show).toHaveBeenCalledWith('face', 'data:image/png;base64,ok')
    await load('face')
    expect(fetch).toHaveBeenCalledTimes(2)
    await load('new-token')
    expect(fetch).toHaveBeenCalledTimes(3)
  })
  it('uses presence for missing historical avatars without mixing speakers', () => {
    const author = { name: 'Rin', kind: 'member' }
    const presence = [{ ...author, avatar: 'current' }, { name: 'Rin', kind: 'member_assistant', avatar: 'ai' }]
    expect(currentAvatar(author, presence)).toBe('current')
    expect(currentAvatar({ ...author, avatar: 'recorded' }, presence)).toBe('recorded')
    expect(currentAvatar(author, [...presence, { ...author, avatar: 'ambiguous' }])).toBeNull()
    expect(currentAvatar(null, presence)).toBeNull()
  })
})

describe('room calendar separators', () => {
  const now = new Date(2026, 10, 14, 0, 5)
  it('labels local today, yesterday, recent weekdays and older dates', () => {
    expect(dayMarker(new Date(2026, 10, 14), now).label).toBe('Today')
    expect(dayMarker(new Date(2026, 10, 13, 23, 59), now).label).toBe('Yesterday')
    expect(dayMarker(new Date(2026, 10, 12), now).label).toBe('Thursday')
    expect(dayMarker(new Date(2026, 10, 11), now).label).toBe('Wednesday')
    expect(dayMarker(new Date(2026, 10, 7), now).label).toBe('7 November 2026')
  })
  it('groups adjacent messages once and keeps invalid dates out of labels', () => {
    const rows = datedLines([{ id: 1, at: new Date(2026, 10, 13) }, { id: 2, at: new Date(2026, 10, 13, 23) }, { id: 3, at: 'bad' }, { id: 4, at: new Date(2026, 10, 14) }], now)
    expect(rows.filter((r) => r.kind === 'day').map((r) => r.label)).toEqual(['Yesterday', 'Today'])
    expect(rows.filter((r) => r.kind === 'message').map((r) => r.line.id)).toEqual([1, 2, 3, 4])
    expect(dayMarker(null, now)).toBeNull()
    expect(dayMarker('bad', now)).toBeNull()
  })
  it('handles year changes and daylight-saving calendar boundaries', () => {
    expect(dayMarker(new Date(2025, 11, 31, 23, 59), new Date(2026, 0, 1)).label).toBe('Yesterday')
    expect(dayMarker(new Date(2026, 2, 8), new Date(2026, 2, 9)).label).toBe('Yesterday')
  })
})
