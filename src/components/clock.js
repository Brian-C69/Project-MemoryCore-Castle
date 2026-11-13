export function formatClock(date, format = '24h') {
  const hour = date.getHours()
  const minute = String(date.getMinutes()).padStart(2, '0')
  return format === '12h' ? `${hour % 12 || 12}:${minute} ${hour < 12 ? 'AM' : 'PM'}` : `${String(hour).padStart(2, '0')}:${minute}`
}
