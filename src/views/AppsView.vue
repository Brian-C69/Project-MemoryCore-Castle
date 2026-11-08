<script setup>
import { computed, nextTick, onMounted, ref } from 'vue'
import { appOpen, appsDrawIcons, appsList } from '../api/castle.js'

const MOST_USED = 8

const apps = ref([])
const query = ref('')
const notice = ref('')
const error = ref('')
const search = ref(null)
let noticeTimer

const matches = computed(() => {
  const words = query.value.trim().toLowerCase()
  return words ? apps.value.filter((app) => app.name.toLowerCase().includes(words)) : apps.value
})

const mostUsed = computed(() =>
  query.value.trim()
    ? []
    : [...apps.value]
        .filter((app) => app.uses > 0)
        .sort((a, b) => b.uses - a.uses || a.name.localeCompare(b.name))
        .slice(0, MOST_USED),
)

function say(text) {
  notice.value = text
  clearTimeout(noticeTimer)
  noticeTimer = setTimeout(() => (notice.value = ''), 2500)
}

async function load() {
  try {
    apps.value = await appsList()
  } catch (reason) {
    error.value = String(reason)
  }
}

async function open(app) {
  error.value = ''
  try {
    await appOpen(app.path)
    app.uses += 1
    say(`Opening ${app.name}`)
  } catch (reason) {
    error.value = String(reason)
  }
}

function openFirst() {
  if (matches.value.length) open(matches.value[0])
}

const initial = (name) => name.trim().charAt(0).toUpperCase() || '?'

onMounted(async () => {
  await load()
  nextTick(() => search.value?.focus())
  // Icons are drawn once and cached; the room shows letters until they are ready.
  try {
    if ((await appsDrawIcons()) > 0) await load()
  } catch {
    // Letter tiles stay.
  }
})
</script>

<template>
  <section class="apps" aria-label="Apps">
    <div class="bar">
      <input
        ref="search"
        v-model="query"
        type="search"
        placeholder="Search apps"
        aria-label="Search apps"
        @keydown.enter="openFirst"
      />
      <span class="count">{{ matches.length }} apps</span>
    </div>

    <p v-if="error" class="error" role="alert">{{ error }}</p>

    <template v-if="mostUsed.length">
      <h3>MOST USED</h3>
      <div class="grid">
        <button v-for="app in mostUsed" :key="'used-' + app.path" class="tile" :title="app.name" @click="open(app)">
          <img v-if="app.icon" :src="app.icon" alt="" />
          <span v-else class="letter" aria-hidden="true">{{ initial(app.name) }}</span>
          <span class="name">{{ app.name }}</span>
        </button>
      </div>
    </template>

    <h3>{{ query.trim() ? 'FOUND' : 'ALL APPS' }}</h3>
    <div class="grid scroll">
      <button v-for="app in matches" :key="app.path" class="tile" :title="app.name" @click="open(app)">
        <img v-if="app.icon" :src="app.icon" alt="" loading="lazy" />
        <span v-else class="letter" aria-hidden="true">{{ initial(app.name) }}</span>
        <span class="name">{{ app.name }}</span>
      </button>
      <p v-if="!matches.length" class="quiet">No app matches "{{ query }}".</p>
    </div>

    <p v-if="notice" class="notice" role="status">{{ notice }}</p>
  </section>
</template>

<style scoped>
.apps {
  position: absolute;
  inset: 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 20px 24px;
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}

.bar {
  display: flex;
  align-items: center;
  gap: 14px;
}

input {
  flex: 1;
  max-width: 520px;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 15px;
}

.count {
  font-size: 12px;
  color: var(--text-dim);
}

h3 {
  margin: 8px 0 0;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
  gap: 8px;
}

.grid.scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  align-content: start;
  padding-right: 6px;
  scrollbar-width: thin;
  scrollbar-color: var(--line) transparent;
}

.tile {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 14px 6px 10px;
  border: 1px solid transparent;
  border-radius: 12px;
  background: none;
  color: var(--text);
  font: inherit;
  cursor: pointer;
}

.tile:hover {
  background: var(--hover);
  border-color: var(--line);
}

.tile img,
.letter {
  width: 40px;
  height: 40px;
}

.tile img {
  object-fit: contain;
}

.letter {
  display: grid;
  place-items: center;
  border-radius: 10px;
  background: color-mix(in srgb, var(--accent) 40%, transparent);
  font-size: 18px;
  font-weight: 600;
}

.name {
  display: -webkit-box;
  max-width: 100%;
  overflow: hidden;
  font-size: 12px;
  line-height: 1.3;
  text-align: center;
  overflow-wrap: anywhere;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.quiet {
  grid-column: 1 / -1;
  color: var(--text-dim);
}

.error {
  margin: 0;
  font-size: 13px;
  color: var(--bad);
}

.notice {
  position: absolute;
  right: 20px;
  bottom: 16px;
  margin: 0;
  padding: 6px 12px;
  border-radius: 8px;
  background: var(--accent);
  color: #160f33;
  font-size: 13px;
  font-weight: 600;
}

input:focus-visible,
.tile:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
