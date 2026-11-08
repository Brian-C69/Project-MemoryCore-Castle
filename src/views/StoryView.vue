<script setup>
// STORY: the member's adventures in a session of their own (in <memory>/adventures, with
// Castle's rulebook). The story's Chat and Log on the left; on the right the setup card for a new
// adventure, the saved worlds with Load and Read, and their scene pictures.
import { computed, onMounted, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import TerminalPanel from '../components/TerminalPanel.vue'
import ModalSheet from '../components/ModalSheet.vue'
import MarkdownText from '../components/MarkdownText.vue'
import ImageViewer from '../components/ImageViewer.vue'
import { memoryRead, storyClose, storyPrepare, storyWorlds, terminalState, terminalWrite } from '../api/castle.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  member: { type: String, default: '' },
  cli: { type: String, default: 'claude' },
})

const CLASSES = ['Sword Saint', 'Lancer', 'Archmage', 'Shadow Reaper', 'Guardian', 'Ranger', 'Enchanter', 'Berserker']
const WORLDS = ['High Fantasy', 'Dark Fantasy', 'Eastern Fantasy', 'Steampunk', 'Celestial', 'Pirate/Naval', 'Demonic', 'Random']

const stamp = ref(0)
const ready = ref(false)
const problem = ref('')
const worlds = ref([])
const setup = ref({ party: 'duo', power: 'OP', hero: '', heroClass: 'Sword Saint', world: 'Random' })
const reader = ref(null)
const viewer = ref(null)

const inProgress = computed(() => worlds.value.filter((w) => w.status === 'In Progress'))

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
    worlds.value = await storyWorlds()
  } catch {
    worlds.value = []
  }
}

// Words go into the STORY session exactly as typed in its Chat.
async function say(text) {
  const state = await terminalState('story').catch(() => null)
  if (!state?.running) {
    problem.value = `Wake ${props.companion} in the story panel first.`
    return
  }
  problem.value = ''
  await terminalWrite(text, 'story')
  await terminalWrite('\r', 'story')
}

function begin() {
  const s = setup.value
  const hero = s.hero.trim() || props.member || 'the hero'
  say(`new adventure: ${s.party}, ${s.power}, hero "${hero}", class ${s.heroClass}, world ${s.world}`)
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

function showScenes(world, i = 0) {
  viewer.value = { pictures: world.scenes.map((p) => ({ path: p, name: p.split(/[\\/]/).pop(), url: convertFileSrc(p) })), start: i }
}

onMounted(prepare)
</script>

<template>
  <div class="story-room" style="--panel-left: 24px; --panel-top: 24px; --panel-bottom: 24px; --panel-width: 56%">
    <TerminalPanel v-if="ready" room="story" :settings-stamp="stamp" :companion="companion" :cli="cli" @setup="prepare" @settings="prepare" />

    <aside class="side">
      <section class="card widget">
        <h3>NEW ADVENTURE</h3>
        <div class="choices">
          <label><input v-model="setup.party" type="radio" value="duo" /> Duo with {{ companion }}</label>
          <label><input v-model="setup.party" type="radio" value="solo" /> Solo</label>
        </div>
        <div class="choices">
          <label><input v-model="setup.power" type="radio" value="OP" /> OP - win with style</label>
          <label><input v-model="setup.power" type="radio" value="Balanced" /> Balanced - real danger</label>
        </div>
        <label class="field">Hero name <input v-model="setup.hero" :placeholder="member || 'Your hero'" maxlength="40" /></label>
        <div class="selects">
          <label class="field">Class
            <select v-model="setup.heroClass"><option v-for="c in CLASSES" :key="c">{{ c }}</option></select>
          </label>
          <label class="field">World
            <select v-model="setup.world"><option v-for="w in WORLDS" :key="w">{{ w }}</option></select>
          </label>
        </div>
        <button class="primary" @click="begin">Open the portal</button>
      </section>

      <section class="card widget worlds">
        <header class="row"><h3>YOUR WORLDS</h3><span class="spacer"></span><button @click="loadWorlds">Refresh</button></header>
        <p v-if="!worlds.length" class="hint">No adventures yet. Choose above and open the portal.</p>
        <article v-for="w in worlds" :key="w.id" class="world">
          <div class="row">
            <strong>{{ w.name }}</strong>
            <span class="status" :class="{ done: w.status !== 'In Progress' }">{{ w.status }}{{ w.chapter ? ` · chapter ${w.chapter}` : '' }}</span>
          </div>
          <div v-if="w.scenes.length" class="scenes">
            <button v-for="(s, i) in w.scenes.slice(-4)" :key="s" class="scene" @click="showScenes(w, w.scenes.length - Math.min(4, w.scenes.length) + i)">
              <img :src="convertFileSrc(s)" alt="" loading="lazy" />
            </button>
          </div>
          <div class="row">
            <button v-if="w.status === 'In Progress'" class="primary" @click="say(`load adventure ${w.name}`)">Continue</button>
            <button :disabled="!w.parts.length && !w.summary" @click="read(w)">Read</button>
          </div>
        </article>
      </section>

      <div class="row">
        <button @click="say('save adventure')" :disabled="!inProgress.length">Save adventure</button>
        <button @click="storyClose()">Leave the story</button>
      </div>
      <p v-if="problem" class="problem">{{ problem }}</p>
    </aside>

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
.side {
  position: absolute;
  top: 24px;
  right: 24px;
  bottom: 24px;
  width: calc(44% - 72px);
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow-y: auto;
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px;
}
h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.14em;
}
.choices,
.selects,
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}
.selects > * {
  flex: 1;
}
.spacer {
  flex: 1;
}
label {
  font-size: 13px;
}
.field {
  display: flex;
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
.world {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 0;
  border-top: 1px solid var(--line);
}
.status {
  font-size: 12px;
  color: var(--good);
}
.status.done {
  color: var(--text-dim);
}
.scenes {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
}
.scene {
  padding: 0;
  overflow: hidden;
  aspect-ratio: 3 / 2;
  border-radius: 6px;
}
.scene img {
  width: 100%;
  height: 100%;
  object-fit: cover;
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
