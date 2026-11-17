<script setup>
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { meterClose, onMeterTrip } from '../api/castle.js'
import { ownerTag, sizeLabel, tripHeadline } from './meterWatch.js'

// The trip card: when CPU or RAM stays above the gear's value, the watch tidies RAM by itself and
// sends what it found here - the five busiest programs, with Close only on ordinary apps, asked once.
// It shows on any tab, closes with x, and leaves by itself after two minutes untouched. A new trip
// replaces it.
const LINGER_MS = 2 * 60 * 1000

const trip = ref(null)
const asking = ref(null)
const note = ref('')
const closed = ref({})
let stop
let timer

function arm() {
  clearTimeout(timer)
  timer = setTimeout(dismiss, LINGER_MS)
}

function dismiss() {
  clearTimeout(timer)
  trip.value = null
  asking.value = null
  note.value = ''
  closed.value = {}
}

function ask(eater) {
  asking.value = eater.pid
  arm()
}

async function close(eater) {
  arm()
  try {
    await meterClose(eater.pid, eater.name)
    closed.value = { ...closed.value, [eater.pid]: true }
    note.value = `${eater.name} closed.`
  } catch (reason) {
    note.value = String(reason)
  } finally {
    asking.value = null
  }
}

onMounted(async () => {
  stop = await onMeterTrip((payload) => {
    trip.value = payload
    asking.value = null
    note.value = ''
    closed.value = {}
    arm()
  })
})
onBeforeUnmount(() => {
  stop?.()
  clearTimeout(timer)
})
</script>

<template>
  <aside v-if="trip" class="meter-alert" role="status" @pointerenter="arm">
    <header>
      <span class="dot" aria-hidden="true"></span>
      <strong>{{ tripHeadline(trip) }}</strong>
      <button class="x" type="button" aria-label="Dismiss" @click="dismiss">×</button>
    </header>
    <ul>
      <li v-for="e in trip.top" :key="e.pid" :class="{ gone: closed[e.pid] }">
        <span class="name" :title="`pid ${e.pid}`">{{ e.name }}</span>
        <span class="num">{{ e.cpu }}%</span>
        <span class="num">{{ sizeLabel(e.memMb) }}</span>
        <span v-if="ownerTag(e.owner)" class="tag">{{ ownerTag(e.owner) }}</span>
        <span v-else-if="closed[e.pid]" class="tag">closed</span>
        <template v-else-if="e.closable">
          <span v-if="asking === e.pid" class="ask">
            Close {{ e.name }}? Unsaved work in it is lost.
            <button type="button" class="yes" @click="close(e)">Close</button>
            <button type="button" @click="asking = null">Keep</button>
          </span>
          <button v-else type="button" class="close" @click="ask(e)">Close</button>
        </template>
      </li>
    </ul>
    <p v-if="note" class="note">{{ note }}</p>
  </aside>
</template>

<style scoped>
/* Bottom-left, so it never sits on the notices in the bottom-right corner. */
.meter-alert {
  position: fixed;
  left: 24px;
  bottom: 24px;
  z-index: 70;
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 400px;
  max-width: calc(100vw - 48px);
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
  box-shadow: 0 12px 34px rgba(0, 0, 0, 0.4);
  color: var(--text);
  font-size: 13px;
}

header {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

header strong {
  flex: 1;
  font-weight: 600;
  line-height: 1.4;
}

.dot {
  flex: none;
  width: 9px;
  height: 9px;
  margin-top: 5px;
  border-radius: 50%;
  background: var(--warn, #ffb37a);
  box-shadow: 0 0 8px var(--warn, #ffb37a);
}

.x {
  padding: 0 4px;
  border: 0;
  background: none;
  color: var(--text-dim);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
}

ul {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin: 0;
  padding: 0;
  list-style: none;
}

li {
  display: grid;
  grid-template-columns: 1fr 52px 64px auto;
  align-items: center;
  gap: 8px;
}

li.gone {
  opacity: 0.45;
}

.name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.num {
  color: var(--text-dim);
  font-size: 12px;
  text-align: right;
}

.tag {
  font-size: 11px;
  color: var(--text-dim);
}

.ask {
  grid-column: 1 / -1;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}

li button,
.ask button {
  padding: 3px 10px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.ask .yes,
.close:hover {
  border-color: var(--warn, #ffb37a);
  color: var(--warn, #ffb37a);
}

.note {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}
</style>
