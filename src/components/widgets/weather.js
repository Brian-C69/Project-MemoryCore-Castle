// The words and figures of HOME's WEATHER widget, kept apart from the card so they can be tested.

const POINTS = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW']

// The compass point the wind comes from, for a direction in degrees.
export function compass(degrees) {
  if (degrees === null || degrees === undefined || !Number.isFinite(degrees)) return ''
  return POINTS[Math.round((((degrees % 360) + 360) % 360) / 45) % 8]
}

// A temperature as the card shows it: whole degrees, "—" when the service left it out.
export function tempText(value) {
  return Number.isFinite(value) ? `${Math.round(value)}°` : '—'
}

// The wind: "10 km/h SW" (mph for imperial). Empty when there is no figure.
export function windText(speed, degrees, units) {
  if (!Number.isFinite(speed)) return ''
  const unit = units === 'imperial' ? 'mph' : 'km/h'
  return [`${Math.round(speed)} ${unit}`, compass(degrees)].filter(Boolean).join(' ')
}

// The name of a forecast day: "Today", then "Thu", "Fri". The date is read as the place's own
// calendar day, not shifted by this computer's time zone.
export function dayName(date, index) {
  if (index === 0) return 'Today'
  const parts = String(date ?? '').split('-').map(Number)
  if (parts.length !== 3 || parts.some((part) => !Number.isFinite(part))) return ''
  return new Date(parts[0], parts[1] - 1, parts[2]).toLocaleDateString('en-US', { weekday: 'short' })
}

// A place on one line: "Kuala Lumpur, Malaysia".
export function placeLine(place) {
  if (!place?.name) return ''
  return place.region ? `${place.name}, ${place.region}` : place.name
}

// The card's mood, so the theme-coloured glow can follow the sky.
export function mood(icon, isDay) {
  if (icon === 'clear' || icon === 'partly') return isDay ? `${icon}-day` : `${icon}-night`
  return icon || 'cloud'
}

// What to say when the computer could not tell where it is (the codes of a geolocation error).
export function hereMessage(code) {
  if (code === 1) return 'Location was not allowed. You can allow it for Castle in Windows (Settings > Privacy > Location), or search for a city instead.'
  if (code === 3) return 'Finding your location took too long. Try again, or search for a city instead.'
  return 'This computer could not tell where it is. Search for a city instead.'
}

// The hover text of the card: the sentence a screen reader or the pointer gets.
export function weatherTitle(reading) {
  if (!reading) return ''
  const bits = [placeLine(reading.place), `${tempText(reading.temp)} ${reading.label}`]
  if (Number.isFinite(reading.feels)) bits.push(`feels like ${tempText(reading.feels)}`)
  if (Number.isFinite(reading.humidity)) bits.push(`humidity ${reading.humidity}%`)
  const wind = windText(reading.wind, reading.windDir, reading.units)
  if (wind) bits.push(`wind ${wind}`)
  return bits.join(' · ')
}
