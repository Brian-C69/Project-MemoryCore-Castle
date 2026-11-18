import { describe, expect, it } from 'vitest'
import { driveName, freeLine, sizeText, tone, totalsLine, usedPercent } from './disks.js'

describe('disk sizes in words', () => {
  it('writes a size in the unit people use', () => {
    expect(sizeText(0)).toBe('0 B')
    expect(sizeText(512)).toBe('512 B')
    expect(sizeText(465_000_000_000)).toBe('465 GB')
    expect(sizeText(1_800_000_000_000)).toBe('1.8 TB')
    expect(sizeText(2_000_000_000_000)).toBe('2 TB')
    expect(sizeText(9_500_000_000)).toBe('9.5 GB')
  })

  it('reads anything unusable as nothing', () => {
    expect(sizeText(NaN)).toBe('0 B')
    expect(sizeText(-5)).toBe('0 B')
    expect(sizeText(undefined)).toBe('0 B')
  })
})

describe('how full a drive is', () => {
  it('is the part used, rounded', () => {
    expect(usedPercent(1000, 250)).toBe(75)
    expect(usedPercent(1000, 0)).toBe(100)
    expect(usedPercent(1000, 1000)).toBe(0)
  })

  it('stays between empty and full and never divides by nothing', () => {
    expect(usedPercent(0, 0)).toBe(0)
    expect(usedPercent(1000, 5000)).toBe(0)
    expect(usedPercent(1000, -5)).toBe(100)
    expect(usedPercent(1000, NaN)).toBe(100)
  })

  it('warns at 85% and 95%', () => {
    expect(tone(84)).toBe('ok')
    expect(tone(85)).toBe('warn')
    expect(tone(94)).toBe('warn')
    expect(tone(95)).toBe('full')
  })
})

describe('drive names and lines', () => {
  it('names a drive the way its owner does', () => {
    expect(driveName('C:\\')).toBe('C:')
    expect(driveName('d:/')).toBe('D:')
    expect(driveName('/')).toBe('/')
    expect(driveName('/Volumes/Backup')).toBe('Backup')
    expect(driveName(undefined)).toBe('')
  })

  it('says how much is free', () => {
    expect(freeLine({ free: 120_000_000_000, total: 465_000_000_000 })).toBe('120 GB free of 465 GB')
  })

  it('adds the drives together', () => {
    expect(totalsLine([{ free: 100_000_000_000, total: 500_000_000_000 }, { free: 900_000_000_000, total: 1_000_000_000_000 }])).toBe('1 TB free of 1.5 TB')
    expect(totalsLine([])).toBe('0 B free of 0 B')
  })
})
