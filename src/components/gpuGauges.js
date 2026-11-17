// Small pure helpers for the graphics-card gauges on the Gauges card.

const gb = (mb) => `${(mb / 1024).toFixed(1)} GB`

// What shows when the pointer rests on a card's gauge: its full name, how busy it is, its memory and
// its temperature (a part that is unknown is left out).
export function gpuTitle(g) {
  const parts = [g.name, `${g.util}% busy`]
  if (g.memTotalMb > 0) parts.push(`VRAM ${gb(g.memUsedMb)} of ${gb(g.memTotalMb)}`)
  if (g.tempC !== null && g.tempC !== undefined) parts.push(`${g.tempC}°C`)
  return parts.join(' · ')
}
