import { describe, expect, it } from 'vitest'
import { clientId, countdown, decimalsOf, openedBy, parsePaperMoney, price, qtyForShare, signedUsd, toStep, tone, usd } from './market.js'

describe('the MARKET room', () => {
  it('says money and prices plainly', () => {
    expect(usd(10182.4)).toBe('USD 10,182.40')
    expect(signedUsd(182.4)).toBe('+USD 182.40')
    expect(signedUsd(-7.96)).toBe('-USD 7.96')
    expect(decimalsOf(0.1)).toBe(1)
    expect(decimalsOf(0.0001)).toBe(4)
    expect(decimalsOf(1)).toBe(0)
    expect(price(67432.18, 0.1)).toBe('67,432.2')
    expect(price(undefined)).toBe('—')
    expect(tone(3)).toBe('gain')
    expect(tone(-0.5)).toBe('loss')
    expect(tone(0)).toBe('')
  })

  it('names who opened a position', () => {
    expect(openedBy('MEMBER', 'Lyra')).toBe('You')
    expect(openedBy('COMPANION_ASKED', 'Lyra')).toBe('Lyra')
    expect(openedBy('COMPANION_AUTO', 'Lyra')).toBe('Lyra (on its own)')
  })

  it('counts down to the next funding', () => {
    const now = 1_000_000
    expect(countdown(now + (2 * 3600 + 14 * 60 + 9) * 1000, now)).toBe('02:14:09')
    expect(countdown(now - 1, now)).toBe('')
  })

  it('turns a share of the money into a quantity on the step', () => {
    expect(toStep(0.12345, 0.001)).toBe(0.123)
    expect(toStep(-1, 0.001)).toBe(0)
    // 1000 * 0.5 * 0.995 * 10 / 50000 = 0.0995 -> 0.099 on a 0.001 step
    expect(qtyForShare(1000, 0.5, 50000, 10, 0.001)).toBe(0.099)
    expect(qtyForShare(0, 1, 50000, 10, 0.001)).toBe(0)
  })

  it('reads a paper-money amount and makes one id per click', () => {
    expect(parsePaperMoney('10,000')).toBe(10000)
    expect(parsePaperMoney('0')).toBe(null)
    expect(parsePaperMoney('2000000000')).toBe(null)
    expect(parsePaperMoney('abc')).toBe(null)
    expect(clientId()).not.toBe(clientId())
    expect(clientId('member')).toMatch(/^member-/)
  })
})
