<script setup>
// The candle chart (design 1): candles and volume for the chosen symbol and interval, drawn with
// lightweight-charts in Castle's own colours. The price strip above it carries the symbol picker,
// last, mark, funding and the age of the price.
import { createChart, CrosshairMode } from 'lightweight-charts'
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import GlassSelect from '../GlassSelect.vue'
import { INTERVALS, countdown, price } from './market.js'
import { rsi, sma, stochastic } from './indicators.js'
import { marketCandlesBefore } from '../../api/castle.js'

const props = defineProps({
  candles: { type: Array, default: () => [] },
  quote: { type: Object, default: null },
  symbol: { type: String, required: true },
  symbols: { type: Array, default: () => [] },
  interval: { type: String, required: true },
  tick: { type: Number, default: 0.01 },
  now: { type: Number, default: () => Date.now() },
  feed: { type: Object, default: null },
  positions: { type: Array, default: () => [] },
})
const emit = defineEmits(['update:symbol', 'update:interval', 'set-exits'])

const box = ref(null)
let chart
let candleSeries
let volumeSeries
let resizer
let lastTime = null // the newest bar's time already drawn, so a live poll only updates what changed
let allCandles = [] // oldest first: the live 300-bar window plus any older history scrolled into view
let loadingMore = false
let noMoreBefore = false // Bybit had nothing older for this scope - stop asking

// Indicators: SMA overlays the candles directly. RSI and Stochastic each get their own separate
// chart in their own row below, the way a real trading platform lays out oscillator panes rather
// than squeezing a 0-100 line into the same box as the candles - so each is its own `createChart`
// instance, time-synced with the main chart (and each other) rather than sharing its price scale.
const showSma20 = ref(false)
const showSma50 = ref(false)
const showRsi = ref(false)
const showStoch = ref(false)
let sma20Series = null
let sma50Series = null
const rsiBox = ref(null)
const stochBox = ref(null)
let rsiChart = null
let rsiLineSeries = null
let stochChart = null
let stochKLine = null
let stochDLine = null
let syncingRange = false

