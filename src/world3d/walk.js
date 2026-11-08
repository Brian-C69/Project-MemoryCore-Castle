// How the crystal travels between isles: the bridges as a graph, the fewest crossings between two
// isles, the points of the trip, and where it is a given distance along them. Pure, so it is
// tested without a GPU; the world supplies the actual points.

// Map of isle id -> Set of isle ids one bridge away.
export function neighbours(bridges) {
  const map = new Map()
  for (const [a, b] of bridges) {
    if (!map.has(a)) map.set(a, new Set())
    if (!map.has(b)) map.set(b, new Set())
    map.get(a).add(b)
    map.get(b).add(a)
  }
  return map
}

// The isles crossed from -> to, fewest bridges first (breadth-first). [] when unreachable.
export function route(bridges, from, to) {
  if (from === to) return [from]
  const near = neighbours(bridges)
  const back = new Map([[from, null]])
  const queue = [from]
  while (queue.length) {
    const here = queue.shift()
    for (const next of near.get(here) ?? []) {
      if (back.has(next)) continue
      back.set(next, here)
      if (next === to) {
        const path = [to]
        for (let step = here; step !== null; step = back.get(step)) path.unshift(step)
        return path
      }
      queue.push(next)
    }
  }
  return []
}

// The points of a trip along a route. `stand(id)` -> [x, y, z] (where the crystal rests on that
// isle); `bridge(a, b)` -> the points across the bridge from a's edge to b's edge.
export function pathPoints(steps, { stand, bridge }) {
  if (!steps.length) return []
  const points = [stand(steps[0])]
  for (let i = 0; i < steps.length - 1; i++) points.push(...bridge(steps[i], steps[i + 1]))
  if (steps.length > 1) points.push(stand(steps[steps.length - 1]))
  return points
}

// A bridge's curve: `steps` + 1 points from a to b, rising `lift` metres at the middle.
export function arc(a, b, lift = 1.5, steps = 16) {
  const points = []
  for (let i = 0; i <= steps; i++) {
    const t = i / steps
    points.push([
      a[0] + (b[0] - a[0]) * t,
      a[1] + (b[1] - a[1]) * t + Math.sin(Math.PI * t) * lift,
      a[2] + (b[2] - a[2]) * t,
    ])
  }
  return points
}

const dist = (a, b) => Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2])

export function pathLength(points) {
  let total = 0
  for (let i = 1; i < points.length; i++) total += dist(points[i - 1], points[i])
  return total
}

// Where the traveller is `distance` metres along the points, and which way it faces (radians
// about y, 0 = +z). done = it has arrived (distance reached the end).
export function along(points, distance) {
  if (!points.length) return { at: [0, 0, 0], heading: 0, done: true }
  if (points.length === 1) return { at: [...points[0]], heading: 0, done: true }
  let left = Math.max(0, distance)
  let heading = 0
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1]
    const b = points[i]
    const d = dist(a, b)
    // a flat-on-the-ground heading; a zero-length (or straight up) step keeps the last one
    if (Math.hypot(b[0] - a[0], b[2] - a[2]) > 1e-9) heading = Math.atan2(b[0] - a[0], b[2] - a[2])
    if (left <= d || i === points.length - 1) {
      const t = d === 0 ? 1 : Math.min(1, left / d)
      return {
        at: [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t],
        heading,
        done: i === points.length - 1 && left >= d,
      }
    }
    left -= d
  }
  return { at: [...points[points.length - 1]], heading, done: true }
}

// The shortest turn from one heading to another, in radians (-PI..PI).
export function turnBetween(from, to) {
  let d = (to - from) % (Math.PI * 2)
  if (d > Math.PI) d -= Math.PI * 2
  if (d < -Math.PI) d += Math.PI * 2
  return d
}
