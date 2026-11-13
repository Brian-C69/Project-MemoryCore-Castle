<script setup>
import { computed, reactive, ref, watch } from 'vue'
import { devTree } from '../../api/devfiles.js'
import { openedFor, sizeWords, visibleRows } from './tree.js'

// A DEV tab's FILES: the project's tree, one folder level read at a time as folders open, entries
// the project's .gitignore covers greyed. Read only: a file clicked opens in the reader.
const props = defineProps({
  // the DEV tab's slot
  slot: { type: Number, required: true },
  // the tab's project folder (opened folders are remembered per project)
  folder: { type: String, required: true },
  // the panel is showing (it reads only then)
  shown: { type: Boolean, default: false },
  // the rel of the file open in the reader, marked in the tree
  selected: { type: String, default: '' },
})
// open: a file entry { name, rel, size, private, ... } to show in the reader
const emit = defineEmits(['open'])

const levels = reactive({})
const more = reactive({})
const opened = ref(openedFor(props.folder))
const filter = ref('')
const problem = ref('')
const reading = ref(false)
// bumped when the opened set changes (a plain Set is not watched by Vue)
const tick = ref(0)

const rows = computed(() => (tick.value, visibleRows(levels, opened.value, filter.value)))

async function load(rel) {
  try {
    const level = await devTree(props.slot, rel)
    levels[rel] = level.entries
    more[rel] = level.more
    problem.value = ''
  } catch (reason) {
    problem.value = String(reason)
    if (rel) opened.value.delete(rel)
  }
}

async function choose(entry) {
  if (!entry.dir) return emit('open', entry)
  if (opened.value.has(entry.rel)) opened.value.delete(entry.rel)
  else {
    opened.value.add(entry.rel)
    if (!levels[entry.rel]) await load(entry.rel)
  }
  tick.value += 1
}

/** Reads the top and every opened folder again (new and changed files appear). */
async function refresh() {
  reading.value = true
  await load('')
  for (const rel of [...opened.value]) await load(rel)
  reading.value = false
  tick.value += 1
}

watch(
  () => [props.slot, props.folder],
  () => {
    for (const key of Object.keys(levels)) delete levels[key]
    for (const key of Object.keys(more)) delete more[key]
    opened.value = openedFor(props.folder)
    tick.value += 1
    if (props.shown) refresh()
  },
)
watch(
  () => props.shown,
  (on) => {
    if (on) refresh()
  },
  { immediate: true },
)

defineExpose({ refresh })
</script>

<template>
  <aside class="file-tree" aria-label="Files">
    <header>
      <span class="tag">FILES</span>
      <span class="spacer"></span>
      <button class="small" title="Read again" aria-label="Read again" :disabled="reading" @click="refresh">&#8635;</button>
    </header>

    <p v-if="problem" class="problem">{{ problem }}</p>
    <ul class="rows" role="tree">
      <li
        v-for="row in rows"
        :key="row.entry.rel"
        role="treeitem"
        :aria-expanded="row.entry.dir ? opened.has(row.entry.rel) : undefined"
        :class="{ dir: row.entry.dir, ignored: row.entry.ignored, private: row.entry.private, picked: row.entry.rel === selected }"
      >
        <button class="name" :style="{ paddingLeft: `${4 + row.depth * 14}px` }" :title="row.entry.rel" @click="choose(row.entry)">
          <span class="caret" aria-hidden="true">{{ row.entry.dir ? (opened.has(row.entry.rel) ? '▾' : '▸') : '' }}</span>
          <span class="label">{{ row.entry.name }}</span>
          <span v-if="row.entry.private" class="lock" title="Kept private - not opened here">private</span>
          <span v-if="!row.entry.dir" class="size">{{ sizeWords(row.entry.size) }}</span>
        </button>
        <p v-if="row.entry.dir && opened.has(row.entry.rel) && more[row.entry.rel]" class="more" :style="{ paddingLeft: `${18 + (row.depth + 1) * 14}px` }">
          and {{ more[row.entry.rel] }} more
        </p>
      </li>
    </ul>
    <p v-if="more['']" class="more">and {{ more[''] }} more</p>
    <p v-if="!problem && !rows.length && levels[''] && !filter" class="quiet">This folder is empty.</p>
    <p v-else-if="!problem && !rows.length && filter" class="quiet">Nothing read so far matches.</p>
    <p v-else-if="!problem && !levels['']" class="quiet">Reading the files…</p>

    <input v-model="filter" class="filter" placeholder="Filter by name" aria-label="Filter the files by name" />
  </aside>
</template>

<style scoped>
.file-tree {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  padding: 14px 10px 12px 12px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
  box-sizing: border-box;
  font-size: 13px;
}

header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 0 8px 4px;
  border-bottom: 1px solid var(--line);
}

.tag {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.2em;
  color: var(--accent);
}

.spacer {
  flex: 1;
}

.rows {
  flex: 1;
  min-height: 0;
  margin: 6px 0 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
  scrollbar-width: thin;
  scrollbar-color: var(--line) transparent;
}

.name {
  display: flex;
  align-items: center;
  gap: 5px;
  width: 100%;
  min-width: 0;
  padding: 3px 6px 3px 4px;
  border: 0;
  border-radius: 6px;
  background: none;
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.name:hover {
  background: var(--hover);
}

.name:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

li.picked > .name {
  background: color-mix(in srgb, var(--accent) 16%, transparent);
}

.caret {
  flex: none;
  width: 10px;
  font-size: 11px;
  color: var(--text-dim);
}

.label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

li.dir > .name .label {
  font-weight: 600;
}

li.ignored > .name .label,
li.ignored > .name .size {
  opacity: 0.45;
}

.size {
  flex: none;
  font-size: 11px;
  color: var(--text-dim);
}

.lock {
  flex: none;
  padding: 0 5px;
  border: 1px solid var(--line);
  border-radius: 5px;
  font-size: 10px;
  letter-spacing: 0.06em;
  color: var(--warn);
}

.more {
  margin: 2px 0;
  font-size: 11px;
  color: var(--text-dim);
}

.quiet {
  margin: 8px 4px;
  font-size: 13px;
  color: var(--text-dim);
}

.problem {
  margin: 8px 4px 0;
  font-size: 13px;
  color: var(--warn);
}

.filter {
  margin-top: 8px;
  padding: 6px 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
}

.filter:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.small {
  padding: 4px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.small:hover:not(:disabled) {
  background: var(--hover);
}

.small:disabled {
  opacity: 0.5;
  cursor: wait;
}

.small:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
