// The one line a room shows about its picture requests, in place of a list: the newest request
// still drawing or waiting, else the newest one that could not be drawn. Finished requests are
// left out (the picture itself shows they are done) and cleared by the line's owner.
const ACTIVE = ['queued', 'working', 'switching']

export function requestLine(jobs = []) {
  const newest = [...jobs].reverse()
  return newest.find((j) => ACTIVE.includes(j.phase)) ?? newest.find((j) => j.phase === 'prompt-saved') ?? null
}

export function finishedIds(jobs = []) {
  return jobs.filter((j) => j.phase === 'completed').map((j) => j.id)
}
