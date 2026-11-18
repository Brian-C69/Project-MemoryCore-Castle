import { describe, expect, it } from 'vitest'
import { compass, dayName, hereMessage, mood, placeLine, tempText, weatherTitle, windText } from './weather.js'

describe('the weather widget\'s words', () => {
  it('gives the compass point the wind comes from', () => {
    expect(compass(0)).toBe('N')
    expect(compass(90)).toBe('E')
    expect(compass(215)).toBe('SW')
    expect(compass(359)).toBe('N')
    expect(compass(-45)).toBe('NW')
    expect(compass(null)).toBe('')
    expect(compass(NaN)).toBe('')
  })

  it('shows whole degrees and a dash for an unknown figure', () => {
    expect(tempText(31.4)).toBe('31°')
    expect(tempText(-0.4)).toBe('0°')
    expect(tempText(null)).toBe('—')
    expect(tempText(undefined)).toBe('—')
  })

  it('writes the wind in km/h or mph, with its direction when there is one', () => {
    expect(windText(9.7, 215, 'metric')).toBe('10 km/h SW')
    expect(windText(9.7, 215, 'imperial')).toBe('10 mph SW')
    expect(windText(5, null, 'metric')).toBe('5 km/h')
    expect(windText(null, 90, 'metric')).toBe('')
  })

  it('names the days from the place\'s own calendar, not this computer\'s time zone', () => {
    expect(dayName('2026-11-18', 0)).toBe('Today')
    expect(dayName('2026-11-19', 1)).toBe('Thu')
    expect(dayName('2026-11-22', 4)).toBe('Sun')
    expect(dayName('', 2)).toBe('')
    expect(dayName('soon', 2)).toBe('')
  })

  it('writes a place on one line', () => {
    expect(placeLine({ name: 'Ipoh', region: 'Perak, Malaysia' })).toBe('Ipoh, Perak, Malaysia')
    expect(placeLine({ name: 'My location', region: '' })).toBe('My location')
    expect(placeLine(null)).toBe('')
  })

  it('tells day from night only for the clear and partly cloudy skies', () => {
    expect(mood('clear', true)).toBe('clear-day')
    expect(mood('clear', false)).toBe('clear-night')
    expect(mood('partly', false)).toBe('partly-night')
    expect(mood('rain', false)).toBe('rain')
    expect(mood('', true)).toBe('cloud')
  })

  it('explains why the location could not be found', () => {
    expect(hereMessage(1)).toMatch(/not allowed/)
    expect(hereMessage(3)).toMatch(/too long/)
    expect(hereMessage(2)).toMatch(/[Ss]earch for a city/)
    expect(hereMessage(undefined)).toMatch(/[Ss]earch for a city/)
  })

  it('describes the weather on hover and leaves out what is unknown', () => {
    const reading = { place: { name: 'Kuala Lumpur', region: 'Malaysia' }, temp: 31.4, label: 'Thunderstorm', feels: 37.2, humidity: 78, wind: 9.7, windDir: 215, units: 'metric' }
    expect(weatherTitle(reading)).toBe('Kuala Lumpur, Malaysia · 31° Thunderstorm · feels like 37° · humidity 78% · wind 10 km/h SW')
    expect(weatherTitle({ place: { name: 'Ipoh', region: '' }, temp: 30, label: 'Clear sky', units: 'metric' })).toBe('Ipoh · 30° Clear sky')
    expect(weatherTitle(null)).toBe('')
  })
})
