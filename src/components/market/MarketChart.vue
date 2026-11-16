<script setup>
// The candle chart (design 1): candles and volume for the chosen symbol and interval, drawn with
// lightweight-charts in Castle's own colours. The price strip above it carries the symbol picker,
// last, mark, funding and the age of the price.
import { createChart, CrosshairMode } from 'lightweight-charts'
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import GlassSelect from '../GlassSelect.vue'
import { INTERVALS, countdown, price } from './market.js'

const props = defineProps({
  candles: { type: Array, default: () => [] },
  quote: { type: Object, default: null },
  symbol: { type: String, required: true },
  symbols: { type: Array, default: () => [] },
  interval: { type: String, required: true },
  tick: { type: Number, default: 0.01 },
  now: { type: Number, default: () => Date.now() },
  feed: { type: Object, default: null },
})
const emit = defineEmits(['update:symbol', 'update:interval'])

const box = ref(null)
let chart
let candleSeries
let volumeSeries
let resizer

function colour(name, fallback) {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

function draw() {
  if (!candleSeries) return
  const rows = props.candles.map((c) => ({ time: Math.floor(c.openMs / 1000), open: c.open, high: c.high, low: c.low, close: c.close }))
  const good = colour('--good', '#4cd6a0')
  const bad = colour('--bad', '#ff9aa8')
  candleSeries.setData(rows)
  volumeSeries.setData(props.candles.map((c) => ({ time: Math.floor(c.openMs / 1000), value: c.volume, color: c.close >= c.open ? `${good}55` : `${bad}55` })))
}

onMounted(async () => {
  await nextTick()
  const text = colour('--text-dim', '#9aa7c4')
  const line = colour('--line', 'rgba(150,170,220,0.25)')
  chart = createChart(box.value, {
    autoSize: true,
    layout: { background: { color: 'transparent' }, textColor: text, fontFamily: 'inherit' },
    grid: { vertLines: { color: line }, horzLines: { color: line } },
    rightPriceScale: { borderColor: line },
    timeScale: { borderColor: line, timeVisible: true, secondsVisible: false },
    crosshair: { mode: CrosshairMode.Normal },
  })
  candleSeries = chart.addCandlestickSeries({
    upColor: colour('--good', '#4cd6a0'),
    downColor: colour('--bad', '#ff9aa8'),
    wickUpColor: colour('--good', '#4cd6a0'),
    wickDownColor: colour('--bad', '#ff9aa8'),
    borderVisible: false,
  })
  volumeSeries = chart.addHistogramSeries({ priceFormat: { type: 'volume' }, priceScaleId: '' })
  volumeSeries.priceScale().applyOptions({ scaleMargins: { top: 0.82, bottom: 0 } })
  draw()
  chart.timeScale().fitContent()
})

watch(() => props.candles, (rows, old) => {
  draw()
  if (!old?.length && rows.length) chart?.timeScale().fitContent()
})
watch(() => props.tick, (tick) => {
  const d = Math.max(0, Math.round(-Math.log10(tick || 0.01)))
  candleSeries?.applyOptions({ priceFormat: { type: 'price', precision: d, minMove: tick || 0.01 } })
})

onBeforeUnmount(() => {
  resizer?.disconnect()
  chart?.remove()
})

const symbolOptions = () => {
  const list = props.symbols.map((s) => ({ value: s.symbol, label: s.symbol }))
  return list.length ? list : [{ value: props.symbol, label: props.symbol }]
}
</script>

<template>
  <section class="panel chart-panel">
    <header class="strip">
      <GlassSelect class="picker" :model-value="symbol" :options="symbolOptions()" @update:model-value="emit('update:symbol', $event)" />
      <div class="figure"><span>LAST</span><b>{{ quote ? price(quote.last, tick) : '—' }}</b></div>
      <div class="figure"><span>MARK</span><b>{{ quote ? price(quote.mark, tick) : '—' }}</b></div>
      <div class="figure"><span>FUNDING</span><b>{{ quote ? `${(quote.fundingRate * 100).toFixed(4)}%` : '—' }} <small>{{ quote ? countdown(quote.nextFundingMs, now) : '' }}</small></b></div>
      <span class="age" :class="{ stale: feed && feed.status !== 'OK' }" :title="feed?.note || ''">
        {{ feed && feed.status !== 'OK' ? (feed.note || 'Prices paused') : quote ? `Bybit · ${Math.round(quote.ageMs / 1000)} s` : '' }}
      </span>
      <span class="spacer"></span>
      <div class="intervals" role="group" aria-label="Interval">
        <button v-for="i in INTERVALS" :key="i" :class="{ on: interval === i }" :aria-pressed="interval === i" @click="emit('update:interval', i)">{{ i }}</button>
      </div>
    </header>
    <div ref="box" class="chart" aria-label="Price chart"></div>
  </section>
</template>

<style scoped>
.chart-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 18px;
  padding: 12px 16px 8px;
}

.picker {
  min-width: 150px;
}

.figure {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.figure span {
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--text-dim);
}

.figure b {
  font-weight: 500;
  font-size: 14px;
}

.figure small {
  color: var(--text-dim);
  font-weight: 400;
  margin-left: 4px;
}

.age {
  font-size: 11px;
  color: var(--text-dim);
}

.age.stale {
  color: var(--warn);
}

.spacer {
  flex: 1;
}

.intervals {
  display: flex;
  gap: 2px;
}

.intervals button {
  padding: 4px 9px;
  border: 0;
  border-radius: 6px;
  background: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.intervals button.on {
  background: color-mix(in srgb, var(--accent) 30%, transparent);
  color: var(--text);
}

.chart {
  flex: 1;
  min-height: 240px;
  padding: 0 8px 8px;
}
</style>
