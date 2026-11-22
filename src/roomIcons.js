// Trusted, bundled artwork only. Inline SVG lets currentColor inherit Castle's live --accent.
const icons = import.meta.glob('./assets/rooms/*.svg', { eager: true, query: '?raw', import: 'default' })
export function roomIcon(id) {
  return icons[`./assets/rooms/${id}.svg`] || ''
}
