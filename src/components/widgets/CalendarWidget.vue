<script setup>
// HOME's CALENDAR card: the reminders raised lately (with the ones missed while Castle was closed),
// and the next few events, so the card is never an empty box. A card like the others: drag it
// anywhere on HOME, size it in WIDGETS. It only reads what the room already loaded.
import { computed } from 'vue'
import { calendarNoticeDismiss } from '../../api/castle.js'
import { refreshCalendarCard } from '../../composables/useCalendar.js'
import { dayTitle } from '../../calendar.js'

// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })
const props = defineProps({ data: { type: Object, default: null } })
defineEmits(['open'])

const notices = computed(() => props.data?.notices || [])
const more = computed(() => props.data?.more || 0)
const next = computed(() => props.data?.next || [])
const problem = computed(() => props.data?.problem || '')
const proposed = computed(() => props.data?.proposed || 0)

async function dismiss(id) {
  try { await calendarNoticeDismiss(id) } catch { /* the card refreshes either way */ }
  refreshCalendarCard()
}
const when = (item) => `${dayTitle(item.date)}${item.time ? ` ${item.time}` : ''}`
</script>

<template>
  <div class="widget calendar-card">
    <div class="head">
      <span class="title">CALENDAR</span>
      <button v-if="notices.length || more" type="button" class="link" @click="dismiss(null)">Dismiss all</button>
    </div>
    <p v-if="problem" class="bad" role="alert">{{ problem }}</p>
    <ul v-if="notices.length" class="notes">
      <li v-for="n in notices" :key="n.id" :class="{ late: n.late }">
        <p class="line">{{ n.text }}</p>
        <button type="button" class="x" :aria-label="`Dismiss ${n.title}`" @click="dismiss(n.id)">&times;</button>
      </li>
    </ul>
    <p v-if="proposed" class="dim">{{ proposed }} suggested event{{ proposed === 1 ? '' : 's' }} waiting for your yes.</p>
    <p v-if="more" class="dim">+{{ more }} more reminder{{ more === 1 ? '' : 's' }} while Castle was closed.</p>
    <div v-if="next.length" class="next">
      <span class="sub">Coming up</span>
      <ul class="plain">
        <li v-for="(item, i) in next" :key="i" :title="item.title"><span class="what">{{ item.title }}</span><span class="at">{{ when(item) }}</span></li>
      </ul>
    </div>
    <p v-else-if="!notices.length && !more && !problem && !proposed" class="dim">Nothing is coming up in the next 7 days.</p>
    <button type="button" class="open" @click="$emit('open')">Open CALENDAR</button>
  </div>
</template>

<style scoped>
.calendar-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 250px;
  max-width: 300px;
  padding: 14px 16px;
}
.head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
}
.title {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}
.sub {
  font-size: 11px;
  letter-spacing: 0.1em;
  color: var(--text-dim);
}
.dim {
  margin: 0;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-dim);
}
.bad {
  margin: 0;
  font-size: 12px;
  color: #ef9393;
}
ul {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.notes {
  max-height: 220px;
  overflow-y: auto;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: var(--scrollbar-width);
}
.notes li {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
  padding-left: 8px;
  border-left: 2px solid var(--accent);
}
.notes li.late {
  border-left-color: color-mix(in srgb, var(--accent) 40%, var(--text-dim));
}
.line {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.4;
  overflow-wrap: anywhere;
}
.x, .link {
  flex: none;
  border: 0;
  background: none;
  color: var(--text-dim);
  font: inherit;
  cursor: pointer;
}
.x { font-size: 16px; line-height: 1; padding: 0 2px; }
.link { font-size: 11px; }
.x:hover, .link:hover { color: var(--text); }
.next { display: flex; flex-direction: column; gap: 4px; }
.plain li {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  font-size: 12.5px;
}
.what {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.at {
  flex: none;
  color: var(--text-dim);
  font-size: 11.5px;
}
.open {
  align-self: flex-start;
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.open:hover { border-color: color-mix(in srgb, var(--accent) 45%, var(--line)); }
.open:focus-visible, .x:focus-visible, .link:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