function colour(name, fallback) {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

function point(c) {
  return { time: Math.floor(c.openMs / 1000), open: c.open, high: c.high, low: c.low, close: c.close }
}

function volumePoint(c, good, bad) {
  return { time: Math.floor(c.openMs / 1000), value: c.volume, color: c.close >= c.open ? `${good}55` : `${bad}55` }
}

// Every poll re-fetches the whole 300-bar window, but replacing it wholesale with setData()
// every few seconds fights any zoom or pan the member just did - lightweight-charts snaps the
// view back each time. A full redraw (setData) is only for the first load or a changed
// symbol/interval; a live poll of the same scope instead updates just the bars that are new
// since last time, the way the library's own real-time pattern expects, leaving the view alone.
function drawFull() {
  if (!candleSeries) return
  allCandles = [...props.candles]
  noMoreBefore = false
  const good = colour('--good', '#4cd6a0')
  const bad = colour('--bad', '#ff9aa8')
  candleSeries.setData(allCandles.map(point))
  volumeSeries.setData(allCandles.map((c) => volumePoint(c, good, bad)))
  lastTime = allCandles.length ? Math.floor(allCandles[allCandles.length - 1].openMs / 1000) : null
  syncIndicators()
}

function upsert(c) {
  const i = allCandles.findIndex((x) => x.openMs === c.openMs)
  if (i === -1) allCandles.push(c)
  else allCandles[i] = c
}

function drawUpdate() {
  if (!candleSeries) return
  const good = colour('--good', '#4cd6a0')
  const bad = colour('--bad', '#ff9aa8')
  for (const c of props.candles) {
    const t = Math.floor(c.openMs / 1000)
    if (lastTime !== null && t < lastTime) continue
    candleSeries.update(point(c))
    volumeSeries.update(volumePoint(c, good, bad))
    upsert(c)
    lastTime = t
  }
  syncIndicators()
}

// The live poll only ever carries the newest 300 bars - scrolling further back than that finds
// nothing more unless something goes and gets it. Lightweight-charts tells us when the visible
// range nears the start of what is loaded; that is the cue to ask Bybit for the page right before
// the oldest bar we have and prepend it, the same way CHAT backfills older lines on request.
async function loadMore() {
  if (loadingMore || noMoreBefore || !allCandles.length || !candleSeries) return
  loadingMore = true
  try {
    const oldest = allCandles[0].openMs
    const older = await marketCandlesBefore(props.symbol, props.interval, 300, oldest - 1)
    const seen = new Set(allCandles.map((c) => c.openMs))
    const fresh = (older || []).filter((c) => !seen.has(c.openMs))
    if (!fresh.length) {
      noMoreBefore = true
      return
    }
    const range = chart?.timeScale().getVisibleLogicalRange()
    allCandles = [...fresh, ...allCandles].sort((a, b) => a.openMs - b.openMs)
    const good = colour('--good', '#4cd6a0')
    const bad = colour('--bad', '#ff9aa8')
    candleSeries.setData(allCandles.map(point))
    volumeSeries.setData(allCandles.map((c) => volumePoint(c, good, bad)))
    syncIndicators()
    // bar indices just shifted right by the number of bars prepended - hold the same view in place
    if (range) chart.timeScale().setVisibleLogicalRange({ from: range.from + fresh.length, to: range.to + fresh.length })
  } catch {
    // left as it was; scrolling back again tries once more
  } finally {
    loadingMore = false
  }
}

function toggleSma(period) {
  const on = period === 20 ? showSma20 : showSma50
  on.value = !on.value
  if (on.value) {
    const colourName = period === 20 ? '--accent' : null
    const series = chart.addLineSeries({ color: colourName ? colour(colourName, '#8fb8ff') : '#f0a868', lineWidth: 2, priceLineVisible: false, lastValueVisible: false })
    if (period === 20) sma20Series = series
    else sma50Series = series
  } else {
    const series = period === 20 ? sma20Series : sma50Series
    if (series) chart.removeSeries(series)
    if (period === 20) sma20Series = null
    else sma50Series = null
  }
  syncIndicators()
}

// Keeps every live chart (main, RSI, Stochastic) on the same visible range, whichever one the
// member just scrolled or zoomed - `syncingRange` stops the resulting setVisibleLogicalRange
// calls from re-triggering this same handler back and forth.
function syncTimeScale(source, range) {
  if (syncingRange || !range) return
  syncingRange = true
  try {
    for (const c of [chart, rsiChart, stochChart]) {
      if (c && c !== source) c.timeScale().setVisibleLogicalRange(range)
    }
  } finally {
    syncingRange = false
  }
}

function oscillatorChart(container) {
  const text = colour('--text-dim', '#9aa7c4')
  const line = colour('--line', 'rgba(150,170,220,0.25)')
  const own = createChart(container, {
    autoSize: true,
    layout: { background: { color: 'transparent' }, textColor: text, fontFamily: 'inherit' },
    grid: { vertLines: { color: line }, horzLines: { color: line } },
    rightPriceScale: { borderColor: line },
    timeScale: { borderColor: line, timeVisible: true, secondsVisible: false },
    crosshair: { mode: CrosshairMode.Normal },
  })
  own.timeScale().subscribeVisibleLogicalRangeChange((range) => syncTimeScale(own, range))
  const current = chart?.timeScale().getVisibleLogicalRange()
  if (current) own.timeScale().setVisibleLogicalRange(current)
  return own
}

function oscillatorLine(ownChart, lineColor) {
  return ownChart.addLineSeries({
    color: lineColor,
    lineWidth: 1,
    priceLineVisible: false,
    lastValueVisible: false,
    autoscaleInfoProvider: () => ({ priceRange: { minValue: 0, maxValue: 100 } }),
  })
}

function referenceLines(series) {
  series.createPriceLine({ price: 70, color: colour('--bad', '#ff9aa8'), lineWidth: 1, lineStyle: 2, axisLabelVisible: false })
  series.createPriceLine({ price: 30, color: colour('--good', '#4cd6a0'), lineWidth: 1, lineStyle: 2, axisLabelVisible: false })
}

async function toggleOscillator(which) {
  const on = which === 'rsi' ? showRsi : showStoch
  on.value = !on.value
  if (!on.value) return
  await nextTick()
  if (which === 'rsi' && !rsiChart) {
    rsiChart = oscillatorChart(rsiBox.value)
    rsiLineSeries = oscillatorLine(rsiChart, colour('--accent', '#8fb8ff'))
    referenceLines(rsiLineSeries)
  } else if (which === 'stoch' && !stochChart) {
    stochChart = oscillatorChart(stochBox.value)
    stochKLine = oscillatorLine(stochChart, colour('--accent', '#8fb8ff'))
    stochDLine = oscillatorLine(stochChart, colour('--text-dim', '#9aa7c4'))
    referenceLines(stochKLine)
  }
  syncIndicators()
}

function syncIndicators() {
  if (!candleSeries || !allCandles.length) return
  const times = allCandles.map((c) => Math.floor(c.openMs / 1000))
  const line = (values) => values.map((v, i) => (v === null ? null : { time: times[i], value: v })).filter(Boolean)
  if (sma20Series) sma20Series.setData(line(sma(allCandles.map((c) => c.close), 20)))
  if (sma50Series) sma50Series.setData(line(sma(allCandles.map((c) => c.close), 50)))
  if (rsiLineSeries) rsiLineSeries.setData(line(rsi(allCandles.map((c) => c.close), 14)))
  if (stochKLine) {
    const { k, d } = stochastic(allCandles.map((c) => c.high), allCandles.map((c) => c.low), allCandles.map((c) => c.close), 14, 3)
    stochKLine.setData(line(k))
    stochDLine.setData(line(d))
  }
}

// Open-position lines (entry/TP/SL) for whichever positions sit on the chart's own symbol. Full
// remove-and-redraw on every relevant change - there are never more than a handful of positions
// at once, so it is simpler and safer than trying to patch individual lines in place.
let positionLines = [] // [{ id, kind: 'entry'|'tp'|'sl', handle, price, side }]
let dragging = null // the line currently being dragged (entry, tp or sl), or null
let previewLine = null // the live TP/SL preview shown while dragging away from the entry line

function clearPositionLines() {
  for (const { handle } of positionLines) candleSeries?.removePriceLine(handle)
  positionLines = []
}

function syncPositionLines() {
  if (!candleSeries || dragging) return
  clearPositionLines()
  const good = colour('--good', '#4cd6a0')
  const bad = colour('--bad', '#ff9aa8')
  const neutral = colour('--text-dim', '#9aa7c4')
  for (const p of props.positions.filter((p) => p.symbol === props.symbol)) {
    positionLines.push({
      id: p.id,
      kind: 'entry',
      price: p.entryPrice,
      side: p.side,
      handle: candleSeries.createPriceLine({ price: p.entryPrice, color: neutral, lineWidth: 1, lineStyle: 2, axisLabelVisible: true, title: 'Entry · drag to set TP/SL' }),
    })
    if (p.tpPrice) {
      positionLines.push({ id: p.id, kind: 'tp', price: p.tpPrice, handle: candleSeries.createPriceLine({ price: p.tpPrice, color: good, lineWidth: 2, axisLabelVisible: true, title: 'TP · drag to move' }) })
    }
    if (p.slPrice) {
      positionLines.push({ id: p.id, kind: 'sl', price: p.slPrice, handle: candleSeries.createPriceLine({ price: p.slPrice, color: bad, lineWidth: 2, axisLabelVisible: true, title: 'SL · drag to move' }) })
    }
  }
}

// Lightweight-charts has no built-in draggable price line, so dragging is done by hand: find the
// line under the pointer by comparing its current price's own y-coordinate, then track the drag
// on window (not just the chart box) so a fast drag that leaves the chart area still works.
// Dragging an existing TP or SL line moves that one; dragging the entry line itself is how a TP
// or SL gets set in the first place (nothing to grab otherwise, before either exists) - which one
// it becomes follows the direction dragged: toward profit is TP, toward loss is SL, same as how
// TP/SL means something regardless of which value is actually being set.
const DRAG_HIT_PX = 6
function lineNear(clientY) {
  const rect = box.value.getBoundingClientRect()
  const y = clientY - rect.top
  const hit = (l) => Math.abs((candleSeries.priceToCoordinate(l.price) ?? Infinity) - y) <= DRAG_HIT_PX
  // an existing TP/SL line takes priority over the entry line beneath it, so adjusting one that
  // already sits close to entry doesn't instead start creating a second one
  return positionLines.find((l) => l.kind !== 'entry' && hit(l)) || positionLines.find((l) => l.kind === 'entry' && hit(l))
}
function removePreview() {
  if (previewLine) candleSeries.removePriceLine(previewLine.handle)
  previewLine = null
}
function onPointerDown(event) {
  const hit = lineNear(event.clientY)
  if (!hit) return
  event.preventDefault()
  event.stopPropagation()
  dragging = hit
  box.value.style.cursor = 'ns-resize'
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp)
}
function onPointerMove(event) {
  if (!dragging) return
  const rect = box.value.getBoundingClientRect()
  const raw = candleSeries.coordinateToPrice(event.clientY - rect.top)
  if (raw === null) return
  // a pixel position turns into an arbitrary float (e.g. 63315.774912...); the instrument only
  // accepts prices on its own tick (Castle's own validation rejects anything else), so every
  // dragged price is snapped to it before it is shown or sent anywhere
  const step = props.tick || 0.01
  const p = Math.round(raw / step) * step
  if (dragging.kind !== 'entry') {
    dragging.price = p
    dragging.handle.applyOptions({ price: p })
    return
  }
  // Castle's own validation checks a new TP/SL against the current mark, not the entry price -
  // classify the drag the same way, or a price that is "above entry" but still below a mark that
  // has since moved gets rejected as an invalid take profit.
  const reference = props.quote?.mark ?? dragging.price
  const favourable = dragging.side === 'LONG' ? p > reference : p < reference
  const kind = favourable ? 'tp' : 'sl'
  const color = colour(favourable ? '--good' : '--bad', favourable ? '#4cd6a0' : '#ff9aa8')
  if (previewLine?.kind === kind) {
    previewLine.handle.applyOptions({ price: p })
  } else {
    removePreview()
    previewLine = { handle: candleSeries.createPriceLine({ price: p, color, lineWidth: 2, lineStyle: 3, axisLabelVisible: true, title: kind.toUpperCase() }) }
  }
  previewLine.kind = kind
  previewLine.price = p
}
function onPointerUp() {
  if (!dragging) return
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', onPointerUp)
  if (box.value) box.value.style.cursor = ''
  const position = props.positions.find((p) => p.id === dragging.id)
  if (position) {
    const { kind, price: newPrice } = dragging.kind === 'entry' ? (previewLine ?? {}) : dragging
    if (kind) emit('set-exits', { positionId: dragging.id, tp: kind === 'tp' ? newPrice : position.tpPrice, sl: kind === 'sl' ? newPrice : position.slPrice })
  }
  removePreview()
  dragging = null
  syncPositionLines()
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
  drawFull()
  chart.timeScale().fitContent()
  chart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
    if (range && range.from < 20) loadMore()
    syncTimeScale(chart, range)
  })
  syncPositionLines()
  box.value.addEventListener('pointerdown', onPointerDown, true)
})

