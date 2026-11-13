// Small plain pieces of Castle's music, shared by the player on every tab (AmbientMusic), the
// top-bar switch (TopMusic) and the world windows (WorldWindow).
import { ambienceFor, songTitle } from '../worlds/ambience.js'
import { DEFAULT_WORLD, findWorld } from '../worlds/registry.js'

// Is a world window really open, the kind that plays the music? Only the screens' own `open` counts
// (a screen saved as on but unplugged has no window), and Castle's own screen never has one.
export const worldWindowOpen = (screens) => (screens ?? []).some((screen) => screen.open && !screen.isMain)

// What the music controls read from the saved settings.
export function musicFrom(settings) {
  const world = settings?.world ?? {}
  return {
    music: !!world.music,
    volume: typeof world.volume === 'number' ? world.volume : 0.5,
    roomOn: settings?.rooms?.world !== false,
  }
}

// The kind of world a screen shows ('2d' or '3d'): its music follows it. An unknown world shows the
// default one, as the stage does.
export function kindFor(worlds, id) {
  return (findWorld(worlds, id) ?? findWorld(worlds, DEFAULT_WORLD))?.kind ?? '3d'
}

// Castle's own music for that kind, then the songs the companion made (`toUrl` turns a file path
// into something the player can load).
export function tracksFor(kind, songs, toUrl) {
  return [...ambienceFor(kind), ...(songs ?? []).map((song) => ({ title: songTitle(song.name), url: toUrl(song.path) }))]
}
