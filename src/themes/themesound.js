// A theme's looping sound on HOME (theme.json "motionSound"), kept pure so it is tested: when it is
// heard, and how loud next to the music.

// The sea sits under any music: this share of the music slider's level.
export const SOUND_SHARE = 0.6

export function shouldSound({ on = false, visible = false, awake = false } = {}) {
  return Boolean(on && visible && awake)
}

// The music slider's level (0..1) as the sound's gain, never outside 0..1.
export function soundLevel(volume) {
  const v = Number(volume)
  if (!Number.isFinite(v)) return SOUND_SHARE * 0.5
  return Math.min(1, Math.max(0, v)) * SOUND_SHARE
}
