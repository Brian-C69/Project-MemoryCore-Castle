<script setup>
// The companion's panel on the desk (design 2): the chat about the market from its newest turns,
// the LAST TRADE card, and the message box. Asking it to trade places a COMPANION_ASKED order
// through the same door as a click.
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { marketTurn, marketTurns, onMarketChanged } from '../../api/castle.js'
import { chatLines, lastTrade } from './turns.js'

const props = defineProps({ companion: { type: String, default: 'Companion' } })

const turns = ref([])
const words = ref('')
const thinking = ref(false)
const problem = ref('')
const list = ref(null)
let stop

const lines = computed(() => chatLines(turns.value))
const last = computed(() => lastTrade(turns.value))

async function load() {
  try {
    turns.value = await marketTurns(null, 0)
  } catch {
    // the last chat stays
  }
}

async function send() {
  const text = words.value.trim()
  if (!text || thinking.value) return
  thinking.value = true
  problem.value = ''
  turns.value = [{ id: 'pending', message: text, reply: null, createdAt: Date.now(), status: 'RUNNING' }, ...turns.value]
  words.value = ''
  try {
    await marketTurn(text)
  } catch (reason) {
    problem.value = String(reason?.message ?? reason)
  } finally {
    thinking.value = false
    await load()
  }
}

watch(lines, async () => {
  await nextTick()
  if (list.value) list.value.scrollTop = list.value.scrollHeight
})

onMounted(async () => {
  load()
  try {
    stop = await onMarketChanged(load)
  } catch {
    // outside Castle's window
  }
})
onBeforeUnmount(() => stop?.())
</script>

<template>
  <aside class="panel companion" aria-label="Companion">
    <h3>{{ companion.toUpperCase() }}</h3>
    <div ref="list" class="lines">
      <p v-if="!lines.length && !thinking" class="dim">Ask {{ companion }} about the market, or to place a trade for you.</p>
      <div v-for="l in lines" :key="l.id" class="line" :class="[l.who, { failed: l.failed }]">
        <span class="bubble">{{ l.text }}</span>
        <time>{{ new Date(l.at).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) }}</time>
      </div>
      <div v-if="thinking" class="line companion"><span class="bubble dim">Looking at the desk…</span></div>
    </div>
    <section v-if="last" class="last">
      <h4>LAST TRADE</h4>
      <p><span class="chip" :class="last.kind">{{ last.chip }}</span> <span class="dim">{{ last.result }}</span></p>
    </section>
    <p v-if="problem" class="problem">{{ problem }}</p>
    <form class="ask" @submit.prevent="send">
      <input v-model="words" :disabled="thinking" maxlength="2000" placeholder="Type a message…" aria-label="Message to your companion" />
      <button :disabled="thinking || !words.trim()" aria-label="Send">➤</button>
    </form>
  </aside>
</template>

<style scoped>
.companion {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  padding: 14px 16px;
}

h3,
h4 {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.lines {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  scrollbar-width: var(--scrollbar-width);
  /* Issue #44: the list sits inside a backdrop-filter panel, where Chromium/WebView2 may scroll it
     without a layer of its own, and lines promoted to their own layers inside such a scroller can
     be drawn at a stale scroll offset - text over text while every box measures correctly. The
     scroller gets the layer and its own paint boundary instead, and the lines paint normally. */
  will-change: transform;
  contain: paint;
  isolation: isolate;
}

.line {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  max-width: 88%;
}

.line.member {
  align-self: flex-end;
  align-items: flex-end;
}

.bubble {
  padding: 8px 11px;
  border-radius: 12px;
  font-size: 13px;
  line-height: 1.4;
  white-space: pre-wrap;
  background: color-mix(in srgb, var(--accent) 14%, transparent);
  border: 1px solid var(--line);
}

.line.member .bubble {
  background: color-mix(in srgb, var(--accent) 45%, transparent);
}

.line.failed .bubble {
  border-color: var(--warn);
}

time {
  margin-top: 2px;
  font-size: 10px;
  color: var(--text-dim);
}

.last {
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
}

.last p {
  margin: 6px 0 0;
  font-size: 12.5px;
}

.chip {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 12px;
}

.chip.buy {
  background: color-mix(in srgb, var(--good) 30%, transparent);
}

.chip.sell {
  background: color-mix(in srgb, var(--bad) 30%, transparent);
}

.chip.close {
  background: color-mix(in srgb, var(--accent) 25%, transparent);
}

.ask {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

.ask input {
  padding: 9px 11px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
}

.ask button {
  width: 38px;
  border: 0;
  border-radius: 50%;
  background: var(--accent);
  color: var(--bg, #0b1020);
  cursor: pointer;
}

.ask button:disabled {
  opacity: 0.5;
}

.dim {
  color: var(--text-dim);
}

.problem {
  margin: 0;
  font-size: 12px;
  color: var(--warn);
}
</style>
