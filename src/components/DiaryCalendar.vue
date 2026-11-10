<script setup>
// The DIARY room as a calendar: one month at a time, the days that hold a page marked, the
// chosen day's pages listed underneath. It only chooses a page; the room shows it.
import { computed, ref, watch } from 'vue'
import { calendarWeeks, longDate, monthsOf, startMonth, stepMonth, undatedOf } from './diaryviews.js'

const props = defineProps({
  shelves: { type: Array, default: () => [] },
  // the page the room is showing: { title, file, date }
  current: { type: Object, default: null },
})
const emit = defineEmits(['open'])

const WEEKDAYS = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su']

function today() {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

const month = ref(startMonth(props.shelves, props.current?.date, today()))
const chosen = ref(props.current?.date ?? null)

const weeks = computed(() => calendarWeeks(month.value, props.shelves))
const months = computed(() => monthsOf(props.shelves))
const undated = computed(() => undatedOf(props.shelves))
const monthLabel = computed(() => new Date(`${month.value}-01T00:00:00`).toLocaleDateString('en-US', { month: 'long', year: 'numeric' }))
const dayPages = computed(() => weeks.value.flat().find((cell) => cell?.date === chosen.value)?.pages ?? [])
// the arrows stop where the diary does (a month either side of its first and last page)
const canGoBack = computed(() => !months.value.length || month.value > months.value[0])
const canGoOn = computed(() => !months.value.length || month.value < months.value[months.value.length - 1])

function pick(cell) {
  if (!cell) return
  chosen.value = cell.date
  if (cell.pages.length) emit('open', cell.pages[0])
}

// the room opened another page (a HOME card, the first load): show its month and day
watch(
  () => props.current?.file,
  () => {
    const date = props.current?.date
    if (date) {
      month.value = date.slice(0, 7)
      chosen.value = date
    }
  },
)
// the first shelves arrive after the room mounts
watch(
  () => props.shelves,
  () => {
    if (!props.current?.date && !chosen.value) month.value = startMonth(props.shelves, null, today())
  },
)
</script>

<template>
  <div class="calendar">
    <div class="bar">
      <button :disabled="!canGoBack" aria-label="Earlier month" @click="month = stepMonth(month, -1)">‹</button>
      <strong>{{ monthLabel }}</strong>
      <button :disabled="!canGoOn" aria-label="Later month" @click="month = stepMonth(month, 1)">›</button>
    </div>
    <div class="grid head">
      <span v-for="d in WEEKDAYS" :key="d">{{ d }}</span>
    </div>
    <div v-for="(week, i) in weeks" :key="i" class="grid">
      <template v-for="(cell, j) in week" :key="j">
        <span v-if="!cell" class="cell empty" />
        <button
          v-else
          class="cell"
          :class="{ has: cell.pages.length, chosen: cell.date === chosen, today: cell.date === today() }"
          :disabled="!cell.pages.length"
          :title="cell.pages.map((p) => p.title).join(' · ')"
          @click="pick(cell)"
        >
          {{ cell.day }}
          <span v-if="cell.pages.length" class="dots">{{ '•'.repeat(Math.min(3, cell.pages.length)) }}</span>
        </button>
      </template>
    </div>

    <div v-if="dayPages.length" class="day">
      <h3>{{ longDate(chosen) }}</h3>
      <button v-for="page in dayPages" :key="page.file" class="entry" :class="{ active: current?.file === page.file }" @click="emit('open', page)">
        {{ page.title }}
      </button>
    </div>
    <p v-else class="hint">Pick a marked day to read it.</p>

    <div v-if="undated.length" class="day">
      <h3>Earlier</h3>
      <button v-for="page in undated" :key="page.file" class="entry" :class="{ active: current?.file === page.file }" @click="emit('open', page)">
        {{ page.title }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.calendar {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 4px 0 6px;
  font-size: 15px;
}
.bar button {
  width: 32px;
  height: 32px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 18px;
  cursor: pointer;
}
.bar button:disabled {
  opacity: 0.3;
  cursor: default;
}
.grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 4px;
}
.head span {
  text-align: center;
  font-size: 11px;
  letter-spacing: 0.08em;
  color: var(--text-dim);
}
.cell {
  position: relative;
  aspect-ratio: 1;
  display: grid;
  place-items: center;
  border: 1px solid transparent;
  border-radius: 8px;
  background: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 13px;
}
.cell.has {
  border-color: var(--line);
  color: var(--text);
  background: var(--hover);
  cursor: pointer;
}
.cell.has:hover,
.cell.chosen {
  border-color: var(--accent);
}
.cell.today {
  font-weight: 700;
}
.dots {
  position: absolute;
  bottom: 1px;
  font-size: 9px;
  line-height: 1;
  color: var(--accent);
}
.day {
  display: grid;
  gap: 4px;
  margin-top: 10px;
}
h3 {
  margin: 0 0 2px;
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.08em;
  color: var(--text-dim);
}
.entry {
  padding: 7px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 14px;
  text-align: left;
  cursor: pointer;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.entry:hover {
  background: var(--hover);
}
.entry.active {
  border-color: var(--accent);
  background: var(--hover);
}
.hint {
  margin: 10px 0 0;
  font-size: 13px;
  color: var(--text-dim);
}
</style>
