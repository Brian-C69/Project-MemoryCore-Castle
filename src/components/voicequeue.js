// The companion's voice lines play one after another (the member's 2:37 AM word: "the voice only play like 5 second and
// stop. should finished it"). A voice that lands while another plays waits its turn; the member's own click
// always plays at once and clears the line.

export function createVoiceQueue() {
  const waiting = []
  let current = null
  return {
    // a new voice arrived by itself: play it now if nothing plays, else queue it -> what to play now
    arrive(item, isPlaying) {
      if (isPlaying) {
        if (item !== current && !waiting.includes(item)) waiting.push(item)
        return null
      }
      current = item
      return item
    },
    // the playing voice ended -> the next one to play, or null
    ended() {
      current = waiting.shift() ?? null
      return current
    },
    // the member pressed play on a line: it plays now, the queue is cleared
    choose(item) {
      waiting.length = 0
      current = item
      return item
    },
    // the member paused, or the room closed
    stop() {
      waiting.length = 0
      current = null
    },
    get waiting() {
      return [...waiting]
    },
  }
}
