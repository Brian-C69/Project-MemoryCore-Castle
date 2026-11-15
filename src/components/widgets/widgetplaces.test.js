import { describe, expect, it } from 'vitest'
import { dragBegins, dropPlace, isOn, startsOnControl, memberPlace, placeFor, placePatch, resetPatch, sizeOf, sizePatch, switchPatch, WIDGETS } from './widgetplaces.js'

describe('HOME widgets as the member arranges them', () => {
  it('lists every widget the themes place', () => {
    expect(WIDGETS.map((w) => w.id)).toEqual(['clock', 'gauges', 'usage', 'today', 'projects', 'diary', 'growth', 'backup', 'database'])
  })

  it('switches widgets on and off, USAGE through its own switch', () => {
    expect(isOn('diary', {})).toBe(true)
    expect(isOn('diary', { widgetsOff: ['diary'] })).toBe(false)
    expect(isOn('usage', { showUsageCard: false })).toBe(false)
    expect(isOn('usage', { widgetsOff: ['usage'] })).toBe(true)
    expect(switchPatch('usage', false)).toEqual({ showUsageCard: false })
    expect(switchPatch('diary', false, { widgetsOff: ['today'] })).toEqual({ widgetsOff: ['diary', 'today'] })
    expect(switchPatch('today', true, { widgetsOff: ['diary', 'today'] })).toEqual({ widgetsOff: ['diary'] })
  })

  it('keeps a size, medium by default and never stored', () => {
    expect(sizeOf('clock', {})).toBe('m')
    expect(sizeOf('clock', { widgetSizes: { clock: 'l' } })).toBe('l')
    expect(sizeOf('clock', { widgetSizes: { clock: 'xl' } })).toBe('m')
    expect(sizePatch('clock', 's', { widgetSizes: { today: 'l' } })).toEqual({ widgetSizes: { today: 'l', clock: 's' } })
    expect(sizePatch('today', 'm', { widgetSizes: { today: 'l' } })).toEqual({ widgetSizes: {} })
  })

  it("replaces the theme's anchors with the member's corner and keeps its width", () => {
    const theme = { right: '2%', top: 'calc(22% + 190px)', width: '28%' }
    expect(placeFor(theme, null)).toBe(theme)
    expect(placeFor(theme, { left: 40, top: 12.5 })).toEqual({ left: '40%', top: '12.5%', width: '28%' })
    expect(placeFor(undefined, { left: 1, top: 2 })).toEqual({ left: '1%', top: '2%' })
  })

  it('keeps a dropped widget wholly on HOME', () => {
    const home = { width: 1000, height: 500 }
    expect(dropPlace({ left: 250, top: 100, width: 200, height: 100 }, home)).toEqual({ left: 25, top: 20 })
    expect(dropPlace({ left: -40, top: -9, width: 200, height: 100 }, home)).toEqual({ left: 0, top: 0 })
    expect(dropPlace({ left: 990, top: 480, width: 200, height: 100 }, home)).toEqual({ left: 80, top: 80 })
    expect(dropPlace({ left: 1, top: 1, width: 1, height: 1 }, { width: 0, height: 0 })).toEqual({ left: 0, top: 0 })
  })

  it('remembers places per theme and resets only the theme in use', () => {
    let s = {}
    s = { ...s, ...placePatch(s, 'seed', 'clock', { left: 10, top: 5 }) }
    s = { ...s, ...placePatch(s, 'seed', 'today', { left: 50, top: 40 }) }
    s = { ...s, ...placePatch(s, 'crystal', 'clock', { left: 70, top: 5 }) }
    expect(memberPlace(s, 'seed', 'today')).toEqual({ left: 50, top: 40 })
    expect(memberPlace(s, 'constellation', 'today')).toBeNull()
    expect(resetPatch(s, 'seed')).toEqual({ widgetPlaces: { crystal: { clock: { left: 70, top: 5 } } } })
  })
})

describe('dragging a widget on HOME', () => {
  it('begins only past a few pixels, so a still press is a click', () => {
    expect(dragBegins(0, 0)).toBe(false)
    expect(dragBegins(3, 4)).toBe(false)
    expect(dragBegins(5, 5)).toBe(true)
    expect(dragBegins(-7, 0)).toBe(true)
  })
  it('never starts on a button, link or field', () => {
    expect(startsOnControl(['SPAN', 'BUTTON', 'DIV'])).toBe(true)
    expect(startsOnControl(['a'])).toBe(true)
    expect(startsOnControl(['DIV', 'P', 'SECTION'])).toBe(false)
    expect(startsOnControl()).toBe(false)
  })
})

