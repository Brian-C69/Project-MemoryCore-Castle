// The parts of Castle's audio player that are not drawing: the speeds it cycles through, the volume
// it remembers between clips and between launches, and "only one clip plays at a time".
import { reactive } from 'vue'

// Click the speed button to go to the next one; after the last it comes back to normal.
export const RATES = [1, 1.25, 1.5, 2, 0.75]

// The next speed after this one (a speed that is not in the list starts again from normal).
export function nextRate(rate) {
  const at = RATES.indexOf(rate)
  return RATES[(at + 1) % RATES.length]
}

// "1×", "1.25×", "0.75×"
export const rateLabel = (rate) => `${rate}×`

// A volume the audio element accepts: 0 to 1, anything unusable is full volume.
export function clampVolume(volume) {
  const value = Number(volume)
  return Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 1
}

const KEY = 'castle.player'

// The remembered volume and mute, or full volume when nothing (or nothing readable) was kept.
export function loadPrefs(storage) {
  try {
    const kept = JSON.parse(storage.getItem(KEY))
    return { volume: clampVolume(kept?.volume ?? 1), muted: kept?.muted === true }
  } catch {
    return { volume: 1, muted: false }
  }
}

// Keeps them; a storage that refuses (private window, blocked site data) is not an error.
export function savePrefs(storage, prefs) {
  try {
    storage.setItem(KEY, JSON.stringify({ volume: clampVolume(prefs.volume), muted: !!prefs.muted }))
  } catch {
    // the volume simply is not remembered
  }
}

// Only one clip plays at a time: a clip that starts stops the one that was playing.
export function playbackRegistry() {
  let current = null
  return {
    claim(id, stop) {
      if (current && current.id !== id) current.stop()
      current = { id, stop }
    },
    release(id) {
      if (current?.id === id) current = null
    },
  }
}

export const sharedPlayback = playbackRegistry()

// One volume for every player in the app, so changing it in one is heard in all of them.
const storage = (() => {
  try {
    return globalThis.localStorage
  } catch {
    return undefined
  }
})()
export const playerPrefs = reactive(storage ? loadPrefs(storage) : { volume: 1, muted: false })

export function setVolume(volume) {
  playerPrefs.volume = clampVolume(volume)
  playerPrefs.muted = playerPrefs.volume === 0
  if (storage) savePrefs(storage, playerPrefs)
}

export function toggleMute() {
  playerPrefs.muted = !playerPrefs.muted
  // un-muting from zero would stay silent, so it comes back to a hearable level
  if (!playerPrefs.muted && playerPrefs.volume === 0) playerPrefs.volume = 0.6
  if (storage) savePrefs(storage, playerPrefs)
}
