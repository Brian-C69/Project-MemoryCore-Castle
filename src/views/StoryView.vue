<script setup>
// STORY: the member's adventures as a visual novel. The Shelf shows their worlds; New adventure
// opens the portal; the Stage plays the story in the look they chose (Settings › Story, or Look
// on the stage's bar). The story's own session (in <memory>/adventures, with Castle's rulebook)
// runs behind it - its terminal stays alive behind Log, and opens by itself when the companion
// needs waking.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import TerminalPanel from '../components/TerminalPanel.vue'
import ModalSheet from '../components/ModalSheet.vue'
import MarkdownText from '../components/MarkdownText.vue'
import ImageViewer from '../components/ImageViewer.vue'
import StageView from '../components/stage/StageView.vue'
import { getSettings, memoryRead, saveSettings, storyClose, storyPrepare, storyWorlds, terminalState, terminalWrite } from '../api/castle.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  member: { type: String, default: '' },
  cli: { type: String, default: 'claude' },
  visible: { type: Boolean, default: true },
})

const CLASSES = ['Sword Saint', 'Lancer', 'Archmage', 'Shadow Reaper', 'Guardian', 'Ranger', 'Enchanter', 'Berserker']
const WORLDS = ['High Fantasy', 'Dark Fantasy', 'Eastern Fantasy', 'Steampunk', 'Celestial', 'Pirate/Naval', 'Demonic', 'Random']
const NEW_WORLD = { id: '', name: 'A new adventure', status: 'In Progress', chapter: null, parts: [], summary: null, scenes: [], folder: '', cast: [], music: [] }

const stamp = ref(0)
const ready = ref(false)
const problem = ref('')
const worlds = ref([])
const settings = ref(null)
const mode = ref('shelf') // shelf | stage
const playing = ref(null) // the world on stage
const logOpen = ref(false)
const setupOpen = ref(false)
const setup = ref({ party: 'duo', power: 'OP', hero: '', heroClass: 'Sword Saint', world: 'Random' })
const reader = ref(null)
const viewer = ref(null)
let worldTimer = 0

const look = computed(() => settings.value?.storyLook ?? 'classic')
const portrait = computed(() => {
  const folder = settings.value?.memoryFolder
  return folder ? `${folder}\\appearance\\portrait.png` : null
})
const keyArt = (w) => (w.scenes?.length ? convertFileSrc(w.scenes[w.scenes.length - 1]) : null)

async function prepare() {
  try {
    await storyPrepare()
    ready.value = true
    stamp.value += 1
  } catch (reason) {
    problem.value = String(reason)
  }
  loadWorlds()
}

async function loadWorlds() {
  try {
    const before = worlds.value.map((w) => w.id)
    worlds.value = await storyWorlds()
    // a new adventure takes the world folder the story just made
    if (playing.value && !playing.value.id) {
      const fresh = worlds.value.find((w) => !before.includes(w.id))
      if (fresh) playing.value = fresh
    } else if (playing.value) {
      playing.value = worlds.value.find((w) => w.id === playing.value.id) ?? playing.value
    }
  } catch {
    worlds.value = []
  }
}

// Words go into the STORY session exactly as typed in its Chat; when it sleeps, its panel opens
// so the member can wake it.
async function say(text) {
  const state = await terminalState('story').catch(() => null)
  if (!state?.running) {
    problem.value = `Wake ${props.companion} to play - the story panel is open.`
    logOpen.value = true
    return false
  }
  problem.value = ''
  await terminalWrite(text, 'story')
  await terminalWrite('\r', 'story')
  return true
}

function play(world) {
  playing.value = world
  mode.value = 'stage'
}

async function begin() {
  const s = setup.value
  const hero = s.hero.trim() || props.member || 'the hero'
  setupOpen.value = false
  play({ ...NEW_WORLD })
  await say(`new adventure: ${s.party}, ${s.power}, hero "${hero}", class ${s.heroClass}, world ${s.world}`)
}

async function continueWorld(world) {
  play(world)
  await say(`load adventure ${world.name}`)
}

