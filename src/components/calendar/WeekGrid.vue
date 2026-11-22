<script setup>
// Seven columns with an hour grid. An all-day strip on top holds everything without a time of
// day; timed events are blocks; automation runs are thin marks, never blocks.
import { computed, nextTick, onMounted, ref } from 'vue'
import { cellItems, dayTitle, groupByDay, itemLine, itemMark, ownerRoom } from '../../calendar.js'
import { canDrag, clock, colourVar, layoutDay, minutes, movedStart } from '../../calendarEvents.js'

const HOUR = 40
const props = defineProps({
  days: { type: Array, required: true },
  items: { type: Array, required: true },
  today: { type: String, required: true },
  selected: { type: String, required: true },
})
const emit = defineEmits(['select', 'open', 'move', 'add'])
const scroller = ref(null)
const by = computed(() => groupByDay(props.items))
const strip = (date) => (by.value[date] || []).filter((i) => !(i.kind === 'event' && !i.allDay) && !(i.kind === 'automation' && i.time))
const marks = (date) => (by.value[date] || []).filter((i) => i.kind === 'automation' && i.time)
const blocks = (date) => layoutDay(by.value[date] || [])
const hours = Array.from({ length: 24 }, (_, h) => h)

onMounted(async () => {
  await nextTick()
  if (scroller.value) scroller.value.scrollTop = 7 * HOUR
})

function pointerTime(e) {
  const rect = e.currentTarget.getBoundingClientRect()
  return clock(Math.floor(((e.clientY - rect.top) / HOUR) * 4) * 15)
}
function dragStart(e, item) {
  e.dataTransfer.setData('application/x-castle-event', JSON.stringify({ id: item.id, occurrence: item.occurrence, allDay: item.allDay, time: item.time }))
  e.dataTransfer.effectAllowed = 'move'
}
function dropInto(e, date, timed) {
  const raw = e.dataTransfer.getData('application/x-castle-event')
  if (!raw) return
  let item
  try { item = JSON.parse(raw) } catch { return }
  emit('move', item, movedStart(item, date, timed ? pointerTime(e) : undefined))
}
function style(block) {
  const width = 100 / block.lanes
  return { top: `${(block.top / 60) * HOUR}px`, height: `${(block.height / 60) * HOUR - 2}px`, left: `${block.lane * width}%`, width: `calc(${width}% - 2px)`, '--tone': colourVar(block.item.colour) }
}
function markStyle(item) {
  return { top: `${((minutes(item.time) || 0) / 60) * HOUR}px` }
}
const open = (item) => emit('open', item)
</script>

