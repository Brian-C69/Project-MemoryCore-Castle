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

createApp(App).mount('#app')
