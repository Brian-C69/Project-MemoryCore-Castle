import { createApp } from 'vue'
import App from './App.vue'

// Browser keys that would reload, print or search the page. Keys the terminal uses
// are handled there first, so only keys nothing else wanted are stopped here.
const BLOCKED = new Set(['F5', 'ctrl+r', 'ctrl+shift+r', 'ctrl+p', 'ctrl+f', 'ctrl+g', 'ctrl+u', 'F3', 'F7'])

window.addEventListener('keydown', (event) => {
  if (event.defaultPrevented) return
  const name = event.key.length === 1 ? event.key.toLowerCase() : event.key
  const combo = [event.ctrlKey && 'ctrl', event.shiftKey && 'shift', name].filter(Boolean).join('+')
  if (BLOCKED.has(combo) || BLOCKED.has(event.key)) event.preventDefault()
})

// A WORLD window on another screen is told what it is before the page loads (world.rs):
// { view: 'world', mode: '3d' | '2d', screen }.
const view = window.__CASTLE_VIEW__
if (view?.view === 'world') {
  Object.assign(document.documentElement.style, { height: '100%', overflow: 'hidden' })
  Object.assign(document.body.style, { margin: '0', height: '100%', overflow: 'hidden', background: '#040817' })
  import('./world2d/WorldWindow.vue').then(({ default: WorldWindow }) => createApp(WorldWindow, { initial: view }).mount('#app'))
} else {
  createApp(App).mount('#app')
}