<template>
  <div class="week">
    <div class="head">
      <span class="gutter"></span>
      <button v-for="d in days" :key="d" :class="['day', { today: d === today, sel: d === selected }]" @click="emit('select', d)">{{ dayTitle(d) }}</button>
    </div>
    <div class="strip">
      <span class="gutter">All day</span>
      <div v-for="d in days" :key="d" class="col" @dragover.prevent @drop.prevent="dropInto($event, d, false)">
        <template v-for="part in [cellItems(strip(d), 4)]" :key="d">
          <button v-for="(item, n) in part.shown" :key="n" :class="['entry', item.kind, item.state]" :style="{ '--tone': colourVar(item.colour) }" :draggable="canDrag(item)" :title="itemLine(item)" @dragstart="dragStart($event, item)" @click="open(item)"><b aria-hidden="true">{{ itemMark(item) }}</b> {{ itemLine(item) }}</button>
          <button v-if="part.more" class="more" @click="emit('select', d)">+{{ part.more }} more</button>
        </template>
      </div>
    </div>
    <div ref="scroller" class="scroll">
      <div class="grid" :style="{ height: `${24 * HOUR}px` }">
        <div class="gutter">
          <span v-for="h in hours" :key="h" class="hour" :style="{ top: `${h * HOUR}px` }">{{ String(h).padStart(2, '0') }}:00</span>
        </div>
        <div v-for="d in days" :key="d" :class="['col', 'timed', { today: d === today }]" @dragover.prevent @drop.prevent="dropInto($event, d, true)" @dblclick.self="emit('add', d, pointerTime($event))">
          <span v-for="h in hours" :key="h" class="line" :style="{ top: `${h * HOUR}px` }"></span>
          <span v-for="(item, n) in marks(d)" :key="'m' + n" :class="['mark', item.state]" :style="markStyle(item)" :title="itemLine(item)" @click="open(item)"></span>
          <button v-for="block in blocks(d)" :key="block.item.id + block.item.time + block.item.date" class="block" :style="style(block)" :draggable="canDrag(block.item)" @dragstart="dragStart($event, block.item)" @click="open(block.item)">
            <b>{{ block.item.time }}</b> {{ block.item.title }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.week { display: flex; flex-direction: column; min-width: 0; gap: 4px; }
.head, .strip, .grid { display: grid; grid-template-columns: 52px repeat(7, minmax(0, 1fr)); gap: 4px; }
.gutter { position: relative; font-size: 11px; color: var(--text-dim); }
.strip .gutter { align-self: start; padding-top: 4px; }
.day { padding: 6px; border: 1px solid var(--line); border-radius: 10px; background: none; color: var(--text); font: inherit; font-size: 12px; cursor: pointer; }
.day.today { border-color: var(--accent); color: var(--accent); }
.day.sel { background: color-mix(in srgb, var(--accent) 12%, transparent); }
.strip .col { display: flex; flex-direction: column; gap: 2px; min-height: 28px; padding: 2px; border: 1px solid var(--line); border-radius: 10px; min-width: 0; }
.entry, .more { display: block; width: 100%; text-align: left; padding: 1px 4px; border: 0; border-radius: 5px; background: none; color: var(--text); font: inherit; font-size: 11.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; }
.entry:hover, .more:hover { background: color-mix(in srgb, var(--accent) 12%, transparent); }
.entry b { display: inline-block; width: 1em; text-align: center; color: var(--tone, var(--accent)); }
.entry.event { border-left: 3px solid var(--tone, var(--accent)); }
.entry.done { opacity: 0.6; text-decoration: line-through; }
.entry.diary { opacity: 0.85; }
.more { color: var(--text-dim); }
.scroll { max-height: 62vh; overflow: auto; border: 1px solid var(--line); border-radius: 10px; scrollbar-width: var(--scrollbar-width); scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track); }
.grid { position: relative; padding-right: 4px; }
.grid .gutter .hour { position: absolute; right: 4px; transform: translateY(-50%); }
.grid .gutter .hour:first-child { transform: none; }
.timed { position: relative; min-width: 0; border-left: 1px solid var(--line); }
.timed.today { background: color-mix(in srgb, var(--accent) 6%, transparent); }
.line { position: absolute; left: 0; right: 0; border-top: 1px solid color-mix(in srgb, var(--line) 60%, transparent); pointer-events: none; }
.block {
  position: absolute; box-sizing: border-box; overflow: hidden; padding: 2px 5px; text-align: left;
  border: 1px solid var(--tone, var(--accent)); border-left-width: 3px; border-radius: 6px;
  background: color-mix(in srgb, var(--tone, var(--accent)) 18%, var(--panel-background)); color: var(--text); font: inherit; font-size: 11.5px; cursor: pointer;
}
.block b { font-weight: 600; margin-right: 4px; }
.mark { position: absolute; left: 0; right: 0; height: 2px; border-radius: 2px; background: color-mix(in srgb, var(--accent) 55%, transparent); cursor: pointer; }
.mark.failed { background: #ef9393; }
.mark.planned { background: none; border-top: 2px dashed color-mix(in srgb, var(--accent) 55%, transparent); height: 0; }
button:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
</style>
