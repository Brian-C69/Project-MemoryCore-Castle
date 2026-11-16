<script setup>
// MARKET: paper money on public Bybit prices, with the companion at the desk. The logic is one
// composable (useMarket); this view holds the room's frame, its sub-tabs and the Desk layout.
// Nothing real is ever ordered.
import { computed, ref } from 'vue'
import MarketAuto from '../components/market/MarketAuto.vue'
import MarketChart from '../components/market/MarketChart.vue'
import MarketCompanion from '../components/market/MarketCompanion.vue'
import MarketLog from '../components/market/MarketLog.vue'
import MarketPositionSheet from '../components/market/MarketPositionSheet.vue'
import MarketStart from '../components/market/MarketStart.vue'
import MarketTables from '../components/market/MarketTables.vue'
import MarketTicket from '../components/market/MarketTicket.vue'
import { signedUsd, tone, usd } from '../components/market/market.js'
import { useMarket } from '../composables/useMarket.js'

const props = defineProps({ companion: { type: String, default: 'Your companion' } })

const m = useMarket()
const SUBTABS = [
  { id: 'desk', label: 'Desk' },
  { id: 'log', label: 'Trade log' },
  { id: 'lessons', label: 'Lessons' },
  { id: 'settings', label: 'Settings' },
]
const sub = ref('desk')
const sheetId = ref(null)
const sheet = computed(() => m.positions.value.find((p) => p.id === sheetId.value) ?? null)
const tick = computed(() => m.instrument.value?.tickSize ?? 0.01)
const companionName = computed(() => props.companion || 'Companion')

const taker = ref('')
const maker = ref('')
const confirmClose = ref(false)

function openFees() {
  taker.value = m.setting.value ? (m.setting.value.takerRate * 100).toFixed(3) : ''
  maker.value = m.setting.value ? (m.setting.value.makerRate * 100).toFixed(3) : ''
}

function saveFees() {
  const t = Number(taker.value) / 100
  const k = Number(maker.value) / 100
  if (!(t >= 0 && t <= 0.01 && k >= 0 && k <= 0.01)) {
    m.problem.value = 'Fees go from 0% to 1%.'
    return
  }
  m.setFees(t, k)
}

async function closeAccount() {
  confirmClose.value = false
  await m.closeRun()
}

async function shut(positionId) {
  await m.close(positionId)
  sheetId.value = null
}
</script>

<template>
  <div class="market">
    <header class="bar">
      <h2>MARKET</h2>
      <nav class="subtabs" role="tablist" aria-label="Market">
        <button v-for="t in SUBTABS" :key="t.id" role="tab" :aria-selected="sub === t.id" :class="{ on: sub === t.id }" @click="sub = t.id; t.id === 'settings' && openFees()">{{ t.label }}</button>
      </nav>
      <span class="spacer"></span>
      <span v-if="m.notice.value" class="notice">{{ m.notice.value }}</span>
      <span v-if="m.problem.value" class="problem">{{ m.problem.value }}</span>
    </header>

    <template v-if="sub === 'desk'">
      <MarketStart v-if="m.state.value && !m.run.value" :busy="m.busy.value" @start="m.startRun" />
      <template v-else-if="m.run.value">
        <section class="panel strip" aria-label="Account">
          <div><span>PAPER MONEY</span><b>{{ usd(m.account.value?.startBalance) }}</b></div>
          <div><span>EQUITY</span><b>{{ usd(m.account.value?.equity) }}</b></div>
          <div><span>AVAILABLE</span><b>{{ usd(m.account.value?.available) }}</b></div>
          <div><span>UNREALISED</span><b :class="tone(m.account.value?.unrealised)">{{ signedUsd(m.account.value?.unrealised) }}</b></div>
          <div><span>RISK</span><b>{{ Math.round((m.account.value?.risk ?? 0) * 100) }}%</b></div>
        </section>
        <div class="desk">
          <MarketChart
            class="chart-area"
            :candles="m.candles.value"
            :quote="m.quote.value"
            :symbol="m.symbol.value"
            :symbols="m.symbols.value"
            :interval="m.interval.value"
            :tick="tick"
            :now="m.now.value"
            :feed="m.feed.value"
            :positions="m.positions.value"
            @set-exits="m.setExits($event.positionId, $event.tp, $event.sl)"
            @update:symbol="m.symbol.value = $event"
            @update:interval="m.interval.value = $event"
          />
          <MarketTicket
            class="ticket-area"
            :symbol="m.symbol.value"
            :quote="m.quote.value"
            :instrument="m.instrument.value"
            :available="m.account.value?.available ?? 0"
            :busy="m.busy.value"
            :preview="m.preview"
            @place="m.place"
            @leverage="m.setLeverage"
          />
          <MarketTables
            class="tables-area"
            :positions="m.positions.value"
            :orders="m.orders.value"
            :companion="companionName"
            :tick="tick"
            :history="m.history"
            :runs="m.runs"
            @open="sheetId = $event.id"
            @cancel="m.cancel"
          />
          <MarketCompanion class="companion-area" :companion="companionName" />
        </div>
      </template>
      <p v-else class="loading">Opening the market…</p>
    </template>

    <section v-else-if="sub === 'settings'" class="settings">
      <MarketAuto :companion="companionName" />
      <article class="panel">
        <h3>TRADING RULES</h3>
        <label>Taker fee <span><input v-model="taker" inputmode="decimal" />%</span></label>
        <label>Maker fee <span><input v-model="maker" inputmode="decimal" />%</span></label>
        <label>Prices from <span class="dim">Bybit public</span></label>
        <button class="primary" :disabled="!!m.busy.value" @click="saveFees">Save fees</button>
        <div v-if="m.run.value" class="danger-zone">
          <button v-if="!confirmClose" class="danger" @click="confirmClose = true">Close account</button>
          <p v-else>Sell everything at market, cancel every order and end this account? It stays readable in History.
            <button class="danger" @click="closeAccount">Close it</button> <button @click="confirmClose = false">Keep it</button>
          </p>
        </div>
      </article>
    </section>

    <MarketLog v-else :view="sub" :companion="companionName" :runs="m.runs" />

    <MarketPositionSheet
      v-if="sheet"
      :position="sheet"
      :tick="tick"
      :companion="companionName"
      :busy="m.busy.value"
      @close="sheetId = null"
      @exits="m.setExits"
      @margin="m.addMargin"
      @shut="shut"
    />
  </div>