async function setLook(next) {
  try {
    settings.value = (await saveSettings({ ...settings.value, storyLook: next })).settings
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function read(world) {
  const file = world.parts[world.parts.length - 1] ?? world.summary
  if (!file) return
  try {
    reader.value = { title: world.name, text: await memoryRead(file), parts: world.parts, file }
  } catch (reason) {
    problem.value = String(reason)
  }
}

async function readPart(file) {
  try {
    reader.value = { ...reader.value, text: await memoryRead(file), file }
  } catch (reason) {
    problem.value = String(reason)
  }
}

function showPictures(paths, i = 0) {
  if (!paths?.length) return
  viewer.value = { pictures: paths.map((p) => ({ path: p, name: p.split(/[\\/]/).pop(), url: convertFileSrc(p) })), start: i }
}

onMounted(async () => {
  try {
    settings.value = (await getSettings()).settings
  } catch {
    // outside Castle's window
  }
  prepare()
  worldTimer = setInterval(() => props.visible && loadWorlds(), 10000)
})
onBeforeUnmount(() => clearInterval(worldTimer))
watch(() => props.visible, (on) => on && loadWorlds())
</script>

<template>
  <div class="story-room">
    <!-- the story's own session: alive behind Log, shown as a drawer when asked -->
    <div class="log-drawer" :class="{ open: logOpen }" style="--panel-left: 0px; --panel-top: 0px; --panel-bottom: 0px; --panel-width: 100%">
      <TerminalPanel v-if="ready" room="story" :settings-stamp="stamp" :companion="companion" :cli="cli" @setup="prepare" @settings="prepare" />
      <button class="close-log" @click="logOpen = false">Close</button>
    </div>

    <section v-if="mode === 'shelf'" class="shelf">
      <header class="row">
        <h2>YOUR ADVENTURES</h2>
        <span class="spacer"></span>
        <button @click="logOpen = !logOpen">Log</button>
        <button @click="storyClose()">Leave the story</button>
        <button class="primary" @click="setupOpen = true">New adventure</button>
      </header>
      <p v-if="problem" class="problem">{{ problem }}</p>
      <div class="cards">
        <article class="card new" @click="setupOpen = true">
          <span class="plus">+</span>
          <strong>New adventure</strong>
          <span class="hint">Choose your class and world, and open the portal.</span>
        </article>
        <article v-for="w in worlds" :key="w.id" class="card">
          <div class="art" :style="keyArt(w) ? { backgroundImage: `url(&quot;${keyArt(w)}&quot;)` } : {}" @click="showPictures(w.scenes, w.scenes.length - 1)"></div>
          <strong>{{ w.name }}</strong>
          <span class="status" :class="{ done: w.status !== 'In Progress' }">{{ w.status }}{{ w.chapter ? ` · chapter ${w.chapter}` : '' }}</span>
          <div class="row">
            <button v-if="w.status === 'In Progress'" class="primary" @click="continueWorld(w)">Continue</button>
            <button v-else @click="play(w)">Open</button>
            <button :disabled="!w.parts.length && !w.summary" @click="read(w)">Read</button>
          </div>
        </article>
      </div>
    </section>

    <section v-else class="stage-wrap">
      <StageView
        :world="playing"
        :companion="companion"
        :portrait="portrait"
        :look="look"
        :active="visible && mode === 'stage'"
        :say="say"
        @shelf="mode = 'shelf'; loadWorlds()"
        @save="say('save adventure')"
        @story="read(playing)"
        @scenes="showPictures(playing.scenes, playing.scenes.length - 1)"
        @cast="showPictures(playing.cast.map((c) => c.path))"
        @log="logOpen = !logOpen"
        @look="setLook"
      />
      <p v-if="problem" class="problem floating">{{ problem }}</p>
    </section>

    <ModalSheet v-if="setupOpen" tag="STORY" title="A new adventure" @close="setupOpen = false">
      <div class="setup">
        <div class="choices">
          <label><input v-model="setup.party" type="radio" value="duo" /> Duo with {{ companion }}</label>
          <label><input v-model="setup.party" type="radio" value="solo" /> Solo</label>
        </div>
        <div class="choices">
          <label><input v-model="setup.power" type="radio" value="OP" /> OP - win with style</label>
          <label><input v-model="setup.power" type="radio" value="Balanced" /> Balanced - real danger</label>
        </div>
        <label class="field">Hero name <input v-model="setup.hero" :placeholder="member || 'Your hero'" maxlength="40" /></label>
        <div class="choices">
          <label class="field">Class
            <select v-model="setup.heroClass"><option v-for="c in CLASSES" :key="c">{{ c }}</option></select>
          </label>
          <label class="field">World
            <select v-model="setup.world"><option v-for="w in WORLDS" :key="w">{{ w }}</option></select>
          </label>
        </div>
        <button class="primary" @click="begin">Open the portal</button>
      </div>
    </ModalSheet>
    <ModalSheet v-if="reader" tag="STORY" :title="reader.title" @close="reader = null">
      <template #head>
        <div v-if="reader.parts.length > 1" class="row parts">
          <button v-for="(p, i) in reader.parts" :key="p" :class="{ primary: p === reader.file }" @click="readPart(p)">Part {{ i + 1 }}</button>
        </div>
      </template>
      <MarkdownText :text="reader.text" />
    </ModalSheet>
    <ImageViewer v-if="viewer" :pictures="viewer.pictures" :start="viewer.start" @close="viewer = null" />
  </div>
</template>

<style scoped>
.story-room {
  position: absolute;
  inset: 0;
}
.log-drawer {
  position: absolute;
  top: 24px;
  right: 24px;
  bottom: 24px;
  z-index: 6;
  width: min(560px, 45%);
  opacity: 0;
  pointer-events: none;
  transform: translateX(20px);
  transition: opacity 0.2s, transform 0.2s;
}
.log-drawer.open {
  opacity: 1;
  pointer-events: auto;
  transform: none;
}
.close-log {
  position: absolute;
  top: 10px;
  right: 54px;
  z-index: 2;
}
.shelf {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
}
.stage-wrap {
  position: absolute;
  inset: 24px;
}
h2 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.2em;
  color: var(--accent);
}
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 16px;
}
.card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
}
.card.new {
  justify-content: center;
  align-items: center;
  min-height: 230px;
  border-style: dashed;
  text-align: center;
  cursor: pointer;
}
.plus {
  font-size: 40px;
  color: var(--accent);
}
.art {
  height: 150px;
  border-radius: 10px;
  background: linear-gradient(160deg, #1b2266, #3a2a6a) center / cover;
  cursor: pointer;
}
.status {
  font-size: 12px;
  color: var(--good);
}
.status.done {
  color: var(--text-dim);
}
.row,
.choices {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}
.spacer {
  flex: 1;
}
.setup {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
label {
  font-size: 13px;
}
.field {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 4px;
}
input:not([type='radio']),
select {
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.3);
  color: var(--text);
  font: inherit;
  font-size: 14px;
}
.hint {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.problem {
  margin: 0;
  font-size: 13px;
  color: var(--warn);
}
.problem.floating {
  position: absolute;
  top: 12px;
  left: 50%;
  padding: 6px 12px;
  border-radius: 8px;
  background: rgba(6, 10, 32, 0.8);
  transform: translateX(-50%);
}
.parts {
  margin-top: 8px;
}
button {
  padding: 6px 12px;
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
button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
