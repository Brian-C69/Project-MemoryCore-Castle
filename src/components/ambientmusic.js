// Whether Castle's own player should play: the WORLD room's music switch is on, the WORLD room
// itself is on, and no world window on another screen is already playing its own music (a world
// window on a saved-as-on screen that is not really open, such as an unplugged monitor, does not
// count - screensOn already reflects only windows that are genuinely open).
export function shouldPlay({ music, roomOn, screensOn }) {
  return Boolean(music && roomOn && !screensOn)
}
