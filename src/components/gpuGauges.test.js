import { describe, expect, it } from 'vitest'
import { gpuTitle } from './gpuGauges.js'

describe('the graphics-card gauges', () => {
  it('describes a card on hover, leaving out what is unknown', () => {
    const card = { name: 'NVIDIA GeForce RTX 3070', util: 4, memUsedMb: 7781, memTotalMb: 8192, tempC: 45 }
    expect(gpuTitle(card)).toBe('NVIDIA GeForce RTX 3070 · 4% busy · VRAM 7.6 GB of 8.0 GB · 45°C')
    expect(gpuTitle({ ...card, memTotalMb: 0, tempC: null })).toBe('NVIDIA GeForce RTX 3070 · 4% busy')
    expect(gpuTitle({ ...card, tempC: undefined })).not.toContain('°C')
  })
})
