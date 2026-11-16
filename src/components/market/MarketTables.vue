<script setup>
// Positions, Orders and History under the chart. Each row says who opened it; a position opens its
// sheet (exits, margin, close). History pages through fills, funding, the ledger and past accounts.
import { ref, watch } from 'vue'
import { openedBy, price, sideLabel, signedUsd, tone, usd } from './market.js'

const props = defineProps({
  positions: { type: Array, default: () => [] },
  orders: { type: Array, default: () => [] },
  companion: { type: String, default: 'Companion' },
  tick: { type: Number, default: 0.01 },
  history: { type: Function, required: true },
  runs: { type: Function, required: true },
})
const emit = defineEmits(['open', 'cancel'])

const tab = ref('positions')
const kind = ref('fills')
const page = ref(0)
const rows = ref([])
const pastRuns = ref([])
const runId = ref(null)

async function loadHistory() {
  try {
    if (kind.value === 'accounts') {
      pastRuns.value = await props.runs()
      return
    }
    const answer = await props.history(kind.value, runId.value, page.value)
    rows.value = Array.isArray(answer) ? answer : answer?.rows ?? []
  } catch {
    rows.value = []
  }
}

watch([tab, kind, page, runId], () => {
  if (tab.value === 'history') loadHistory()
})
</script>

<template>
  <section class="panel tables">
    <nav class="tabs" role="tablist">
      <button v-for="t in ['positions', 'orders', 'history']" :key="t" role="tab" :class="{ on: tab === t }" :aria-selected="tab === t" @click="tab = t">
        {{ t.toUpperCase() }}<span v-if="t === 'positions' && positions.length" class="count">{{ positions.length }}</span><span v-if="t === 'orders' && orders.length" class="count">{{ orders.length }}</span>
      </button>
    </nav>

    <table v-if="tab === 'positions'">
      <thead><tr><th>SYMBOL</th><th>SIDE</th><th>QTY</th><th>ENTRY</th><th>MARK</th><th>TP</th><th>SL</th><th>PNL</th><th>LIQ.</th><th>OPENED BY</th></tr></thead>
      <tbody>
        <tr v-for="p in positions" :key="p.id" class="clickable" @click="emit('open', p)">
          <td>{{ p.symbol }}</td>
          <td :class="p.side === 'SHORT' ? 'loss' : 'gain'">{{ sideLabel(p.side) }}</td>
          <td>{{ p.qty }}</td>
          <td>{{ price(p.entryPrice, tick) }}</td>
          <td>{{ price(p.mark, tick) }}</td>
          <td class="gain">{{ p.tpPrice ? price(p.tpPrice, tick) : '—' }}</td>
          <td class="loss">{{ p.slPrice ? price(p.slPrice, tick) : '—' }}</td>
          <td :class="tone(p.unrealisedPnl)">{{ signedUsd(p.unrealisedPnl) }}</td>
          <td>{{ p.liquidationPrice ? price(p.liquidationPrice, tick) : '—' }}</td>
          <td>{{ openedBy(p.openedBy, companion) }}</td>
        </tr>
        <tr v-if="!positions.length"><td colspan="10" class="empty">No open positions.</td></tr>
      </tbody>
    </table>

    <table v-else-if="tab === 'orders'">
      <thead><tr><th>SYMBOL</th><th>SIDE</th><th>TYPE</th><th>QTY</th><th>PRICE</th><th>BY</th><th></th></tr></thead>
      <tbody>
        <tr v-for="o in orders" :key="o.id">
          <td>{{ o.symbol }}</td>
          <td :class="o.side === 'SHORT' ? 'loss' : 'gain'">{{ sideLabel(o.side) }}{{ o.reduceOnly ? ' · reduce' : '' }}</td>
          <td>{{ o.orderType === 'LIMIT' ? 'Limit' : 'Market' }}</td>
          <td>{{ o.qty }}</td>
          <td>{{ o.price ? price(o.price, tick) : '—' }}</td>
          <td>{{ openedBy(o.source, companion) }}</td>
          <td><button class="link" @click="emit('cancel', o.id)">Cancel</button></td>
        </tr>
        <tr v-if="!orders.length"><td colspan="7" class="empty">No resting orders.</td></tr>
      </tbody>
    </table>

    <div v-else class="history">
      <div class="kinds">
        <button v-for="k in ['fills', 'orders', 'funding', 'ledger', 'accounts']" :key="k" :class="{ on: kind === k }" @click="kind = k; page = 0">{{ k[0].toUpperCase() + k.slice(1) }}</button>
        <span class="spacer"></span>
        <button v-if="kind !== 'accounts'" :disabled="page === 0" @click="page--">‹</button>
        <button v-if="kind !== 'accounts'" :disabled="rows.length < 20" @click="page++">›</button>
      </div>
      <table v-if="kind === 'accounts'">
        <thead><tr><th>ACCOUNT</th><th>STARTED WITH</th><th>BALANCE</th><th>STATUS</th><th></th></tr></thead>
        <tbody>
          <tr v-for="r in pastRuns" :key="r.id">
            <td>Account {{ r.runNo }}</td><td>{{ usd(r.startBalance) }}</td><td>{{ usd(r.balance) }}</td><td>{{ r.endedAt ? 'Closed' : 'Open' }}</td>
            <td><button class="link" @click="runId = r.id; kind = 'fills'">Fills</button></td>
          </tr>
        </tbody>
      </table>
      <table v-else>
        <tbody>
          <tr v-for="r in rows" :key="r.id">
            <td>{{ r.symbol ?? r.kind }}</td>
            <td>{{ r.side ? sideLabel(r.side) : r.note ?? '' }}</td>
            <td>{{ r.qty ?? '' }}</td>
            <td>{{ r.price ? price(r.price, tick) : r.rate != null ? `${(r.rate * 100).toFixed(4)}%` : '' }}</td>
            <td :class="tone(r.realisedPnl ?? r.amount)">{{ r.realisedPnl != null ? signedUsd(r.realisedPnl) : r.amount != null ? signedUsd(r.amount) : r.status ?? '' }}</td>
            <td class="dim">{{ r.createdAt ? new Date(r.createdAt).toLocaleString() : '' }}</td>
          </tr>
          <tr v-if="!rows.length"><td colspan="6" class="empty">Nothing here yet.</td></tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.tables {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: auto;
  scrollbar-color: var(--line) transparent;
  scrollbar-width: thin;
}

.tabs,
.kinds {
  display: flex;
  gap: 4px;
  padding: 10px 14px 0;
}

.tabs button,
.kinds button {
  padding: 6px 10px;
  border: 0;
  border-bottom: 2px solid transparent;
  background: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  letter-spacing: 0.12em;
  cursor: pointer;
}

.tabs button.on,
.kinds button.on {
  color: var(--text);
  border-bottom-color: var(--accent);
}

.count {
  margin-left: 6px;
  padding: 0 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 30%, transparent);
  font-size: 10px;
}

.spacer {
  flex: 1;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
}

th {
  padding: 8px 14px;
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.12em;
  text-align: left;
  color: var(--text-dim);
}

td {
  padding: 8px 14px;
  border-top: 1px solid var(--line);
}

.clickable {
  cursor: pointer;
}

.clickable:hover {
  background: color-mix(in srgb, var(--accent) 8%, transparent);
}

.gain {
  color: var(--good);
}

.loss {
  color: var(--bad);
}

.dim,
.empty {
  color: var(--text-dim);
}

.link {
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent);
  font: inherit;
  cursor: pointer;
}
</style>
