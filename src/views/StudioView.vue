<script setup>
// STUDIO: the four powers (picture, video, song, voice) with whether each is ready, built in
// (Codex draws by itself) or saves a prompt (no key yet), and everything the companion made.
import { computed, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import ImageViewer from '../components/ImageViewer.vue'
import { chatMediaOpen, studioLibrary, studioPowers } from '../api/castle.js'

const props = defineProps({
  companion: { type: String, default: 'Your companion' },
  stamp: { type: Number, default: 0 },
})
const emit = defineEmits(['keys'])

const POWERS = [
  { id: 'image', label: 'Pictures', key: 'ChatGPT (OpenAI)', ask: 'Draw me a picture of' },
  { id: 'video', label: 'Videos', key: 'Seedance', ask: 'Make a short video of' },
  { id: 'song', label: 'Songs', key: 'Suno', ask: 'Write and sing a song about' },
  { id: 'voice', label: 'Voice', key: 'ElevenLabs', ask: 'Say this out loud:' },
]
const STATE = {
  ready: { text: 'Ready', cls: 'ready' },
  builtin: { text: 'Built in (Codex)', cls: 'ready' },
  prompt: { text: 'Prompt only - add a key', cls: 'prompt' },
}

const powers = ref({})
const lib = ref({ images: [], videos: [], songs: [], voices: [], saved: [] })
const tab = ref('images')
const viewerStart = ref(null)
const src = (path) => convertFileSrc(path)

const pictures = computed(() => lib.value.images.map((m) => ({ path: m.path, name: m.name, url: src(m.path) })))
const counts = computed(() => ({
  images: lib.value.images.length,
  videos: lib.value.videos.length,
  songs: lib.value.songs.length,
  voices: lib.value.voices.length,
  saved: lib.value.saved.length,
}))

function when(at) {
  return at ? new Date(at * 1000).toLocaleString('en-US', { dateStyle: 'medium', timeStyle: 'short' }) : ''
}

async function load() {
  try {
    powers.value = await studioPowers()
    lib.value = await studioLibrary()
  } catch {
    // outside Castle's window
  }
}

onMounted(load)
watch(() => props.stamp, load)
</script>

<template>
  <div class="studio">
    <section class="powers">
      <article v-for="p in POWERS" :key="p.id" class="power widget">
        <h3>{{ p.label }}</h3>
        <span class="state" :class="STATE[powers[p.id]]?.cls">{{ STATE[powers[p.id]]?.text ?? '…' }}</span>
        <p class="hint">Ask {{ companion }}: <em>"{{ p.ask }} …"</em></p>
        <button v-if="powers[p.id] === 'prompt'" @click="emit('keys')">Add the {{ p.key }} key</button>
      </article>
    </section>

    <section class="gallery widget">
      <nav class="tabs">
        <button v-for="t in ['images', 'videos', 'songs', 'voices', 'saved']" :key="t" :class="{ active: tab === t }" @click="tab = t">
          {{ { images: 'Pictures', videos: 'Videos', songs: 'Songs', voices: 'Voice', saved: 'Saved prompts' }[t] }} ({{ counts[t] }})
        </button>
        <span class="spacer"></span>
        <button @click="load">Refresh</button>
      </nav>

      <div v-if="tab === 'images'" class="grid">
        <button v-for="(m, i) in lib.images" :key="m.path" class="thumb" :title="m.name" @click="viewerStart = i">
          <img :src="src(m.path)" :alt="m.name" loading="lazy" />
        </button>
      </div>
      <div v-else-if="tab === 'videos'" class="list">
        <div v-for="m in lib.videos" :key="m.path" class="video">
          <video :src="src(m.path)" controls preload="metadata"></video>
          <span>{{ m.name }} · {{ when(m.at) }}</span>
        </div>
      </div>
      <div v-else-if="tab === 'songs' || tab === 'voices'" class="list">
        <div v-for="m in lib[tab]" :key="m.path" class="audio">
          <div><strong>{{ m.name }}</strong><span v-if="m.group" class="group">{{ m.group }}</span></div>
          <audio :src="src(m.path)" controls preload="none"></audio>
        </div>
      </div>
      <div v-else class="list">
        <p class="hint">These were saved instead of made, because a key was missing. Add the key and ask {{ companion }} again to make the real thing.</p>
        <div v-for="m in lib.saved" :key="m.path" class="saved">
          <span>{{ m.name }}</span><span class="group">{{ when(m.at) }}</span>
          <button @click="chatMediaOpen(m.path, true)">Show file</button>
        </div>
      </div>
      <p v-if="!counts[tab]" class="empty">Nothing here yet.</p>
    </section>

    <ImageViewer v-if="viewerStart !== null" :pictures="pictures" :start="viewerStart" @close="viewerStart = null" />
  </div>
</template>

<style scoped>
.studio {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.widget {
  background: var(--panel-background);
  border: 1px solid var(--line);
  border-radius: 14px;
  backdrop-filter: blur(var(--glass-blur));
}
.powers {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
}
.power {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 16px;
}
h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 500;
}
.state {
  align-self: flex-start;
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
}
.state.ready {
  background: color-mix(in srgb, var(--good) 22%, transparent);
  color: var(--good);
}
.state.prompt {
  background: color-mix(in srgb, var(--warn) 22%, transparent);
  color: var(--warn);
}
.hint,
.empty {
  margin: 0;
  font-size: 13px;
  color: var(--text-dim);
}
.gallery {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  padding: 14px 18px;
}
.tabs {
  display: flex;
  gap: 6px;
  margin-bottom: 12px;
}
.tabs .active {
  border-color: var(--accent);
  background: var(--hover);
}
.spacer {
  flex: 1;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
  gap: 10px;
  overflow-y: auto;
}
.thumb {
  padding: 0;
  overflow: hidden;
  aspect-ratio: 1;
  border-radius: 10px;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
}
.video {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
}
.video video {
  max-width: 560px;
  border-radius: 10px;
}
.audio,
.saved {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
  font-size: 14px;
}
.group {
  margin-left: 10px;
  font-size: 12px;
  color: var(--text-dim);
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
</style>
