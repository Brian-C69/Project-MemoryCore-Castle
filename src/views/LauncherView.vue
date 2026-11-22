<script setup>
// APPS: every room in one place. A search box, then a grid of large tiles in quiet groups; the pin on a
// tile puts the room in the top bar. Keyboard: type to filter, Enter opens the first match, the arrow keys
// move between tiles, P pins or unpins the focused tile, Esc clears the search and then closes APPS.
// (The room of Start-menu apps is called DESKTOP; its id is `apps`. This room's id is `launcher`.)
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import LauncherSearch from '../components/launcher/LauncherSearch.vue'
import RoomTile from '../components/launcher/RoomTile.vue'
import HelpTip from '../components/HelpTip.vue'
import { HINTS, TIP_LEAVE_MS, filterGroups, firstMatch, groupRooms, moveFocus, tipDelay } from '../launcher.js'

const props = defineProps({
  // the rooms the member has available, in their tab order
  tabs: { type: Array, required: true },
  pins: { type: Array, default: () => [] },
  // what is waiting in each room: { roomId: { count, tone, text } }
  badges: { type: Object, default: () => ({}) },
  current: { type: String, default: '' },
  // opened with Ctrl+K: the cursor goes to the search box; opened any other way it does not
  searchOnOpen: { type: Boolean, default: false },
})
const emit = defineEmits(['open', 'pin', 'close'])
const query = ref('')
const search = ref(null)
const root = ref(null)
// One flat grid like a phone's home screen: the groups still decide the ORDER (Companion,
// Work, Create, Connect, System) but are not drawn, so the keyboard sees a single run of tiles.
const shown = computed(() => {
  const all = filterGroups(groupRooms(props.tabs), query.value).flatMap((g) => g.tabs)
  return all.length ? [{ id: 'all', label: 'All rooms', tabs: all }] : []
})
const sizes = computed(() => shown.value.map((g) => g.tabs.length))

// the page itself takes focus (no ring), not the search box: keys still work, typing moves to the search
onMounted(() => nextTick(() => (props.searchOnOpen ? search.value : root.value)?.focus({ preventScroll: true })))
defineExpose({ focusSearch: () => search.value?.focus() })

function columns() {
  const grid = root.value?.querySelector('.grid')
  return grid ? Math.max(1, getComputedStyle(grid).gridTemplateColumns.split(' ').length) : 1
}
function focusTile(at) {
  if (!at) return
  root.value?.querySelector(`.open[data-g="${at.g}"][data-i="${at.i}"]`)?.focus()
}
function tileOf(el) {
  const open = el?.closest?.('.open')
  return open ? { g: Number(open.dataset.g), i: Number(open.dataset.i) } : null
}
function idAt(at) {
  return shown.value[at.g]?.tabs[at.i]?.id
}

// The tile tooltip. One bubble for the whole grid. The first one waits a moment; while the pointer stays on
// the tiles, the first two wait a moment and from the third tile on they show at once ("tooltip mode");
// the count starts again when the pointer has left the tiles.
const tip = ref(null)
let tipsShown = 0   // tips shown since the pointer last came onto the tiles
let showTimer = 0
let leaveTimer = 0
function pretty(label) {
  return label.toLowerCase().replace(/(^|\s)\S/g, (c) => c.toUpperCase())
}
function showTip(open, now = false) {
  clearTimeout(showTimer)
  clearTimeout(leaveTimer)
  const at = tileOf(open)
  const tab = at && shown.value[at.g]?.tabs[at.i]
  if (!tab) return
  const place = () => {
    const r = open.getBoundingClientRect()
    const host = root.value.getBoundingClientRect()
    const half = 130
    const x = Math.min(Math.max(r.left + r.width / 2 - host.left, half), host.width - half)
    tip.value = { text: [HINTS[tab.id], props.badges[tab.id]?.text].filter(Boolean).join(' · ') || pretty(tab.label), x, y: r.bottom - host.top + root.value.scrollTop + 6 }
    tipsShown += 1
  }
  const wait = now ? 0 : tipDelay(tipsShown)
  if (wait === 0) place()
  else showTimer = setTimeout(place, wait)
}
function hideTip() {
  clearTimeout(showTimer)
  tip.value = null
  clearTimeout(leaveTimer)
  leaveTimer = setTimeout(() => { tipsShown = 0 }, TIP_LEAVE_MS)
}
function onOver(e) {
  const open = e.target.closest?.('.open')
  if (open) showTip(open)
}
function onOut(e) {
  if (!e.target.closest?.('.open')) return
  if (!e.relatedTarget?.closest?.('.open')) hideTip()
}
function onFocusIn(e) {
  const open = e.target.closest?.('.open')
  if (open && e.target.matches(':focus-visible')) showTip(open, true)
}
onBeforeUnmount(() => { clearTimeout(showTimer); clearTimeout(leaveTimer) })

