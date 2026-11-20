// Chat's pictures, films and the companion's voice: files loaded once from the core (which serves
// only files the chat showed, or pictures the member shared), their lengths, and one player that
// plays one voice at a time and never cuts one off.
import { reactive, ref } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { chatMedia, chatMediaStream } from '../api/castle.js'
import { createVoiceQueue } from './voicequeue.js'

export function useChatMedia(room) {
  // path -> data URL (null when it cannot be read)
  const media = reactive({})
  const durations = reactive({})
  const playing = ref(null)
  const player = new Audio()
  // a voice that lands while another plays waits its turn
  const voices = createVoiceQueue()
  const isPlaying = () => !!playing.value && !player.paused && !player.ended

  player.addEventListener('ended', () => {
    playing.value = null
    const next = voices.ended()
    if (next) startVoice(next)
  })

  // A film, or a sound too large for a data URL, is streamed from its file: the core allows that one
  // file and the player reads it as it plays.
  async function streamOf(path) {
    await chatMediaStream(path, room())
    return convertFileSrc(path)
  }

  async function loadMedia(entry) {
    if (!entry.path || media[entry.path]) return media[entry.path]
    try {
      if (entry.role === 'video') {
        media[entry.path] = await streamOf(entry.path)
        return media[entry.path]
      }
      media[entry.path] = await chatMedia(entry.path, room()).catch((reason) => (entry.role === 'voice' ? streamOf(entry.path) : Promise.reject(reason)))
      if (entry.role === 'voice') {
        const probe = new Audio(media[entry.path])
        probe.addEventListener('loadedmetadata', () => (durations[entry.path] = probe.duration))
      }
    } catch {
      media[entry.path] = null
    }
    return media[entry.path]
  }

  const loadPath = (path) => loadMedia({ path, role: 'picture' })

  async function startVoice(entry) {
    const url = await loadMedia(entry)
    if (!url) {
      const next = voices.ended()
      if (next) startVoice(next)
      return
    }
    player.pause()
    player.src = url
    player.currentTime = 0
    try {
      await player.play()
      playing.value = entry.path
    } catch {
      playing.value = null
    }
  }

  // the member's click: this voice plays now and the waiting line is cleared
  const playVoice = (entry) => startVoice(voices.choose(entry))

  function toggleVoice(entry) {
    if (playing.value === entry.path) {
      player.pause()
      voices.stop()
      playing.value = null
    } else {
      playVoice(entry)
    }
  }

  // A voice that arrived live plays now, or waits its turn behind the one playing.
  function arrive(entry) {
    const now = voices.arrive(entry, isPlaying())
    if (now) startVoice(now)
  }

  function stop() {
    voices.stop()
    player.pause()
  }

  return { media, durations, playing, loadMedia, loadPath, playVoice, toggleVoice, arrive, stop }
}