let lastScope = `${props.symbol}:${props.interval}`
watch(() => props.candles, (rows, old) => {
  const scope = `${props.symbol}:${props.interval}`
  // a fresh symbol or interval brings a whole new set of bars - the old zoom/pan range (set for
  // the previous timeframe's bar count) means nothing against it, so start fitted again
  const fresh = (!old?.length && rows.length) || scope !== lastScope
  if (fresh) {
    drawFull()
    chart?.timeScale().fitContent()
  } else {
    drawUpdate()
  }
  lastScope = scope
})
watch(() => props.tick, (tick) => {
  const d = Math.max(0, Math.round(-Math.log10(tick || 0.01)))
  candleSeries?.applyOptions({ priceFormat: { type: 'price', precision: d, minMove: tick || 0.01 } })
})
watch(() => [props.positions, props.symbol], syncPositionLines, { deep: true })

onBeforeUnmount(() => {
  resizer?.disconnect()
  box.value?.removeEventListener('pointerdown', onPointerDown, true)
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', onPointerUp)
  chart?.remove()
  rsiChart?.remove()
  stochChart?.remove()
})

const symbolOptions = () => {
  const list = props.symbols.map((s) => ({ value: s.symbol, label: s.symbol }))
  return list.length ? list : [{ value: props.symbol, label: props.symbol }]
}
</script>

