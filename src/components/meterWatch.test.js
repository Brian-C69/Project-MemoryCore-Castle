import { describe, expect, it } from 'vitest'
import { boundLimit, freedLine, isHot, ownerTag, sizeLabel, tripHeadline } from './meterWatch.js'

describe('the gauges keep watch', () => {
  it('holds a typed value to the gear bounds', () => {
    expect(boundLimit(10)).toBe(50)
    expect(boundLimit(120)).toBe(99)
    expect(boundLimit('85')).toBe(85)
    expect(boundLimit('', 70)).toBe(70)
    expect(boundLimit('abc')).toBe(90)
  })

  it('says sizes plainly', () => {
    expect(sizeLabel(850)).toBe('850 MB')
    expect(sizeLabel(1434)).toBe('1.4 GB')
    expect(sizeLabel(undefined)).toBe('0 MB')
  })

  it('tags every protected program and leaves an ordinary app untagged', () => {
    for (const owner of ['castle', 'companion', 'database', 'project', 'server', 'editor', 'system']) {
      expect(ownerTag(owner)).not.toBe('')
    }
    expect(ownerTag('')).toBe('')
  })

  it('tells the trip honestly', () => {
    expect(tripHeadline({ meter: 'cpu', value: 93 })).toBe('CPU at 93% - here is what is using it')
    expect(tripHeadline({ meter: 'memory', value: 91, freedMb: 600 })).toContain('some comes back as apps use it')
    expect(tripHeadline({ meter: 'memory', value: 91, freedMb: 0 })).toBe('RAM at 91% - nothing more to tidy')
    expect(freedLine(null)).toContain('not available')
    expect(freedLine(2048)).toBe('Tidied 2.0 GB - some comes back as apps use it.')
  })

  it('glows only while the watch is on and the value is reached', () => {
    expect(isHot(91, 90, true)).toBe(true)
    expect(isHot(91, 90, false)).toBe(false)
    expect(isHot(89, 90, true)).toBe(false)
  })
})
