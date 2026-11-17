<script setup>
// One position's sheet (page 3): its figures, the exits (take profit, stop loss), margin for an
// isolated position, and Close position. Esc or a click outside closes the sheet.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { openedBy, price, sideLabel, signedUsd, tone } from './market.js'

const props = defineProps({
  position: { type: Object, required: true },
  tick: { type: Number, default: 0.01 },
  companion: { type: String, default: 'Companion' },
  busy: { type: String, default: '' },
})
const emit = defineEmits(['close', 'exits', 'margin', 'shut'])

const tp = ref('')
const sl = ref('')
const margin = ref('')

watch(
  () => props.position,
  (p) => {
    tp.value = p.tpPrice ?? ''
    sl.value = p.slPrice ?? ''
  },
  { immediate: true },
)

const num = (v) => (String(v ?? '').trim() === '' ? null : Number(v))

function onKey(event) {
  if (event.key === 'Escape') {
    event.preventDefault()
    event.stopImmediatePropagation()
    emit('close')
  }
}
onMounted(() => window.addEventListener('keydown', onKey, true))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey, true))
</script>

<template>
  <Teleport to="body">
    <div class="backdrop" role="dialog" aria-modal="true" :aria-label="`${position.symbol} ${sideLabel(position.side)}`" @click.self="emit('close')">
      <article class="sheet">
        <header>
          <h3>{{ position.symbol }} {{ sideLabel(position.side).toUpperCase() }}</h3>
          <button class="x" aria-label="Close" @click="emit('close')">×</button>
        </header>
        <dl>
          <dt>Qty</dt><dd>{{ position.qty }}</dd>
          <dt>Entry</dt><dd>{{ price(position.entryPrice, tick) }}</dd>
          <dt>Mark</dt><dd>{{ price(position.mark, tick) }}</dd>
          <dt>PnL</dt><dd :class="tone(position.unrealisedPnl)">{{ signedUsd(position.unrealisedPnl) }}</dd>
          <dt>Leverage</dt><dd>{{ position.leverage }}x · {{ position.marginMode === 'ISOLATED' ? 'Isolated' : 'Cross' }}</dd>
          <dt>Liquidation</dt><dd>{{ position.liquidationPrice ? price(position.liquidationPrice, tick) : '—' }}</dd>
          <dt>Opened by</dt><dd>{{ openedBy(position.openedBy, companion) }}</dd>
        </dl>
        <section>
          <h4>EXITS</h4>
          <label>Take profit<input v-model="tp" inputmode="decimal" placeholder="—" /></label>
          <label>Stop loss<input v-model="sl" inputmode="decimal" placeholder="—" /></label>
          <button class="primary" :disabled="!!busy" @click="emit('exits', position.id, num(tp), num(sl))">Save exits</button>
        </section>
        <section v-if="position.marginMode === 'ISOLATED'">
          <h4>MARGIN</h4>
          <div class="row">
            <input v-model="margin" inputmode="decimal" placeholder="USD" />
            <button :disabled="!!busy || !(Number(margin) > 0)" @click="emit('margin', position.id, Number(margin)); margin = ''">Add margin</button>
          </div>
        </section>
        <footer>
          <button class="danger" :disabled="!!busy" @click="emit('shut', position.id)">Close position</button>
          <button @click="emit('close')">Cancel</button>
        </footer>
      </article>
    </div>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: grid;
  place-items: center;
  background: rgba(4, 8, 20, 0.45);
  -webkit-backdrop-filter: blur(4px); backdrop-filter: blur(4px);
}

.sheet {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: 400px;
  max-width: calc(100vw - 32px);
  padding: 18px 20px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  -webkit-backdrop-filter: blur(var(--glass-blur)); backdrop-filter: blur(var(--glass-blur));
  color: var(--text);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
}

header {
  display: flex;
  align-items: center;
}

h3 {
  flex: 1;
  margin: 0;
  font-size: 14px;
  letter-spacing: 0.14em;
}

h4 {
  margin: 0 0 6px;
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.x {
  border: 0;
  background: none;
  color: var(--text-dim);
  font-size: 20px;
  cursor: pointer;
}

dl {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 6px 12px;
  margin: 0;
  font-size: 13px;
}

dt {
  color: var(--text-dim);
}

dd {
  margin: 0;
  text-align: right;
}

section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 10px;
  border-top: 1px solid var(--line);
}

label {
  display: grid;
  grid-template-columns: 110px 1fr;
  align-items: center;
  font-size: 13px;
}

input {
  padding: 7px 9px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
}

.row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

button {
  padding: 8px 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: none;
  color: var(--text);
  font: inherit;
  cursor: pointer;
}

button.primary {
  background: color-mix(in srgb, var(--accent) 40%, transparent);
}

footer {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

button.danger {
  border-color: var(--bad);
  background: color-mix(in srgb, var(--bad) 30%, transparent);
}

.gain {
  color: var(--good);
}

.loss {
  color: var(--bad);
}
</style>
