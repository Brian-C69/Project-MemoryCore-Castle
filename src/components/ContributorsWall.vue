<script setup>
// Settings > About's Credits as a wall: the contributors' avatars with their merged pull requests,
// a detail panel for the one chosen, and thanks for help that is not code. Everything is read from
// files packed into Castle (src/contributors/) - the page never asks GitHub.
import { computed, ref } from 'vue'
import { openUrl } from '../api/castle.js'
import counts from '../contributors/counts.json'
import { FOUNDER, PEOPLE, THANKS } from '../contributors/people.js'
import { contributorsLine, founderEntry, mergedLine, wall } from '../contributors/wall.js'

const files = import.meta.glob('../contributors/avatars/*.png', { eager: true, query: '?url', import: 'default' })
const avatarOf = (login) => files[`../contributors/avatars/${login}.png`] || null

const all = ref(false)
const board = computed(() => wall(counts, PEOPLE, { all: all.value }))
const founder = founderEntry(FOUNDER)
// the founder's circle first, then the contributors in count order
const faces = computed(() => (founder ? [founder, ...board.value.people] : board.value.people))
const chosenLogin = ref(null)
const chosen = computed(() => faces.value.find((p) => p.login === chosenLogin.value) || faces.value[0] || null)
const pillOf = (p) => (p.founder ? 'Founder' : mergedLine(p.merged))
const broken = ref(new Set())
const pictureOf = (login) => (broken.value.has(login) ? null : avatarOf(login))
function lost(login) {
  broken.value = new Set([...broken.value, login])
}
</script>

<template>
  <div class="contributors">
    <div class="wall">
      <p class="count">{{ founder ? `Founder ${founder.name} · ` : '' }}{{ contributorsLine(board.total) }}</p>
      <div class="grid" role="list">
        <button
          v-for="p in faces"
          :key="p.login"
          class="face"
          :class="{ chosen: chosen?.login === p.login, founder: p.founder }"
          role="listitem"
          :aria-pressed="chosen?.login === p.login"
          :aria-label="`${p.name}, ${pillOf(p)}`"
          :title="`${p.name} - ${pillOf(p)}`"
          @click="chosenLogin = p.login"
        >
          <img v-if="pictureOf(p.login)" :src="pictureOf(p.login)" alt="" @error="lost(p.login)" />
          <span v-else class="initials">{{ p.initials }}</span>
          <span v-if="p.founder" class="badge crown" aria-hidden="true">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M2 12.5h12l1-8-4 3-3-5-3 5-4-3z" /></svg>
          </span>
          <span v-else class="badge">{{ p.merged }}</span>
        </button>
      </div>
      <button v-if="board.hidden || all" class="more" @click="all = !all">{{ all ? 'Show fewer' : `Show all (${board.hidden} more)` }}</button>
      <template v-if="THANKS.length">
        <h4>Special thanks</h4>
        <div class="thanks">
          <button v-for="t in THANKS" :key="t.name" class="chip" :title="t.for" @click="t.url && openUrl(t.url).catch(() => {})">
            <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"><circle cx="8" cy="5.5" r="2.8" /><path d="M2.8 14c.6-2.7 2.7-4.3 5.2-4.3s4.6 1.6 5.2 4.3" /></svg>
            {{ t.name }}<span class="for"> · {{ t.for }}</span>
          </button>
        </div>
      </template>
    </div>
    <aside v-if="chosen" class="panel" aria-live="polite">
      <div class="big">
        <img v-if="pictureOf(chosen.login)" :src="pictureOf(chosen.login)" alt="" @error="lost(chosen.login)" />
        <span v-else class="initials">{{ chosen.initials }}</span>
      </div>
      <h4 class="name">{{ chosen.name }}</h4>
      <span class="pill" :class="{ founder: chosen.founder }">{{ pillOf(chosen) }}</span>
      <ul>
        <li v-for="item in chosen.built" :key="item">{{ item }}</li>
      </ul>
    </aside>
  </div>
</template>

<style scoped>
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
.contributors {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 20px;
  align-items: start;
}
.count {
  margin: 0 0 12px;
  font-size: 12px;
  color: var(--text-dim);
}
.grid {
  display: grid;
  grid-template-columns: repeat(4, 64px);
  gap: 14px 18px;
}
.face {
  position: relative;
  width: 64px;
  height: 64px;
  padding: 0;
  border: 2px solid var(--line);
  border-radius: 50%;
  background: color-mix(in srgb, var(--text) 8%, transparent);
  cursor: pointer;
}
.face.chosen {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 30%, transparent);
}
.face:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
}
.face img,
.big img {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  object-fit: cover;
}
.initials {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  font-size: 18px;
  font-weight: 600;
  color: var(--text);
}
.badge {
  position: absolute;
  top: -6px;
  right: -8px;
  min-width: 22px;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--accent);
  color: #07142e;
  font-size: 11px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  line-height: 16px;
}
.badge.crown {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 22px;
  height: 20px;
  padding: 0;
  background: #f2c14e;
}
.face.founder {
  border-color: color-mix(in srgb, #f2c14e 70%, var(--line));
}
.face.founder.chosen {
  border-color: #f2c14e;
  box-shadow: 0 0 0 3px color-mix(in srgb, #f2c14e 30%, transparent);
}
.pill.founder {
  background: color-mix(in srgb, #f2c14e 28%, transparent);
}
.more {
  margin-top: 12px;
}
h4 {
  margin: 18px 0 8px;
  font-size: 14px;
  font-weight: 600;
}
.thanks {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border-radius: 999px;
}
.chip .for {
  color: var(--text-dim);
}
.panel {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: color-mix(in srgb, var(--text) 4%, transparent);
}
.big {
  width: 88px;
  height: 88px;
  border: 2px solid var(--accent);
  border-radius: 50%;
  background: color-mix(in srgb, var(--text) 8%, transparent);
}
.big .initials {
  font-size: 26px;
}
.name {
  margin: 4px 0 0;
  font-size: 18px;
}
.pill {
  padding: 3px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 22%, transparent);
  color: var(--text);
  font-size: 12px;
  font-weight: 600;
}
ul {
  margin: 6px 0 0;
  padding-left: 18px;
  font-size: 13px;
  line-height: 1.55;
}
li::marker {
  color: var(--accent);
}
@media (max-width: 640px) {
  .contributors {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
