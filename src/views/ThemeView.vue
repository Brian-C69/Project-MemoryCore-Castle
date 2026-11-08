<script setup>
// THEME: every theme (built-in and the member's own) as a card, Use / Delete, and Create theme -
// Castle drafts a working theme from a picture at once, then the companion may polish it or
// paint a wallpaper for it. Castle's loader checks every theme before it is used.
import { computed, onMounted, ref } from 'vue'
import {
  choosePicture,
  getSettings,
  saveSettings,
  terminalState,
  terminalWrite,
  themeCreate,
  themeDelete,
  themeFolder,
  userThemes,
} from '../api/castle.js'
import { builtInThemes, fromUserFolders, problemWith } from '../themes/loader.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
})
const emit = defineEmits(['changed', 'show-chat'])

const themes = ref([])
const broken = ref([])
const currentId = ref('')
const busy = ref('')
const problem = ref('')
const done = ref('')
const form = ref({ open: false, name: '', mood: '', picture: '' })
const lastCreated = ref('')
let settings = null

const pictureName = computed(() => form.value.picture.split(/[\\/]/).pop())

async function load() {
  try {
    const view = await getSettings()
    settings = view.settings
    currentId.value = view.settings.theme ?? 'constellation'
    const yours = fromUserFolders(await userThemes())
    themes.value = [...builtInThemes(), ...yours.themes]
    broken.value = yours.broken
  } catch {
    themes.value = builtInThemes()
  }
}

async function attempt(label, work) {
  problem.value = ''
  done.value = ''
  busy.value = label
  try {
    return await work()
  } catch (reason) {
    problem.value = String(reason)
    return null
  } finally {
    busy.value = ''
  }
}

async function use(theme) {
  const why = problemWith(theme)
  if (why) {
    problem.value = why
    return
  }
  const saved = await attempt('Changing the theme…', () => saveSettings({ ...settings, theme: theme.id }))
  if (saved) {
    settings = saved.settings
    currentId.value = theme.id
    emit('changed')
  }
}

async function remove(theme) {
  if (theme.id === currentId.value) {
    problem.value = 'Choose another theme first - this one is in use.'
    return
  }
  if (await attempt('Deleting…', () => themeDelete(theme.id)) !== null) await load()
}

async function pickPicture() {
  const path = await choosePicture('Choose a picture for your theme')
  if (path) form.value.picture = path
}

async function create() {
  const id = await attempt('Making your theme…', () => themeCreate(form.value.name, form.value.mood, form.value.picture || null))
  if (!id) return
  lastCreated.value = id
  await load()
  const made = themes.value.find((t) => t.id === id)
  if (made) await use(made)
  done.value = `${made?.name ?? 'Your theme'} is ready.`
  form.value = { open: false, name: '', mood: '', picture: '' }
}

// A brief for the companion, sent through the session like typed words (several lines go in
// as one paste). The rules keep theme.json something Castle's loader accepts.
async function askCompanion(theme, paint) {
  const state = await terminalState().catch(() => null)
  if (!state?.running) {
    problem.value = `Wake ${props.companion} first, then ask again.`
    return
  }
  const folder = await themeFolder(theme.id)
  const lines = paint
    ? [
        `Please paint a wallpaper for my Castle theme "${theme.name}".`,
        `Mood: ${theme.description || 'calm and beautiful'}. A wide landscape picture with no text, calm on the left third (a panel sits there) and on the right side (cards sit there).`,
        `Save it as ${folder}\\wallpaper.png (1536x1024) with Castle's image helper, and set "wallpaper": "wallpaper.png" in ${folder}\\theme.json.`,
        'Tell me the cost first and ask before you draw.',
      ]
    : [
        `Please polish my Castle theme "${theme.name}" in ${folder}\\theme.json.`,
        `My words for it: ${theme.description || 'make it feel like the picture'}.`,
        'Edit only that file. Keep "id", "name" and "wallpaper". Colours go only in "colors" as --variables (--background, --text, --text-dim, --accent, --line, --panel-background, --widget-background, --bar-background, --hover). Widget places use only left, top, right, bottom, width, height.',
        'Keep text easy to read on the background. Tell me when you are done.',
      ]
  const text = lines.join('\n')
  await terminalWrite(`\u001b[200~${text}\u001b[201~`)
  await terminalWrite('\r')
  done.value = `Asked ${props.companion}. When it is done, press Reload themes.`
  emit('show-chat')
}

