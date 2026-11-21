// What the picture viewer offers at each moment. Looking: zoom, ◀ ▶ between the message's
// pictures, and the pen to mark the one shown. Marking: the markup sheet has the picture, so the
// viewer's own controls rest until the marks are kept or left, and each one says why.

export const MARKING_WHY = 'Finish marking first: use the marked copy, or cancel'

/**
 * The viewer's controls for `count` pictures, while `marking` (the sheet is open) or not, for a
 * picture that is `markable` (a draft picture whose original is loaded). Each control is
 * { shown, on, why }: shown at all, usable now, and the reason when it is resting.
 */
export function viewerControls({ count = 0, marking = false, markable = false } = {}) {
  const control = (shown) => ({ shown, on: shown && !marking, why: shown && marking ? MARKING_WHY : '' })
  return { zoom: control(count > 0), nav: control(count > 1), markup: control(markable), keys: !marking }
}

/** Where the viewer stands in `paths` for the picture `path` (the first one when it is gone). */
export const viewerIndex = (paths, path) => Math.max(0, paths.indexOf(path))
