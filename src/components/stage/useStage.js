// The STORY stage's behaviour, shared by every look (classic, cinema, book, messenger, jrpg).
// The looks only draw what this returns; they never fetch or decide anything themselves.
//
// Beats come from the world's beat cards (story_beat, polled while the stage is shown), or - when
// the story has written no card yet - from its plain reply (the `castle-story-reply` window event
// the hidden story chat sends). Each beat is prepared (its scene picture loaded) before it plays;
// a beat that arrives while one is playing waits for Continue.
import { computed, onBeforeUnmount, onMounted, reactive, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { storyBeat } from '../../api/castle.js'
import { castFor, findScene, inWorld, initialOf, readBeatCard, readStageText, toPages } from './stage.js'

const POLL_MS = 1200
const TYPE_MS = 22
const TYPE_STEP = 2
const CLEAN_MS = 2000
const MUSIC_LEVEL = 0.45
const MUSIC_UNDER_VOICE = 0.08

const url = (path) => {
  if (!path) return null
  try {
    return convertFileSrc(path)
  } catch {
    return path
  }
}

const preload = (src) =>
  new Promise((resolve) => {
    if (!src) return resolve()
    const img = new Image()
    img.onload = img.onerror = () => resolve()
    img.src = src
  })

/**
 * world: () => World (from story_worlds), companion: () => name, portrait: () => path,
 * active: () => boolean, say: (text) => void (typed into the story session)
 */
export function useStage({ world, companion, portrait, active, say }) {
  const s = reactive({
    shown: null, // the beat on stage
    waiting: null, // the next beat, prepared, until Continue
    history: [], // every beat seen this visit (the messenger look scrolls through them)
    page: 0,
    typed: '',
    typing: false,
    scene: null, // the scene picture's URL
    clean: false, // the first seconds of a new scene: the picture alone
    preparing: false,
    music: true,
    volume: 0.5,
    cinema: false,
  })
  let lastBeat = 0
  let lastText = ''
  let poll = 0
  let typer = 0
  let cleanTimer = 0
  const bgm = typeof Audio === 'undefined' ? null : new Audio()
  const voice = typeof Audio === 'undefined' ? null : new Audio()
  if (bgm) bgm.loop = true

  const pages = computed(() => toPages(s.shown?.lines ?? []))
  const current = computed(() => pages.value[s.page] ?? null)
  const atEnd = computed(() => !s.typing && s.page >= pages.value.length - 1)
  const choices = computed(() => (atEnd.value ? s.shown?.choices ?? [] : []))

  function faceOf(speaker) {
    const w = world()
    return url(castFor(speaker, w?.cast ?? [], companion(), portrait()))
  }
  const initial = initialOf

  // ---------- music ----------
  function musicSource() {
    const w = world()
    return w?.music?.length ? url(w.music[0]) : '/ambience/castle-theme.mp3'
  }
  function syncMusic() {
    if (!bgm) return
    const want = musicSource()
    if (bgm.dataset?.src !== want) {
      bgm.src = want
      if (bgm.dataset) bgm.dataset.src = want
    }
    bgm.volume = s.volume * (voice && !voice.paused ? MUSIC_UNDER_VOICE : MUSIC_LEVEL) * 2
    if (s.music && active()) bgm.play().catch(() => {})
    else bgm.pause()
  }

  // ---------- pages ----------
  function startPage() {
    clearInterval(typer)
    const text = current.value?.text ?? ''
    s.typed = ''
    s.typing = true
    typer = setInterval(() => {
      s.typed = text.slice(0, s.typed.length + TYPE_STEP)
      if (s.typed.length >= text.length) {
        s.typing = false
        clearInterval(typer)
      }
    }, TYPE_MS)
    const v = current.value?.voice
    if (voice && v) {
      voice.src = url(inWorld(world()?.folder ?? '', v)) ?? ''
      voice.onended = syncMusic
      voice.play().then(syncMusic).catch(() => {})
    }
  }

  function show(beat) {
    const scene = beat.sceneUrl ?? s.scene
    if (scene && scene !== s.scene) {
      s.scene = scene
      s.clean = true
      clearTimeout(cleanTimer)
      cleanTimer = setTimeout(() => (s.clean = false), CLEAN_MS)
    }
    s.shown = beat
    s.history = [...s.history, beat].slice(-60)
    s.page = 0
    startPage()
  }

  async function offer(beat) {
    const w = world()
    const path = findScene(beat.scene, w?.scenes ?? [], w?.folder ?? '')
    beat.sceneUrl = url(path)
    s.preparing = true
    await preload(beat.sceneUrl)
    s.preparing = false
    if (!s.shown) show(beat)
    else s.waiting = beat
  }

  // a click: finish the line, then the next page, then the waiting beat
  function next() {
    if (s.typing) {
      clearInterval(typer)
      s.typed = current.value?.text ?? ''
      s.typing = false
      return
    }
    if (s.page < pages.value.length - 1) {
      s.page += 1
      startPage()
    } else if (s.waiting) {
      const beat = s.waiting
      s.waiting = null
      show(beat)
    }
  }

  function choose(index) {
    const text = s.shown?.choices?.[index] ?? ''
    // "(Say something)" opens the say box instead of being sent
    if (/^\(?say something\)?$/i.test(text.trim())) return 'say'
    say(String(index + 1))
    return 'sent'
  }

  function speak(text) {
    const t = String(text ?? '').trim()
    if (t) say(t)
  }

  // ---------- where beats come from ----------
  async function checkCard() {
    const w = world()
    if (!w?.id || !active()) return
    try {
      const card = readBeatCard(await storyBeat(w.id))
      if (card && card.beat && card.beat > lastBeat && card.lines.length) {
        lastBeat = card.beat
        await offer(card)
      }
    } catch {
      // the story room may be off, or outside Castle
    }
  }

  function onReply(event) {
    // the plain reply is only a fallback: once a world writes cards, the cards lead
    if (lastBeat) return
    const text = String(event.detail ?? '')
    if (!text || text === lastText) return
    lastText = text
    const beat = readStageText(text)
    if (beat.lines.length) offer(beat)
  }

  function reset() {
    lastBeat = 0
    lastText = ''
    s.shown = null
    s.waiting = null
    s.history = []
    s.scene = null
    const w = world()
    if (w?.scenes?.length) s.scene = url(w.scenes[w.scenes.length - 1])
  }

  onMounted(() => {
    reset()
    window.addEventListener('castle-story-reply', onReply)
    poll = setInterval(checkCard, POLL_MS)
    checkCard()
    syncMusic()
  })
  onBeforeUnmount(() => {
    clearInterval(poll)
    clearInterval(typer)
    clearTimeout(cleanTimer)
    window.removeEventListener('castle-story-reply', onReply)
    bgm?.pause()
    voice?.pause()
  })
  watch(() => world()?.id, () => {
    reset()
    checkCard()
    syncMusic()
  })
  watch(() => [s.music, s.volume, active()], syncMusic)

  return { s, pages, current, atEnd, choices, faceOf, initial, next, choose, speak }
}