onMounted(load)
</script>

<template>
  <div class="theme-room">
    <header class="bar">
      <h2>THEME</h2>
      <span class="spacer"></span>
      <span v-if="busy" class="busy">{{ busy }}</span>
      <span v-else-if="problem" class="problem">{{ problem }}</span>
      <span v-else-if="done" class="good">{{ done }}</span>
      <button @click="load">Reload themes</button>
      <button class="primary" @click="form.open = !form.open">+ Create theme</button>
    </header>

    <section v-if="form.open" class="create widget">
      <h3>Create a theme</h3>
      <div class="row">
        <label>Name <input v-model="form.name" placeholder="Sakura Night" maxlength="40" /></label>
        <label class="wide">Mood and colours, in your words <input v-model="form.mood" placeholder="soft pink blossoms under a dark blue night, calm" /></label>
      </div>
      <div class="row">
        <button @click="pickPicture">{{ form.picture ? 'Change picture' : 'Upload a picture' }}</button>
        <span class="hint">{{ form.picture ? pictureName : 'No picture: Castle makes a soft gradient, and you can ask your companion to paint one after.' }}</span>
        <span class="spacer"></span>
        <button class="primary" :disabled="!form.name.trim() || !!busy" @click="create">Create</button>
      </div>
      <p class="hint">Castle takes the colours from your picture so the theme works right away. Then you can ask {{ companion }} to polish it.</p>
    </section>

    <div class="grid">
      <article v-for="theme in themes" :key="theme.id" class="card widget" :class="{ current: theme.id === currentId }">
        <div class="preview" :style="theme.wallpaperUrl ? { backgroundImage: `url(&quot;${theme.wallpaperUrl}&quot;)` } : { background: theme.colors?.['--background'] ?? '#050b1f' }">
          <span class="swatch" :style="{ background: theme.colors?.['--accent'] }"></span>
          <span v-if="theme.id === currentId" class="badge">In use</span>
        </div>
        <div class="info">
          <strong>{{ theme.name }}</strong>
          <span class="source">{{ theme.source === 'yours' ? 'Yours' : 'Built in' }}</span>
        </div>
        <p class="desc">{{ theme.description }}</p>
        <div class="row">
          <button v-if="theme.id !== currentId" class="primary" @click="use(theme)">Use</button>
          <template v-if="theme.source === 'yours'">
            <button :title="`Ask ${companion} to polish the colours and layout`" @click="askCompanion(theme, false)">Polish</button>
            <button :title="`Ask ${companion} to paint a wallpaper`" @click="askCompanion(theme, true)">Paint</button>
            <button class="danger" @click="remove(theme)">Delete</button>
          </template>
        </div>
      </article>
    </div>

    <div v-if="broken.length" class="broken widget">
      <h3>Themes that could not be read</h3>
      <p v-for="b in broken" :key="b.folder">{{ b.folder }}: {{ b.error }}</p>
    </div>
  </div>
</template>

<style scoped>
.theme-room {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.bar,
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}
h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}
h3 {
  margin: 0 0 10px;
  font-size: 15px;
  font-weight: 500;
}
.spacer {
  flex: 1;
}
.create {
  padding: 18px 20px;
}
label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
}
label.wide {
  flex: 1;
}
input {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.25);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}
.card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
}
.card.current {
  border-color: var(--accent);
}
.preview {
  position: relative;
  height: 150px;
  border-radius: 10px;
  background-size: cover;
  background-position: center;
}
.swatch {
  position: absolute;
  bottom: 10px;
  left: 10px;
  width: 22px;
  height: 22px;
  border: 2px solid rgba(255, 255, 255, 0.7);
  border-radius: 50%;
}
.badge {
  position: absolute;
  top: 10px;
  right: 10px;
  padding: 3px 10px;
  border-radius: 999px;
  background: var(--accent);
  color: #07142e;
  font-size: 12px;
  font-weight: 600;
}
.info {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.source,
.desc,
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-dim);
}
.busy {
  font-size: 13px;
  color: var(--accent);
}
.problem {
  font-size: 13px;
  color: var(--warn);
}
.good {
  font-size: 13px;
  color: var(--good);
}
.broken {
  padding: 14px 18px;
  font-size: 13px;
  color: var(--warn);
}
button {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #07142e;
  font-weight: 600;
}
button.danger {
  border-color: var(--bad);
  color: var(--bad);
}
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