</template>

<style scoped>
.market {
  position: absolute;
  inset: 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  color: var(--text);
}

.bar {
  display: flex;
  align-items: center;
  gap: 18px;
}

.bar h2 {
  margin: 0;
  font-size: 15px;
  letter-spacing: 0.2em;
}

.subtabs {
  display: flex;
  gap: 4px;
}

.subtabs button {
  padding: 6px 14px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: none;
  color: var(--text-dim);
  font: inherit;
  cursor: pointer;
}

.subtabs button.on {
  border-color: var(--line);
  background: color-mix(in srgb, var(--accent) 25%, transparent);
  color: var(--text);
}

.spacer {
  flex: 1;
}

.notice {
  font-size: 12px;
  color: var(--good);
}

.problem {
  font-size: 12px;
  color: var(--warn);
}

:deep(.panel) {
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--panel-background);
  backdrop-filter: blur(var(--glass-blur));
}

.strip {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  padding: 12px 18px;
}

.strip div {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.strip span {
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}

.strip b {
  font-size: 18px;
  font-weight: 500;
}

.desk {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) 280px 300px;
  grid-template-rows: minmax(0, 3fr) minmax(0, 2fr);
  grid-template-areas:
    'chart ticket companion'
    'tables ticket companion';
  gap: 14px;
}

.chart-area {
  grid-area: chart;
}

.ticket-area {
  grid-area: ticket;
  min-width: 0;
  overflow: auto;
}

.tables-area {
  grid-area: tables;
}

.companion-area {
  grid-area: companion;
}

.companion-area h3,
.settings h3 {
  margin: 0;
  font-size: 11px;
  letter-spacing: 0.16em;
  color: var(--text-dim);
}

.gain {
  color: var(--good);
}

.loss {
  color: var(--bad);
}

.dim,
.loading {
  color: var(--text-dim);
}

.settings {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 14px;
  align-items: start;
}

.settings article {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 20px;
}

.settings label {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  color: var(--text-dim);
}

.settings input {
  width: 80px;
  margin-right: 4px;
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text);
  font: inherit;
  text-align: right;
}

.settings button {
  padding: 8px 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: none;
  color: var(--text);
  font: inherit;
  cursor: pointer;
}

.settings button.primary {
  align-self: flex-start;
  background: color-mix(in srgb, var(--accent) 35%, transparent);
}

.danger-zone {
  padding-top: 12px;
  border-top: 1px solid var(--line);
}

.settings button.danger {
  border-color: var(--bad);
  color: var(--bad);
}

</style>
