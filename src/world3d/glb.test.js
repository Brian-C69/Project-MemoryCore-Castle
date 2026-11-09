import { describe, expect, it } from 'vitest'
import { fitScale, orbitAt, pieceProblems } from './glb.js'

describe('model pieces', () => {
  it('sizes a piece by its largest side', () => {
    expect(fitScale({ x: 2, y: 4, z: 1 }, 8)).toBe(2)
    expect(fitScale([10, 2, 5], 5)).toBe(0.5)
    expect(fitScale({ x: 0, y: 0, z: 0 }, 5)).toBe(1)
    expect(fitScale({ x: 3, y: 3, z: 3 }, 0)).toBe(1)
  })

  it('circles an islet once per lap at its height', () => {
    const orbit = { radius: 40, height: 6, lap: 240, phase: 0 }
    const start = orbitAt(orbit, 0)
    expect(start.x).toBeCloseTo(0)
    expect(start.z).toBeCloseTo(40)
    expect(start.y).toBe(6)
    const quarter = orbitAt(orbit, 60)
    expect(quarter.x).toBeCloseTo(40)
    expect(quarter.z).toBeCloseTo(0)
    const lap = orbitAt(orbit, 240)
    expect(lap.x).toBeCloseTo(start.x)
    expect(lap.z).toBeCloseTo(start.z)
    expect(Math.hypot(orbitAt(orbit, 97).x, orbitAt(orbit, 97).z)).toBeCloseTo(40)
  })

  it('says plainly what keeps a world from being built', () => {
    const good = { layout: 'centrepiece', pieces: [{ file: 'a.glb', role: 'base', size: 40 }, { file: 'b.glb', role: 'companion', size: 9 }] }
    expect(pieceProblems(good)).toEqual([])
    expect(pieceProblems({ layout: 'centrepiece', pieces: [{ file: 'b.glb', role: 'islet', size: 9 }] })).toContain('a centrepiece world needs exactly one base piece')
    expect(pieceProblems({ layout: 'archipelago', pieces: [{ file: 'i.glb', role: 'isle', size: 9 }] })).toContain('i.glb does not say which isle it is')
    expect(pieceProblems({ layout: 'centrepiece', pieces: [] })).toContain('it has no pieces')
    expect(pieceProblems({ layout: 'centrepiece', pieces: [{ file: 'a.glb', role: 'base' }] })).toContain('a.glb has no size')
  })
})
