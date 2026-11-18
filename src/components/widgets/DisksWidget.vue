<script setup>
// HOME's DISKS card: every drive of this computer with how much room is left, as a bar that turns
// amber at 85% and red at 95%. Read only (Sweep is the card that frees space). It looks again every
// half minute while Castle is on screen.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { diskDrives } from '../../api/castle.js'
import { driveName, freeLine, tone, totalsLine, usedPercent } from './disks.js'

// Widgets are handed the whole data bundle; only declared props are used.
defineOptions({ inheritAttrs: false })

const drives = ref([])
const loaded = ref(false)
let timer

const rows = computed(() =>
  drives.value.map((d) => {
    const percent = usedPercent(d.total, d.free)
    const extra = [d.label, d.media, d.removable ? 'removable' : ''].filter(Boolean).join(' · ')
    return { key: d.mount, name: driveName(d.mount), extra, percent, tone: tone(percent), line: freeLine(d), title: `${d.mount} - ${percent}% used` }
  }),
)
const summary = computed(() => (drives.value.length > 1 ? totalsLine(drives.value) : ''))

async function read() {
  if (document.visibilityState !== 'visible') return
  try {
    drives.value = await diskDrives()
  } catch {
    // the last list stays
  }
  loaded.value = true
}

onMounted(() => {
  read()
  timer = setInterval(read, 30000)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <div class="widget disks">
    <div class="head">
      <span class="title">DISKS</span>
      <span v-if="summary" class="summary">{{ summary }}</span>
    </div>
    <p v-if="loaded && !rows.length" class="none">No drives found.</p>
    <ul v-else class="drives">
      <li v-for="row in rows" :key="row.key" :title="row.title">
        <div class="top">
          <strong>{{ row.name }}</strong>
          <span v-if="row.extra" class="extra">{{ row.extra }}</span>
          <span class="percent" :class="row.tone">{{ row.percent }}%</span>
        </div>
        <div
          class="bar"
          role="progressbar"
          :aria-valuenow="row.percent"
          aria-valuemin="0"
          aria-valuemax="100"
          :aria-label="`${row.name} ${row.percent}% used`"
        >
          <span class="fill" :class="row.tone" :style="{ width: row.percent + '%' }"></span>
        </div>
        <span class="line">{{ row.line }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.disks {
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
.summary {
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
.none {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.drives {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 260px;
  margin: 0;
  padding: 0 4px 0 0;
  overflow-y: auto;
  list-style: none;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: var(--scrollbar-width);
}
li {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.top {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}
.top strong {
  flex: none;
  font-size: 14px;
}
.extra {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-dim);
}
.percent {
  flex: none;
  margin-left: auto;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
.percent.warn { color: var(--warn, #ffb37a); }
.percent.full { color: var(--bad, #ff7a7a); }
.bar {
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--text) 12%, transparent);
}
.fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
  transition: width 0.4s ease;
}
.fill.warn { background: var(--warn, #ffb37a); }
.fill.full { background: var(--bad, #ff7a7a); }
.line {
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}
</style>
