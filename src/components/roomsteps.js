// The CHAT room's door, as three steps: sign in on the Library, the Library card, the MemoryCore
// Foundation. Signing in happens on the Library page in the member's browser; Castle then connects
// to that sign-in through the Portfolio's desktop page ("Continue as <name>", one click).

export const LIBRARY_OPENED_KEY = 'castle.room.libraryOpened'

const FOUNDATION = {
  NONE: 'Join the MemoryCore Foundation on your Library card (the MemoryCore tab).',
  PENDING: 'Waiting for Kiyo to promote you - Castle checks by itself, nothing to paste.',
  REJECTED: 'This request was not accepted. You can ask again on your Library card.',
  REVOKED: 'Your Foundation place was removed, so the room is closed to you.',
}

/** The first step's line: what the member does next to sign in. */
export function signInLine(member, libraryOpened) {
  if (member?.signedIn) return `Signed in${member.name ? ` as ${member.name}` : ''}.`
  if (member?.expired) return 'Your sign-in ran out - press Connect Castle to sign in again.'
  if (libraryOpened) return 'Signed in on the Library? Press Connect Castle, and your browser asks once to continue as you.'
  return 'Castle opens the Library in your browser; sign in there with Google.'
}

/** The three steps for the member's state ({ signedIn, card, foundation, roomOpen, name, expired }). */
export function roomSteps(member, libraryOpened = false) {
  const signed = !!member?.signedIn
  const card = member?.card === 'active'
  const f = member?.foundation ?? 'NONE'
  const foundationLine = !signed || !card
    ? 'After your Library card, join the MemoryCore Foundation on its MemoryCore tab.'
    : f === 'APPROVED'
      ? member?.roomOpen ? 'You are in.' : 'Promoted - opening the room…'
      : FOUNDATION[f] ?? 'Castle is checking your Foundation place…'
  return [
    { n: 1, title: 'Sign in with Google', line: signInLine(member, libraryOpened), state: signed ? 'done' : 'todo' },
    {
      n: 2,
      title: 'Your Library card',
      line: !signed
        ? 'Made for you when you sign in.'
        : card
          ? 'Your card is active.'
          : member?.card === 'inactive'
            ? 'Your Library card is not active yet - open the Library to set it up.'
            : 'Castle is checking your card…',
      state: card ? 'done' : signed ? 'wait' : 'todo',
    },
    { n: 3, title: 'The MemoryCore Foundation', line: foundationLine, state: f === 'APPROVED' ? 'done' : f === 'PENDING' ? 'wait' : 'todo' },
  ]
}

/** Whether this viewer already opened the Library from Castle (remembered on this computer only). */
export function rememberedLibraryOpened(storage = globalThis.localStorage) {
  try {
    return storage?.getItem(LIBRARY_OPENED_KEY) === '1'
  } catch {
    return false
  }
}

export function rememberLibraryOpened(storage = globalThis.localStorage) {
  try {
    storage?.setItem(LIBRARY_OPENED_KEY, '1')
  } catch {
    // private window or blocked storage: the step simply asks again next time
  }
}
