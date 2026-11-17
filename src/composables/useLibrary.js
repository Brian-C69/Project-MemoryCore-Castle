// LIBRARY's two shelves, kept for the whole run: the member's own books and the Spiral Library's
// catalogue (pulled from the Portfolio over the internet, which is the slow part). Castle reads them
// at start-up behind the start-up screen, so opening LIBRARY shows the shelves at once; the room then
// reads them again quietly and swaps in anything new. A slow or unreachable catalogue never holds
// start-up: the piece has its own short limit in App.vue, and the room just reads it on arrival then.
import { ref } from 'vue'
import { libraryBooks, spiralKept, spiralRoom } from '../api/castle.js'

export const libraryMine = ref([])
export const librarySpiral = ref({ shelves: [], books: [], signedIn: false })
// which Spiral books are already in My books (the edition), by the book's ref
export const libraryKept = ref({})
// true once the catalogue has been read at least once: until then the room says it is loading
export const librarySpiralLoaded = ref(false)

export async function prepareLibrary() {
  // the two reads are independent, and a failure of one leaves the other's shelf
  await Promise.allSettled([
    libraryBooks().then((books) => {
      libraryMine.value = books
    }),
    spiralRoom().then(async (room) => {
      librarySpiral.value = room
      librarySpiralLoaded.value = true
      libraryKept.value = await spiralKept().catch(() => libraryKept.value)
    }),
  ])
}