function onKey(e) {
  if (e.ctrlKey || e.metaKey || e.altKey) return
  const k = e.key
  const inSearch = e.target?.id === 'launcher-search'
  if (k === 'Escape') {
    e.preventDefault()
    if (query.value) { query.value = ''; search.value?.focus() } else emit('close')
    return
  }
  if (inSearch) {
    if (k === 'Enter') {
      const m = firstMatch(shown.value)
      if (m) { e.preventDefault(); emit('open', m.id) }
    } else if (k === 'ArrowDown') {
      e.preventDefault()
      focusTile({ g: 0, i: 0 })
    }
    return
  }
  const at = tileOf(e.target)
  if (!at) {
    if (e.target !== root.value) return
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(k)) { e.preventDefault(); focusTile({ g: 0, i: 0 }) }
    else if (k.length === 1 && /\S/.test(k)) search.value?.focus()
    return
  }
  if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(k)) {
    e.preventDefault()
    // Up from the first row goes back to the search box
    if (k === 'ArrowUp' && at.g === 0 && at.i < columns()) { search.value?.focus(); return }
    focusTile(moveFocus(sizes.value, at, k, columns()))
  } else if (k === 'p' || k === 'P') {
    const id = idAt(at)
    if (id && id !== 'home') { e.preventDefault(); emit('pin', { id, on: !props.pins.includes(id) }) }
  } else if (k.length === 1 && /\S/.test(k)) {
    // typing anywhere starts a search
    search.value?.focus()
  }
}
</script>

<template>
<section ref="root" class="launcher-room" tabindex="-1" @keydown="onKey">
 <header class="bar">
  <h2>APPS</h2>
  <HelpTip text="Ctrl+K opens APPS from anywhere and goes to the search. Type to find a room and press Enter to open the first. Esc goes back. Pin a room (or press P on it) to keep it in the top bar." />
  <LauncherSearch ref="search" v-model="query" />
 </header>
 <p v-if="!shown.length" class="none" role="status">No room called that.</p>
 <div class="groups" @pointerover="onOver" @pointerout="onOut" @focusin="onFocusIn" @focusout="hideTip" @click="hideTip">
  <section v-for="(g, gi) in shown" :key="g.id" class="group" :aria-label="g.label">
   <div class="grid">
    <RoomTile v-for="(t, ti) in g.tabs" :key="t.id" :tab="t" :group="gi" :index="ti" :hint="HINTS[t.id] || ''" :pinned="pins.includes(t.id)" :pinnable="t.id !== 'home'" :badge="badges[t.id] || null" :current="current === t.id" @open="emit('open', $event)" @pin="emit('pin', $event)" />
   </div>
  </section>
 </div>
 <div v-if="tip" class="tip" role="tooltip" :style="{ left: `${tip.x}px`, top: `${tip.y}px` }">
  {{ tip.text }}
 </div>
</section>
</template>

<style scoped>
.launcher-room {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-x: hidden;
  overflow-y: auto;
  outline: none;
  color: var(--text);
  font-size: 13px;
  background-image: radial-gradient(color-mix(in srgb, var(--text) 14%, transparent) 1px, transparent 1.2px);
  background-size: 18px 18px;
  scrollbar-width: var(--scrollbar-width);
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}
.bar { display: flex; align-items: center; gap: 10px; }
.bar h2 { margin: 0; font-size: 13px; font-weight: 500; letter-spacing: 0.2em; color: var(--accent); }
.lede { margin: 0; max-width: 60em; color: var(--text-dim); line-height: 1.5; }
.none { margin: 0; text-align: center; color: var(--text-dim); }
.groups { width: 100%; max-width: 1040px; margin: 3vh auto 0; }
.group { display: flex; flex-direction: column; gap: 6px; flex-shrink: 0; }
.grid { display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: 20px 6px; }
@media (max-width: 1100px) { .grid { grid-template-columns: repeat(6, minmax(0, 1fr)); } }
@media (max-width: 760px) { .grid { grid-template-columns: repeat(4, minmax(0, 1fr)); } }
@media (max-width: 460px) { .grid { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
.tip {
  position: absolute; z-index: 30; transform: translateX(-50%); width: max-content; max-width: 280px; pointer-events: none;
  padding: 4px 8px; border: 1px solid var(--line); border-radius: 3px;
  background: var(--panel-background); color: var(--text); font-size: 12px; line-height: 1.35;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
}
</style>
