import { describe, expect, it } from 'vitest'
import { countMerged, countsFile, countsLine, smallAvatar } from './contributors-tools.mjs'

const pull = (login, merged, extra = {}) => ({ merged_at: merged ? '2026-11-19T10:00:00Z' : null, user: { login, type: 'User', avatar_url: `https://avatars.githubusercontent.com/u/${login.length}?v=4`, ...extra } })

describe('the contributors wall', () => {
  it('counts merged pull requests per person, the most first', () => {
    const people = countMerged([pull('Brian-C69', true), pull('OneNonly148', true), pull('Brian-C69', true), pull('Brian-C69', false)])
    expect(people.map(({ login, merged }) => [login, merged])).toEqual([['Brian-C69', 2], ['OneNonly148', 1]])
    expect(people[0].avatar).toContain('avatars.githubusercontent.com')
  })

  it('leaves out the owner, bots and pulls that were never merged', () => {
    const people = countMerged([pull('Kiyoraka', true), pull('kiyoraka', true), pull('dependabot[bot]', true), pull('helper', true, { type: 'Bot' }), pull('Rin', false), { merged_at: 'x' }])
    expect(people).toEqual([])
  })

  it('orders a tie by login and builds the file the page reads', () => {
    const people = countMerged([pull('zed', true), pull('amy', true)])
    expect(people.map((p) => p.login)).toEqual(['amy', 'zed'])
    const file = countsFile([{ login: 'Brian-C69', merged: 49, avatar: 'x' }, { login: 'OneNonly148', merged: 4 }], '2026-11-19T16:00:00Z')
    expect(file).toEqual({ at: '2026-11-19T16:00:00Z', merged: 53, people: [{ login: 'Brian-C69', merged: 49 }, { login: 'OneNonly148', merged: 4 }] })
    expect(countsLine(file)).toBe('Contributors: 2 contributors, 53 merged')
    expect(countsLine(countsFile([{ login: 'a', merged: 1 }], 'x'))).toBe('Contributors: 1 contributor, 1 merged')
  })

  it('asks for a small avatar', () => {
    expect(smallAvatar('https://avatars.githubusercontent.com/u/1?v=4')).toBe('https://avatars.githubusercontent.com/u/1?v=4&s=96')
    expect(smallAvatar('https://example.com/a.png', 64)).toBe('https://example.com/a.png?s=64')
  })
})
