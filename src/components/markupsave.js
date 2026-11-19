// How a marked copy takes the original's place in the message: the core saves the PNG beside the
// original, the message's words then name the copy instead, and the marks are remembered (by the
// copy's path) so the member can open it again and change them until the message is sent.

const escape = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

/** A path as the message carries it: quoted when it has a space. */
export const quotedPath = (path) => (/\s/.test(path) ? `"${path}"` : path)

/** `draft` with every mention of the picture `from` (quoted or bare) naming `to` instead. */
export function replacePicturePath(draft, from, to) {
  if (!from || from === to) return draft
  const mention = new RegExp(`"${escape(from)}"|(?<=^|[\\s(])${escape(from)}(?=$|[\\s),;!?]|\\.(?:\\s|$))`, 'g')
  return draft.replace(mention, () => quotedPath(to))
}

/** What the sheet opens for the picture `path` shows now: its original, and the marks it carries. */
export function markupSource(marks, path) {
  const known = marks[path]
  return known ? { original: known.original, ops: known.ops, previous: path } : { original: path, ops: [], previous: null }
}

/**
 * After "Use marked copy": the draft and the marks once `shown` (the path in the message now)
 * gives way to `saved` (the new copy, or the original again when every mark was taken away).
 */
export function applyMarked({ draft, marks }, { shown, original, ops, saved }) {
  const next = { ...marks }
  delete next[shown]
  if (ops.length && saved !== original) next[saved] = { original, ops }
  return { draft: replacePicturePath(draft, shown, saved), marks: next }
}

/** The marks kept only for pictures still in the message (sent or removed ones are forgotten). */
export function keepMarks(marks, paths) {
  const still = new Set(paths)
  return Object.fromEntries(Object.entries(marks).filter(([path]) => still.has(path)))
}

/**
 * Saves the marked copy: `picture` is the finished PNG as a data address, `save` the core's
 * chat_markup_save. With no marks left, nothing is saved and the original comes back.
 */
export async function saveMarkedCopy({ original, previous, ops, picture, save }) {
  if (!ops.length) return original
  if (!picture?.startsWith('data:image/png;base64,')) throw new Error('The marked picture could not be made.')
  return save(original, picture, previous)
}
