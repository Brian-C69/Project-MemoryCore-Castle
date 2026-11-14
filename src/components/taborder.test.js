import { describe, expect, it } from 'vitest'
import { moveTab, neighbourOf, orderTabs } from './taborder.js'

const TABS = ['home', 'theme', 'apps', 'chat', 'dev'].map((id) => ({ id, label: id.toUpperCase() }))
const ids = (tabs) => tabs.map((t) => t.id)

describe('the top bar in the member order', () => {
  it('keeps Castle order when nothing is saved', () => {
    expect(ids(orderTabs(TABS, []))).toEqual(['home', 'theme', 'apps', 'chat', 'dev'])
    expect(ids(orderTabs(TABS, undefined))).toEqual(['home', 'theme', 'apps', 'chat', 'dev'])
  })
  it('follows the saved order, puts tabs it does not name after, and passes over old ids', () => {
    expect(ids(orderTabs(TABS, ['dev', 'gone', 'home']))).toEqual(['dev', 'home', 'theme', 'apps', 'chat'])
  })
})

describe('moving a tab', () => {
  const all = ['home', 'theme', 'apps', 'chat', 'dev']
  it('drops it after the tab it lands on when moving right, before it when moving left', () => {
    expect(moveTab(all, 'home', 'apps')).toEqual(['theme', 'apps', 'home', 'chat', 'dev'])
    expect(moveTab(all, 'dev', 'theme')).toEqual(['home', 'dev', 'theme', 'apps', 'chat'])
    expect(moveTab(all, 'dev', 'home')).toEqual(['dev', 'home', 'theme', 'apps', 'chat'])
  })
  it('gives the same list back when nothing moves', () => {
    expect(moveTab(all, 'chat', 'chat')).toBe(all)
    expect(moveTab(all, 'nope', 'chat')).toBe(all)
    expect(all).toEqual(['home', 'theme', 'apps', 'chat', 'dev'])
  })
  it('finds the visible neighbour for the keyboard, and none at the ends', () => {
    expect(neighbourOf(['home', 'chat', 'dev'], 'chat', -1)).toBe('home')
    expect(neighbourOf(['home', 'chat', 'dev'], 'chat', 1)).toBe('dev')
    expect(neighbourOf(['home', 'chat', 'dev'], 'dev', 1)).toBeNull()
    expect(neighbourOf(['home', 'chat', 'dev'], 'gone', 1)).toBeNull()
  })
})
