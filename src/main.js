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

// A window on another screen is told what it is before the page loads (world.rs):
// { view: 'world', screen, lead, music, volume, config } or { view: 'identify', number }.
// Dev only: http://localhost:<port>/?world=3d/crystal-isle&camera=orbit draws one world in the
// browser, for tuning a world.json by eye (MAKE-IT-YOURS, "A world of your own").
const devWorld = import.meta.env.DEV ? new URLSearchParams(location.search).get('world') : null
const view = devWorld
  ? { view: 'world', screen: 0, lead: false, config: { world: devWorld, camera: new URLSearchParams(location.search).get('camera') || 'orbit', holdMin: 8, holdMax: 20, glide: 'mix', shots: ['follow', 'wide', 'close', 'orbit', 'crane'], seconds: 15, shuffle: true } }
  : window.__CASTLE_VIEW__
if (view?.view === 'world') {
  Object.assign(document.documentElement.style, { height: '100%', overflow: 'hidden' })
  Object.assign(document.body.style, { margin: '0', height: '100%', overflow: 'hidden', background: '#040817' })
  import('./worlds/WorldWindow.vue').then(({ default: WorldWindow }) => createApp(WorldWindow, { initial: view }).mount('#app'))
} else if (view?.view === 'identify') {
  // see-through, so only the number and its frame show over whatever is on that screen
  Object.assign(document.documentElement.style, { height: '100%', overflow: 'hidden', background: 'transparent' })
  Object.assign(document.body.style, { margin: '0', height: '100%', overflow: 'hidden', background: 'transparent' })
  import('./worlds/IdentifyNumber.vue').then(({ default: IdentifyNumber }) => createApp(IdentifyNumber, { number: Number(view.number) || 1 }).mount('#app'))
} else {
  createApp(App).mount('#app')
}
