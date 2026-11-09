// What the STORY stage shows for one beat. First choice: the beat card the story writes with
// castle-beat.ps1 (<world>/stage/beat-NNNN.json). Fallback: its plain reply, read the same way.
//
// A beat: { beat, chapter, scene, lines: [{ speaker, text, voice }], choices: [text] }
//   scene   - a path inside the world folder, or a scene name to look up, or null (keep the last)
//   speaker - null for narration, else a name from the world's setting

const PAGE_CHARS = 260

// The five looks a member can choose (Settings › Story, or Look on the STORY bar).
export const LOOKS = [
  { id: 'classic', title: 'Classic VN', line: 'The scene fills the screen, the cast stands over it, a glass box with a name plate.' },
  { id: 'cinema', title: 'Cinematic', line: 'Letterboxed like a film, the words as subtitles, choices at the side.' },
  { id: 'book', title: 'Storybook', line: 'An open book: the scene on the left page, the story on the right.' },
  { id: 'messenger', title: 'Messenger', line: 'The story as a chat thread with the cast, choices as quick replies.' },
  { id: 'jrpg', title: 'JRPG window', line: 'A framed window with the speaker’s big face, choices under it.' },
]

const plain = (text) =>
  String(text ?? '')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/(^|[^*])\*([^*\n]+)\*/g, '$1$2')
    .replace(/`([^`]+)`/g, '$1')
    .trim()

const textOr = (v) => (typeof v === 'string' && v.trim() ? v.trim() : null)

/** A beat card read defensively: anything missing becomes empty, never an error. */
export function readBeatCard(card) {
  if (!card || typeof card !== 'object') return null
  const lines = (Array.isArray(card.lines) ? card.lines : [])
    .map((line) => ({ speaker: textOr(line?.speaker), text: plain(line?.text), voice: textOr(line?.voice) }))
    .filter((line) => line.text)
  return {
    beat: Number.isFinite(card.beat) ? card.beat : null,
    chapter: typeof card.chapter === 'string' ? card.chapter.trim() : '',
    scene: textOr(card.scene),
    lines,
    choices: (Array.isArray(card.choices) ? card.choices : []).map(plain).filter(Boolean),
  }
}

const SCENE = /^\s*\[scene:\s*([^\]]+)\]\s*$/i
const SPEECH = /^\**([\p{L}][\p{L}\p{N}' .-]{0,30}?)\**\s*:\s*[“"](.+)[”"]\s*$/u
const CHOICE = /^\s*(\d+)[.)]\s+(.+)$/
const CHAPTER = /^#{1,3}\s+(.+)$/

/**
 * A plain reply read as a beat: `[scene: name]` sets the backdrop, `Name: "line"` is speech,
 * other paragraphs are narration, the closing numbered list is the choices, a heading the chapter.
 */
export function readStageText(text) {
  const rows = String(text ?? '')
    .replace(/\r/g, '')
    .split('\n')
  let scene = null
  let chapter = ''
  const lines = []
  let choices = []
  let paragraph = []
  const flush = () => {
    if (paragraph.length) lines.push({ speaker: null, text: plain(paragraph.join(' ')), voice: null })
    paragraph = []
  }
  rows.forEach((raw, index) => {
    const row = raw.trim()
    if (!row) return flush()
    const tag = row.match(SCENE)
    const heading = row.match(CHAPTER)
    const choice = row.match(CHOICE)
    const speech = row.match(SPEECH)
    if (tag) {
      flush()
      scene = tag[1].trim()
    } else if (heading && !chapter) {
      flush()
      chapter = plain(heading[1])
    } else if (choice && rows.slice(index + 1).every((r) => !r.trim() || CHOICE.test(r.trim()))) {
      // the closing numbered list is the choices
      flush()
      choices.push(plain(choice[2]))
    } else if (speech) {
      flush()
      lines.push({ speaker: speech[1].trim(), text: plain(speech[2]), voice: null })
    } else {
      paragraph.push(row)
    }
  })
  flush()
  return { beat: null, chapter, scene, lines: lines.filter((l) => l.text), choices }
}

/** Long narration split into pages at sentence ends, so the box never overflows. */
export function toPages(lines, limit = PAGE_CHARS) {
  const pages = []
  for (const line of lines) {
    if (line.text.length <= limit || line.voice) {
      pages.push(line)
      continue
    }
    const sentences = line.text.match(/[^.!?…]+[.!?…]+["”’)]*\s*|[^.!?…]+$/g) ?? [line.text]
    let page = ''
    for (const sentence of sentences) {
      if (page && (page + sentence).length > limit) {
        pages.push({ ...line, text: page.trim() })
        page = ''
      }
      page += sentence
    }
    if (page.trim()) pages.push({ ...line, text: page.trim() })
  }
  return pages
}

const slug = (text) => String(text ?? '').toLowerCase().replace(/[^\p{L}\p{N}]+/gu, '-').replace(/^-|-$/g, '')

/** A path inside the world folder made whole (forward or back slashes). */
export function inWorld(folder, rel) {
  if (!rel) return null
  if (/^[a-z]:[\\/]/i.test(rel) || rel.startsWith('/') || rel.includes('..')) return null
  const sep = folder.includes('\\') ? '\\' : '/'
  return `${folder.replace(/[\\/]+$/, '')}${sep}${rel.replace(/[\\/]+/g, sep)}`
}

/** The scene picture for a beat: an exact path inside the world, else a scene whose name holds the words. */
export function findScene(scene, scenes, folder = '') {
  if (!scene) return null
  const list = scenes ?? []
  const want = scene.replaceAll('\\', '/')
  const exact = list.find((s) => s.replaceAll('\\', '/').endsWith(want))
  if (exact) return exact
  const words = slug(scene)
  const named = list.find((s) => slug(s.split(/[\\/]/).pop()).includes(words))
  if (named) return named
  return want.includes('/') && folder ? inWorld(folder, scene) : null
}

/**
 * The face for a speaker: their cast portrait (matched by name, case and punctuation aside), the
 * companion's own portrait, else null (the look draws the first letter).
 */
export function castFor(speaker, cast = [], companion = '', companionPortrait = null) {
  if (!speaker) return null
  const want = slug(speaker)
  const found = cast.find((c) => slug(c.name) === want) ?? cast.find((c) => want && (slug(c.name).includes(want) || want.includes(slug(c.name))))
  if (found) return found.path
  if (companion && slug(companion) === want) return companionPortrait
  return null
}

export const initialOf = (speaker) => (speaker ? String(speaker).trim()[0]?.toUpperCase() ?? '?' : '✦')
