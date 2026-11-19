<script setup>
import { computed, ref, watch } from 'vue'
import { projectState, projectTaskAdd, projectTaskDone, projectTaskOrder, projectTaskRemove } from '../api/castle.js'
import { progressLine, reorder, stateLine } from './projectshelf.js'

// Under an open project in the PROJECTS room: its parent (a way back up), how far its tasks are,
// its sub-projects as cards, and its own tasks. The member ticks, adds, removes and drags tasks
// here; the companion does the same with `castle-memory task add | done | undo | remove`.
const props = defineProps({
  file: { type: String, required: true }, // db/project/<id>.md
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['open', 'changed'])

const state = ref(null)
const problem = ref('')
const title = ref('')
const due = ref('')
const busy = ref(false)
const dragging = ref(-1)
const over = ref(-1)

const line = computed(() => (state.value ? progressLine(state.value.done, state.value.total) : ''))
const pct = computed(() => (state.value?.total ? Math.round((state.value.done / state.value.total) * 100) : 0))

async function load() {
  try {
    state.value = await projectState(props.file)
    problem.value = ''
  } catch (reason) {
    state.value = null
    problem.value = String(reason?.message ?? reason)
  }
}

async function act(work) {
  busy.value = true
  problem.value = ''
  try {
    await work()
    await load()
    emit('changed')
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  }
  busy.value = false
}

function add() {
  const t = title.value.trim()
  if (!t) return
  act(async () => {
    await projectTaskAdd(props.file, t, due.value || null)
    title.value = ''
    due.value = ''
  })
}

const tick = (task, done) => act(() => projectTaskDone(task.id, done))
const remove = (task) => act(() => projectTaskRemove(task.id))

function place(from, to) {
  const ids = state.value.tasks.map((t) => t.id)
  const next = reorder(ids, from, to)
  if (next.join() === ids.join()) return
  // drawn in the new order at once, then saved
  state.value.tasks = next.map((id) => state.value.tasks.find((t) => t.id === id))
  act(() => projectTaskOrder(props.file, next))
}

function drop(to) {
  if (dragging.value >= 0) place(dragging.value, to)
  dragging.value = -1
  over.value = -1
}

// Alt+↑ / Alt+↓ moves a task for the keyboard
function key(event, i) {
  if (!event.altKey) return
  if (event.key === 'ArrowUp' && i > 0) {
    event.preventDefault()
    place(i, i - 1)
  } else if (event.key === 'ArrowDown' && i < state.value.tasks.length - 1) {
    event.preventDefault()
    place(i, i + 1)
  }
}

function dueLabel(d) {
  if (!d) return ''
  const day = new Date(`${d}T00:00:00`)
  return Number.isNaN(day.getTime()) ? d : `by ${day.toLocaleDateString('en-US', { day: 'numeric', month: 'short' })}`
}

watch(() => [props.file, props.stamp], load, { immediate: true })
</script>

<template>
  <div v-if="state" class="project-tasks">
    <nav v-if="state.parent" class="crumb" aria-label="Parent project">
      <button @click="emit('open', state.parent.file)">{{ state.parent.name }}</button>
      <span aria-hidden="true">›</span>
      <span>{{ state.name }}</span>
    </nav>

    <div v-if="state.total" class="progress-block">
      <div class="meta">{{ line }}</div>
      <div class="progress" role="progressbar" :aria-valuenow="pct" aria-valuemin="0" aria-valuemax="100" :aria-label="line"><i :style="{ width: `${pct}%` }"></i></div>
    </div>

    <section v-if="state.kids.length" class="section">
      <h3>Sub-projects</h3>
      <div class="kids">
        <button v-for="k in state.kids" :key="k.file" class="kid" :class="k.shelf" @click="emit('open', k.file)">
          <b>{{ k.name }}</b>
          <span class="meta"><span class="dot"></span>{{ k.shelf === 'active' ? k.status || 'Active' : stateLine({ shelf: k.shelf }) }}<template v-if="k.total"> · {{ k.done }}/{{ k.total }}</template></span>
          <span v-if="k.total" class="progress"><i :style="{ width: `${Math.round((k.done / k.total) * 100)}%` }"></i></span>
        </button>
      </div>
    </section>

    <section class="section">
      <h3>Tasks</h3>
      <ul v-if="state.tasks.length" class="tasks">
        <li
          v-for="(t, i) in state.tasks"
          :key="t.id"
          :class="{ done: t.done, over: over === i && dragging !== i }"
          draggable="true"
          @dragstart="dragging = i"
          @dragover.prevent="over = i"
          @dragleave="over = -1"
          @drop.prevent="drop(i)"
          @dragend="dragging = -1; over = -1"
        >
          <span class="grip" aria-hidden="true" title="Drag to reorder (or Alt+↑ / Alt+↓)">⋮⋮</span>
          <input :id="`task-${t.id}`" type="checkbox" :checked="t.done" :disabled="busy" @change="tick(t, $event.target.checked)" @keydown="key($event, i)" />
          <label :for="`task-${t.id}`">{{ t.title }}</label>
          <span class="meta">{{ dueLabel(t.due) }}</span>
          <button class="remove" :disabled="busy" :aria-label="`Remove ${t.title}`" title="Remove" @click="remove(t)">×</button>
        </li>
      </ul>
      <p v-else class="meta">No tasks yet.</p>
      <form class="add" @submit.prevent="add">
        <input v-model="title" maxlength="300" placeholder="Add a task and press Enter" aria-label="New task" />
        <input v-model="due" type="date" aria-label="Due date (optional)" title="Due date (optional)" />
        <button type="submit" :disabled="busy || !title.trim()">Add</button>
      </form>
    </section>
    <p v-if="problem" class="problem">{{ problem }}</p>
  </div>
  <p v-else-if="problem" class="problem">{{ problem }}</p>
</template>

<style scoped>
.project-tasks {
  display: grid;
  gap: 14px;
  margin: 0 0 16px;
}
.crumb {
  display: flex;
  gap: 6px;
  align-items: center;
  font-size: 12px;
  color: var(--text-dim);
}
.crumb button {
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent);
  font: inherit;
  cursor: pointer;
}
.meta {
  font-size: 12px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}
