// The MARKET room's logic, shared by its views: the state every 2 s and the candles every 5 s while
// the room is on screen, the symbol and interval, and every action the member takes. Layout lives in
// the components; nothing here draws.
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import {
  marketCancel, marketCandles, marketClose, marketCloseRun, marketExits, marketFees, marketHistory, marketLeverage,
  marketMargin, marketNewRun, marketPlace, marketPreview, marketRuns, marketState, marketSymbols, onMarketChanged,
} from '../api/castle.js'

const SYMBOL_KEY = 'castle.market.symbol'
const INTERVAL_KEY = 'castle.market.interval'

function remembered(key, fallback) {
  try {
    return localStorage.getItem(key) || fallback
  } catch {
    return fallback
  }
}

function remember(key, value) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // the choice is simply not kept
  }
}

export function useMarket() {
  const symbol = ref(remembered(SYMBOL_KEY, 'BTCUSDT'))
  const interval = ref(remembered(INTERVAL_KEY, '15m'))
  const state = ref(null)
  const candles = ref([])
  const symbols = ref([])
  const problem = ref('')
  const notice = ref('')
  const busy = ref('')
  const now = ref(Date.now())
  let stateTimer
  let candleTimer
  let clockTimer
  let stopListening
  let stateTurn = 0
  let candleTurn = 0

  const run = computed(() => state.value?.run ?? null)
  const account = computed(() => state.value?.account ?? null)
  const quote = computed(() => state.value?.quote ?? null)
  const positions = computed(() => state.value?.positions ?? [])
  const orders = computed(() => state.value?.orders ?? [])
  const instrument = computed(() => state.value?.instrument ?? null)
  const setting = computed(() => state.value?.setting ?? null)
  const feed = computed(() => state.value?.feed ?? null)

  const visible = () => document.visibilityState === 'visible'

  async function readState() {
    if (!visible()) return
    const mine = ++stateTurn
    try {
      const answer = await marketState(symbol.value)
      if (mine === stateTurn) {
        state.value = answer
        problem.value = ''
      }
    } catch (reason) {
      if (mine === stateTurn) problem.value = String(reason?.message ?? reason)
    }
  }

  async function readCandles() {
    if (!visible()) return
    const mine = ++candleTurn
    try {
      const answer = await marketCandles(symbol.value, interval.value, 300)
      if (mine === candleTurn) candles.value = answer
    } catch {
      // the chart keeps its last candles
    }
  }

  async function readSymbols() {
    try {
      symbols.value = await marketSymbols()
    } catch {
      symbols.value = []
    }
  }

  /** Runs one action, says what went wrong in plain words, and reads the state again. */
  async function act(label, call, said = '') {
    busy.value = label
    problem.value = ''
    notice.value = ''
    try {
      const answer = await call()
      notice.value = said
      await readState()
      return answer
    } catch (reason) {
      problem.value = String(reason?.message ?? reason)
      return null
    } finally {
      busy.value = ''
    }
  }

  const place = (order) => act('Placing…', () => marketPlace(order), 'Order placed.')
  const preview = (order) => marketPreview(order)
  const cancel = (orderId) => act('Cancelling…', () => marketCancel(orderId), 'Order cancelled.')
  const close = (positionId) => act('Closing…', () => marketClose(positionId), 'Position closed.')
  const setExits = (positionId, tp, sl) => act('Saving…', () => marketExits(positionId, tp, sl), 'Exits saved.')
  const addMargin = (positionId, amount) => act('Adding…', () => marketMargin(positionId, amount), 'Margin added.')
  const setLeverage = (marginMode, leverage) => act('Saving…', () => marketLeverage(symbol.value, marginMode, leverage), 'Leverage saved.')
  const startRun = (money) => act('Starting…', () => marketNewRun(money), 'Your paper money is ready.')
  const closeRun = () => act('Closing the account…', () => marketCloseRun(), 'The account is closed. It stays readable in History.')
  const setFees = (taker, maker) => act('Saving…', () => marketFees(taker, maker), 'Fees saved.')
  const history = (kind, runId, page) => marketHistory(kind, runId ?? null, page)
  const runs = () => marketRuns()

  watch(symbol, (s) => {
    remember(SYMBOL_KEY, s)
    candles.value = []
    readState()
    readCandles()
  })
  watch(interval, (i) => {
    remember(INTERVAL_KEY, i)
    candles.value = []
    readCandles()
  })

  onMounted(async () => {
    readState()
    readCandles()
    readSymbols()
    stateTimer = setInterval(readState, 2000)
    candleTimer = setInterval(readCandles, 5000)
    clockTimer = setInterval(() => (now.value = Date.now()), 1000)
    try {
      stopListening = await onMarketChanged(readState)
    } catch {
      // outside Castle's window
    }
  })
  onBeforeUnmount(() => {
    clearInterval(stateTimer)
    clearInterval(candleTimer)
    clearInterval(clockTimer)
    stopListening?.()
  })

  return {
    symbol, interval, state, candles, symbols, problem, notice, busy, now,
    run, account, quote, positions, orders, instrument, setting, feed,
    readState, place, preview, cancel, close, setExits, addMargin, setLeverage, startRun, closeRun, setFees, history, runs,
  }
}
