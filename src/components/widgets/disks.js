// The DISKS card's numbers in plain words: sizes, how full a drive is, and when to warn.

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']

// "465 GB", "1.8 TB", "0 B". One decimal under 10, none above; anything unusable reads 0 B.
export function sizeText(bytes) {
  let value = Number.isFinite(bytes) && bytes > 0 ? bytes : 0
  let unit = 0
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000
    unit += 1
  }
  const text = unit > 0 && value < 10 ? value.toFixed(1).replace(/\.0$/, '') : String(Math.round(value))
  return `${text} ${UNITS[unit]}`
}

// How full a drive is, 0 to 100 (a drive that reports no size is 0).
export function usedPercent(total, free) {
  if (!(total > 0)) return 0
  const used = Math.min(total, Math.max(0, total - (Number.isFinite(free) ? free : 0)))
  return Math.round((used / total) * 100)
}

// The bar's colour: fine, getting full (85%), nearly full (95%).
export function tone(percent) {
  if (percent >= 95) return 'full'
  if (percent >= 85) return 'warn'
  return 'ok'
}

// "C:" for "C:\", the folder name for "/Volumes/Backup", "/" for the root.
export function driveName(mount) {
  const text = String(mount ?? '')
  if (/^[A-Za-z]:[\\/]?$/.test(text)) return text.slice(0, 2).toUpperCase()
  if (text === '/') return '/'
  const parts = text.split(/[\\/]/).filter(Boolean)
  return parts[parts.length - 1] ?? text
}

// "123 GB free of 465 GB"
export function freeLine(drive) {
  return `${sizeText(drive.free)} free of ${sizeText(drive.total)}`
}

// All drives together: the room left and the room there is.
export function totalsLine(drives) {
  const total = drives.reduce((sum, d) => sum + (d.total || 0), 0)
  const free = drives.reduce((sum, d) => sum + (d.free || 0), 0)
  return `${sizeText(free)} free of ${sizeText(total)}`
}