.progress-block {
  display: grid;
  gap: 6px;
}
.progress {
  display: block;
  height: 6px;
  border-radius: 99px;
  background: var(--hover);
  overflow: hidden;
}
.progress i {
  display: block;
  height: 100%;
  background: var(--good);
  transition: width 0.25s;
}
.section {
  display: grid;
  gap: 8px;
}
h3 {
  margin: 0;
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-dim);
}
.kids {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 8px;
}
.kid {
  display: grid;
  gap: 6px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--hover);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  min-width: 0;
}
.kid b {
  font-weight: 600;
  overflow-wrap: anywhere;
}
.kid:hover {
  border-color: var(--accent);
}
.kid.paused {
  border-style: dashed;
}
.kid.archived,
.kid.deleted {
  opacity: 0.7;
}
.dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-right: 6px;
  border-radius: 50%;
  background: var(--good);
}
.paused .dot {
  background: var(--warn);
}
.archived .dot,
.deleted .dot {
  background: var(--text-dim);
}
.tasks {
  display: grid;
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.tasks li {
  display: grid;
  grid-template-columns: 14px 18px 1fr auto auto;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: var(--hover);
  font-size: 14px;
}
.tasks li.over {
  border-color: var(--accent);
}
.tasks li.done label {
  color: var(--text-dim);
  text-decoration: line-through;
}
.tasks label {
  min-width: 0;
  overflow-wrap: anywhere;
  cursor: pointer;
}
.grip {
  color: var(--text-dim);
  font-size: 11px;
  cursor: grab;
  user-select: none;
}
.tasks input[type='checkbox'] {
  width: 16px;
  height: 16px;
  accent-color: var(--good);
}
.remove {
  padding: 0 6px;
  border: 0;
  background: none;
  color: var(--text-dim);
  font-size: 16px;
  cursor: pointer;
  opacity: 0;
}
.tasks li:hover .remove,
.remove:focus-visible {
  opacity: 1;
}
.add {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.add input {
  padding: 6px 8px;
  border: 1px dashed var(--line);
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 13px;
}
.add input:first-child {
  flex: 1;
  min-width: 160px;
}
.add button {
  padding: 5px 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--hover);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.add button:disabled {
  opacity: 0.5;
  cursor: default;
}
button:focus-visible,
input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
@media (prefers-reduced-motion: reduce) {
  .progress i {
    transition: none;
  }
}
</style>
