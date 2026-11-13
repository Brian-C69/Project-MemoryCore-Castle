<script setup>
// HOME's USAGE card: the five-hour and weekly limits of Claude Code, Codex or both, side by side -
// the ones switched on in Settings > Companion. Castle asks the CLIs at most every five minutes.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { cliUsage } from '../../api/castle.js'
import { rows } from './usage.js'

// Changes whenever Settings saves, so a CLI switched on or off there shows or goes at once.
const props = defineProps({ stamp: { type: Number, default: 0 } })
watch(() => props.stamp, () => read())

const usage = ref(null)
const now = ref(Date.now())
let timer
// Counts the reads. Only the newest one may fill the card, so a slow older answer that still
// carries a CLI just switched off never puts it back.
let latest = 0

async function read() {
  now.value = Date.now()
  if (document.visibilityState !== 'visible') return
  const mine = ++latest
  try {
    const answer = await cliUsage()
    if (mine === latest) usage.value = answer
  } catch {
    // The last reading stays.
  }
}

const clis = computed(() => (Array.isArray(usage.value) ? usage.value : []).map((u) => ({ ...u, card: rows(u, now.value) })))

onMounted(() => {
  read()
  timer = setInterval(read, 60000)
})
onBeforeUnmount(() => clearInterval(timer))
</script>

<template>
  <section class="widget card usage">
    <h3><span class="icon">◔</span>USAGE</h3>
    <p v-if="!usage" class="empty">Checking…</p>
    <p v-else-if="!clis.length" class="empty">Usage is off. Turn on Claude Code or Codex under Settings &gt; Companion.</p>
    <div v-for="u in clis" :key="u.cli" class="cli">
      <p class="name">{{ u.label }}<span v-if="u.card.stale" class="stale">{{ u.card.stale }}</span></p>
      <div class="bars">
        <p v-if="u.card.message" class="empty">{{ u.card.message }}</p>
        <div v-for="b in u.card.bars" :key="b.label" class="bar-row" :class="b.level" role="meter" :aria-label="`${u.label} ${b.label}`" :aria-valuenow="b.percent" aria-valuemin="0" aria-valuemax="100" :title="b.note">
          <span class="label">{{ b.label.startsWith('Week') ? 'Wk' : b.label }}</span>
          <span class="track"><span class="fill" :style="{ width: `${Math.min(100, b.percent)}%` }" /></span>
          <span class="percent">{{ b.percent }}%</span>
          <span class="note">{{ b.note }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.card {
  padding: 14px 18px;
  box-sizing: border-box;
}
h3 {
  margin: 0 0 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--line);
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.14em;
}
.icon {
  margin-right: 10px;
  color: var(--accent);
}
.cli {
  display: grid;
  grid-template-columns: 6.4em minmax(0, 1fr);
  gap: 10px;
  align-items: start;
}
.cli + .cli {
  margin-top: 8px;
}
.name {
  display: grid;
  margin: 0;
  font-size: 13px;
  line-height: 1.45;
}
.stale,
.note,
.empty {
  font-size: 12px;
  color: var(--text-dim);
}
.empty {
  margin: 0;
}
.bar-row {
  display: grid;
  grid-template-columns: 2.4em minmax(2em, 1fr) 2.8em minmax(0, 8.2em);
  gap: 8px;
  align-items: center;
  font-size: 12px;
  line-height: 1.7;
}
.label {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  color: var(--text-dim);
}
.track {
  height: 6px;
  overflow: hidden;
  border-radius: 3px;
  background: var(--hover);
}
.fill {
  display: block;
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
}
.warm .fill {
  background: #ffc27a;
}
.full .fill {
  background: #ff7a7a;
}
.percent {
  text-align: right;
}
.note {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
</style>
