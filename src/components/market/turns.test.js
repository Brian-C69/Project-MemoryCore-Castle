import { describe, expect, it } from 'vitest'
import { actionsOf, chatLines, chipKind, chipOf, lastTrade, percent, tagOf, turnCounts, unfinishedOf } from './turns.js'

const buy = { action: { type: 'place_order', symbol: 'ETHUSDT', side: 'LONG', orderType: 'MARKET', qty: 0.05 }, result: 'Filled', ok: true }
const limitSell = { action: { type: 'place_order', symbol: 'BTCUSDT', side: 'SHORT', orderType: 'LIMIT', qty: 0.01, price: 68000 }, result: 'Resting', ok: true }
const close = { action: { type: 'close_position', symbol: 'SOLUSDT' }, result: 'No open position on SOLUSDT.', ok: false }

describe('the companion at the desk', () => {
  it('reads a turn and draws its chips', () => {
    expect(actionsOf({ actionsJson: JSON.stringify([buy]) })).toHaveLength(1)
    expect(actionsOf({ actionsJson: 'broken' })).toEqual([])
    expect(chipOf(buy)).toBe('Buy ETHUSDT 0.05')
    expect(chipOf(limitSell)).toBe('Sell BTCUSDT 0.01 @ 68,000')
    expect(chipOf(close)).toBe('Close SOLUSDT')
    expect([chipKind(buy), chipKind(limitSell), chipKind(close)]).toEqual(['buy', 'sell', 'close'])
    expect(tagOf({ triggerKind: 'AUTO' })).toBe('On its own')
    expect(tagOf({ triggerKind: 'ASKED' })).toBe('You asked')
  })

  it('makes the chat from the newest turns, oldest line first', () => {
    const turns = [
      { id: 2, message: null, reply: 'No setup right now.', createdAt: 2, status: 'DONE', triggerKind: 'AUTO' },
      { id: 1, message: 'buy a small ETH long', reply: 'Done, stop below 3,100.', createdAt: 1, status: 'DONE', actionsJson: JSON.stringify([buy]) },
    ]
    expect(chatLines(turns).map((l) => l.text)).toEqual(['buy a small ETH long', 'Done, stop below 3,100.', 'No setup right now.'])
    expect(lastTrade(turns)).toMatchObject({ chip: 'Buy ETHUSDT 0.05', kind: 'buy' })
    expect(lastTrade([{ actionsJson: JSON.stringify([close]) }])).toBe(null)
  })

  it('shows a lesson result as a percent', () => {
    expect(percent(1.8)).toBe('+1.80%')
    expect(percent(-0.9)).toBe('-0.90%')
  })
})

describe('the Trade log', () => {
  it('counts what the shown turns did', () => {
    const turns = [
      { status: 'DONE', actionsJson: '[]' },
      { status: 'DONE', actionsJson: JSON.stringify([buy, close]) },
      { status: 'FAILED', actionsJson: null },
    ]
    expect(turnCounts(turns)).toEqual({ turns: 3, done: 1, refused: 1, noTrade: 1 })
    expect(turnCounts([])).toEqual({ turns: 0, done: 0, refused: 0, noTrade: 0 })
  })

  it('says why a turn did not finish', () => {
    expect(unfinishedOf({ status: 'DONE' })).toBeNull()
    expect(unfinishedOf({ status: 'RUNNING' })).toBe('Thinking…')
    expect(unfinishedOf({ status: 'STOPPED' })).toBe('Stopped')
    expect(unfinishedOf({ status: 'FAILED' })).toBe('Did not finish')
  })
})
