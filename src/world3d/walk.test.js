import { describe, expect, it } from 'vitest'
import { along, arc, neighbours, pathLength, pathPoints, route, turnBetween } from './walk.js'

const BRIDGES = [
  ['heart', 'east'],
  ['heart', 'north'],
  ['heart', 'west'],
  ['east', 'north'],
  ['far', 'away'],
]

describe('routes along the bridges', () => {
  it('lists each isle\'s neighbours both ways', () => {
    const near = neighbours(BRIDGES)
    expect([...near.get('heart')].sort()).toEqual(['east', 'north', 'west'])
    expect(near.get('east').has('heart')).toBe(true)
  })

  it('takes the fewest crossings', () => {
    expect(route(BRIDGES, 'east', 'north')).toEqual(['east', 'north'])
    expect(route(BRIDGES, 'west', 'east')).toEqual(['west', 'heart', 'east'])
    expect(route(BRIDGES, 'heart', 'heart')).toEqual(['heart'])
  })

  it('gives [] when no bridge leads there', () => {
    expect(route(BRIDGES, 'heart', 'away')).toEqual([])
    expect(route(BRIDGES, 'heart', 'nowhere')).toEqual([])
  })

  it('joins the stand points and bridge points of a trip', () => {
    const stand = (id) => ({ a: [0, 0, 0], b: [10, 0, 0] })[id]
    const bridge = () => [[2, 0, 0], [8, 0, 0]]
    expect(pathPoints(['a', 'b'], { stand, bridge })).toEqual([[0, 0, 0], [2, 0, 0], [8, 0, 0], [10, 0, 0]])
    expect(pathPoints(['a'], { stand, bridge })).toEqual([[0, 0, 0]])
    expect(pathPoints([], { stand, bridge })).toEqual([])
  })
})

describe('bridge arcs', () => {
  it('start and end at the edges and rise in the middle', () => {
    const points = arc([0, 0, 0], [10, 2, 0], 3, 10)
    expect(points).toHaveLength(11)
    expect(points[0]).toEqual([0, 0, 0])
    expect(points[10][0]).toBeCloseTo(10)
    expect(points[10][1]).toBeCloseTo(2)
    expect(points[5][1]).toBeCloseTo(1 + 3)
  })
})

describe('along a path', () => {
  const path = [[0, 0, 0], [0, 0, 10], [10, 0, 10]]

  it('measures the whole path', () => {
    expect(pathLength(path)).toBeCloseTo(20)
    expect(pathLength([[1, 2, 3]])).toBe(0)
  })

  it('starts at the first point facing the first step', () => {
    const step = along(path, 0)
    expect(step.at).toEqual([0, 0, 0])
    expect(step.heading).toBeCloseTo(0) // toward +z
    expect(step.done).toBe(false)
  })

  it('finds the point part way and turns at the corner', () => {
    const mid = along(path, 5)
    expect(mid.at).toEqual([0, 0, 5])
    const second = along(path, 15)
    expect(second.at[0]).toBeCloseTo(5)
    expect(second.at[2]).toBeCloseTo(10)
    expect(second.heading).toBeCloseTo(Math.PI / 2) // toward +x
    expect(second.done).toBe(false)
  })

  it('arrives at the end and stays there past it', () => {
    expect(along(path, 20).done).toBe(true)
    const beyond = along(path, 99)
    expect(beyond.at).toEqual([10, 0, 10])
    expect(beyond.done).toBe(true)
  })

  it('handles one point, no points and negative distance', () => {
    expect(along([[1, 2, 3]], 4)).toEqual({ at: [1, 2, 3], heading: 0, done: true })
    expect(along([], 4).done).toBe(true)
    expect(along(path, -3).at).toEqual([0, 0, 0])
  })
})

describe('turning', () => {
  it('takes the short way round', () => {
    expect(turnBetween(0, Math.PI / 2)).toBeCloseTo(Math.PI / 2)
    expect(turnBetween(Math.PI - 0.1, -Math.PI + 0.1)).toBeCloseTo(0.2)
    expect(turnBetween(0, 4 * Math.PI)).toBeCloseTo(0)
  })
})
