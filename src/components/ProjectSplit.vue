<script setup>
import { computed, ref, watch } from 'vue'
import { projectSplitApply, projectSplitProposals } from '../api/castle.js'

// The one-time split: projects named like "Castle · Update chip" only look grouped. Here the
// member ticks the ones to turn into real sub-projects (the parent is made when it is not there
// yet). Nothing is renamed without a tick. "Not now" hides it until the grouped names change.
const props = defineProps({ stamp: { type: Number, default: 0 } })
const emit = defineEmits(['applied'])

const KEY = 'castle.projects.splitDismissed'
const found = ref([])
const ticked = ref(new Set())
const opened = ref(false)
const busy = ref(false)
const problem = ref('')
const dismissed = ref(read())

function read() {
  try {
    return localStorage.getItem(KEY) ?? ''
  } catch {
    return ''
  }
}

// what the proposal is made of, so a new grouped name brings it back
const signature = computed(() => found.value.flatMap((s) => s.items.map((i) => i.id)).join(','))
const count = computed(() => found.value.reduce((n, s) => n + s.items.length, 0))
const show = computed(() => count.value > 0 && dismissed.value !== signature.value)

async function load() {
  try {
    found.value = await projectSplitProposals()
  } catch {
    found.value = []
  }
  // everything ticked to begin with: the member takes off what should stay as it is
  ticked.value = new Set(found.value.flatMap((s) => s.items.map((i) => i.id)))
}

function flip(id) {
  const next = new Set(ticked.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  ticked.value = next
}

function later() {
  dismissed.value = signature.value
  try {
    localStorage.setItem(KEY, signature.value)
  } catch {
    // the browser keeps nothing here; it hides until the room is opened again
  }
  opened.value = false
}

async function apply() {
  busy.value = true
  problem.value = ''
  try {
    await projectSplitApply([...ticked.value])
    opened.value = false
    await load()
    emit('applied')
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
  busy.value = false
}

watch(() => props.stamp, load, { immediate: true })
</script>

<template>
  <div v-if="show" class="split">
    <button v-if="!opened" class="lead" @click="opened = true">
      {{ count }} project{{ count === 1 ? '' : 's' }} look grouped by name. <span class="go">Make sub-projects…</span>
    </button>
    <div v-else class="box" role="group" aria-labelledby="split-h">
      <p id="split-h"><strong>Turn grouped names into sub-projects?</strong> Tick the ones to move. Each goes under its parent with the short name.</p>
      <div v-for="s in found" :key="s.parent" class="group">
        <div class="parent">{{ s.parent }} <span v-if="!s.parentExists" class="dim">(new parent)</span></div>
        <label v-for="i in s.items" :key="i.id" class="item">
          <input type="checkbox" :checked="ticked.has(i.id)" @change="flip(i.id)" />
          <span>{{ i.newName }}</span>
        </label>
      </div>
      <div class="row">
        <button class="primary" :disabled="busy || !ticked.size" @click="apply">Make {{ ticked.size }} sub-project{{ ticked.size === 1 ? '' : 's' }}</button>
        <button :disabled="busy" @click="later">Not now</button>
      </div>
      <p v-if="problem" class="problem">{{ problem }}</p>
    </div>
  </div>
</template>

<style scoped>
.split {
  font-size: 13px;
}
.lead {
  width: 100%;
  padding: 8px 10px;
  border: 1px dashed var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text-dim);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.lead:hover {
  border-color: var(--accent);
}
.go {
  color: var(--accent);
}
.box {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: color-mix(in srgb, var(--hover) 60%, transparent);
}
.box p {
  margin: 0;
}
.group {
  display: grid;
  gap: 2px;
}
.parent {
  font-weight: 600;
}
.item {
  display: flex;
  gap: 8px;
  align-items: center;
  padding-left: 12px;
  cursor: pointer;
}
.item input {
  accent-color: var(--good);
}
.dim {
  font-weight: 400;
  color: var(--text-dim);
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.row button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--hover);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.row button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--background, #050b1f);
  font-weight: 600;
}
.row button:disabled {
  opacity: 0.5;
  cursor: default;
}
button:focus-visible,
input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.problem {
  color: var(--warn);
}
</style>
