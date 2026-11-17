<script setup>
import { devOpenLink } from '../../api/devgit.js'
import { linkOf, ownerWords, programOf } from './devprojects.js'

// Every live developer server on this computer and which DEV project owns its port. View only:
// Castle never stops a server, so a port another project uses is always safe here.
defineProps({
  servers: { type: Array, default: () => [] },
  tabs: { type: Array, default: () => [] },
})
const emit = defineEmits(['close'])
</script>

<template>
  <div class="servers" role="dialog" aria-label="Live servers" @keydown.esc="emit('close')">
    <header>
      <span>LIVE SERVERS</span>
      <button aria-label="Close" @click="emit('close')">&times;</button>
    </header>
    <table v-if="servers.length">
      <thead>
        <tr><th>PORT</th><th>PROGRAM</th><th>PROJECT</th><th>STARTED BY</th><th></th></tr>
      </thead>
      <tbody>
        <tr v-for="s in servers" :key="s.port">
          <td class="port">{{ s.port }}</td>
          <td :title="`pid ${s.pid}${s.folder ? ' · ' + s.folder : ''}`">{{ programOf(s) }}</td>
          <td>{{ ownerWords(s, tabs) }}</td>
          <td>{{ s.startedBy || '-' }}</td>
          <td><button class="open" :aria-label="`Open port ${s.port}`" @click="devOpenLink(linkOf(s)).catch(() => {})">open &#8599;</button></td>
        </tr>
      </tbody>
    </table>
    <p v-else class="none">No developer servers are listening right now.</p>
    <p class="rule">Castle only shows these. It never stops a server, so a port another project uses is always left alone.</p>
  </div>
</template>

<style scoped>
.servers {
  position: absolute;
  z-index: 8;
  top: 52px;
  right: 16px;
  width: min(640px, calc(100% - 32px));
  max-height: 70%;
  overflow-y: auto;
  padding: 14px 16px;
  border: 1px solid color-mix(in srgb, var(--good, #8fe3b0) 45%, transparent);
  border-radius: 12px;
  background: var(--panel-background);
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55);
  color: var(--text);
  font-size: 13px;
}

header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 8px;
  letter-spacing: 0.16em;
  font-size: 11px;
  color: var(--good, #8fe3b0);
}

header button {
  border: 0;
  background: none;
  color: inherit;
  font-size: 18px;
  cursor: pointer;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th {
  text-align: left;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.08em;
  color: var(--text-dim);
}

td,
th {
  padding: 5px 6px;
  border-bottom: 1px solid var(--line);
}

.port {
  font-family: 'Cascadia Mono', Consolas, ui-monospace, Menlo, monospace;
}

.open {
  border: 0;
  background: none;
  color: var(--good, #8fe3b0);
  font: inherit;
  cursor: pointer;
}

.none,
.rule {
  color: var(--text-dim);
}

.rule {
  margin: 10px 0 0;
  font-size: 12px;
}

button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
</style>
