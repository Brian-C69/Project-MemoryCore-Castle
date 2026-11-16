// Pure indicator math for the MARKET chart - kept free of lightweight-charts and Vue so the
// formulas themselves are simple to test. Every function returns one value per input candle,
// `null` wherever there isn't yet enough history to compute one (so callers can filter those out
// before handing a series to the chart).

/** The simple moving average of `values` over `period` bars; `null` before the window fills. */
export function sma(values, period) {
  if (!(period > 0)) return values.map(() => null)
  const out = new Array(values.length).fill(null)
  let sum = 0
  for (let i = 0; i < values.length; i++) {
    sum += values[i]
    if (i >= period) sum -= values[i - period]
    if (i >= period - 1) out[i] = sum / period
  }
  return out
}

/** Wilder's RSI of `closes` over `period` (14 is the usual default); `null` before it starts. */
export function rsi(closes, period = 14) {
  const out = new Array(closes.length).fill(null)
  if (closes.length <= period) return out
  let gainSum = 0
  let lossSum = 0
  for (let i = 1; i <= period; i++) {
    const diff = closes[i] - closes[i - 1]
    if (diff > 0) gainSum += diff
    else lossSum -= diff
  }
  let avgGain = gainSum / period
  let avgLoss = lossSum / period
  out[period] = rsiOf(avgGain, avgLoss)
  for (let i = period + 1; i < closes.length; i++) {
    const diff = closes[i] - closes[i - 1]
    const gain = diff > 0 ? diff : 0
    const loss = diff < 0 ? -diff : 0
    avgGain = (avgGain * (period - 1) + gain) / period
    avgLoss = (avgLoss * (period - 1) + loss) / period
    out[i] = rsiOf(avgGain, avgLoss)
  }
  return out
}

function rsiOf(avgGain, avgLoss) {
  if (avgLoss === 0) return avgGain === 0 ? 50 : 100
  return 100 - 100 / (1 + avgGain / avgLoss)
}

/**
 * The stochastic oscillator: %K is where the close sits within the `kPeriod` high/low range
 * (0-100), %D is %K's own `dPeriod`-bar moving average. Both `null` before they have enough bars.
 */
export function stochastic(highs, lows, closes, kPeriod = 14, dPeriod = 3) {
  const k = closes.map((close, i) => {
    if (i < kPeriod - 1) return null
    let highest = -Infinity
    let lowest = Infinity
    for (let j = i - kPeriod + 1; j <= i; j++) {
      if (highs[j] > highest) highest = highs[j]
      if (lows[j] < lowest) lowest = lows[j]
    }
    const range = highest - lowest
    return range === 0 ? 50 : (100 * (close - lowest)) / range
  })
  return { k, d: smaOverNulls(k, dPeriod) }
}

/** `sma`, but a `null` resets the averaging window instead of poisoning it with NaN. */
function smaOverNulls(values, period) {
  const out = new Array(values.length).fill(null)
  let windowValues = []
  let sum = 0
  for (let i = 0; i < values.length; i++) {
    const v = values[i]
    if (v === null) {
      windowValues = []
      sum = 0
      continue
    }
    windowValues.push(v)
    sum += v
    if (windowValues.length > period) sum -= windowValues.shift()
    if (windowValues.length === period) out[i] = sum / period
  }
  return out
}