<template>
  <section class="panel chart-panel">
    <header class="strip">
      <GlassSelect class="picker" searchable placeholder="Symbol" :model-value="symbol" :options="symbolOptions()" @update:model-value="emit('update:symbol', $event)" />
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
    <div class="indicators" role="group" aria-label="Indicators">
      <button :class="{ on: showSma20 }" :aria-pressed="showSma20" @click="toggleSma(20)">SMA 20</button>
      <button :class="{ on: showSma50 }" :aria-pressed="showSma50" @click="toggleSma(50)">SMA 50</button>
      <button :class="{ on: showRsi }" :aria-pressed="showRsi" @click="toggleOscillator('rsi')">RSI 14</button>
      <button :class="{ on: showStoch }" :aria-pressed="showStoch" @click="toggleOscillator('stoch')">Stoch 14/3</button>
    </div>
    <div ref="box" class="chart" aria-label="Price chart"></div>
    <div v-show="showRsi" class="oscillator-row">
      <span class="oscillator-label">RSI 14</span>
      <div ref="rsiBox" class="oscillator-chart" aria-label="RSI"></div>
    </div>
    <div v-show="showStoch" class="oscillator-row">
      <span class="oscillator-label">Stochastic 14/3</span>
      <div ref="stochBox" class="oscillator-chart" aria-label="Stochastic"></div>
    </div>
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

.indicators {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0 16px 6px;
}

.indicators button {
  padding: 3px 9px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: none;
  color: var(--text-dim);
  font: inherit;
  font-size: 11px;
  cursor: pointer;
}

.indicators button.on {
  background: color-mix(in srgb, var(--accent) 25%, transparent);
  border-color: var(--accent);
  color: var(--text);
}

.oscillator-row {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  border-top: 1px solid var(--line);
  padding: 4px 8px 6px;
}

.oscillator-label {
  font-size: 10px;
  letter-spacing: 0.12em;
  color: var(--text-dim);
  padding: 2px 4px;
}

.oscillator-chart {
  flex: 1;
  min-height: 0;
}

.chart {
  flex: 3;
  min-height: 0;
  padding: 0 8px 8px;
}
</style>
