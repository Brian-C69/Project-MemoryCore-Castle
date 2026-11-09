import { describe, expect, it } from 'vitest'
import { castFor, findScene, inWorld, initialOf, readBeatCard, readStageText, toPages } from './stage.js'

describe('the story stage', () => {
  it('reads a beat card defensively', () => {
    const card = readBeatCard({ beat: 7, chapter: 'Chapter 2', scene: 'scenes/03-bridge.png', lines: [{ speaker: '', text: 'Mist.' }, { speaker: 'Nova', text: '**Stay** close.', voice: 'stage/voice/0007-1.mp3' }, { speaker: 'X', text: '' }], choices: ['Cross', '', 'Wait'] })
    expect(card.lines).toEqual([
      { speaker: null, text: 'Mist.', voice: null },
      { speaker: 'Nova', text: 'Stay close.', voice: 'stage/voice/0007-1.mp3' },
    ])
    expect(card.choices).toEqual(['Cross', 'Wait'])
    expect(readBeatCard(null)).toBeNull()
  })

  it('reads the plain reply the same way', () => {
    const beat = readStageText('## Chapter 2\n[scene: the moon bridge]\nMist slides over\nthe old bridge.\n\nNova: "Stay close."\nGuard: "Halt!"\n\n1. Cross now\n2. Find another way\n3. (Say something)\n')
    expect(beat.chapter).toBe('Chapter 2')
    expect(beat.scene).toBe('the moon bridge')
    expect(beat.lines).toEqual([
      { speaker: null, text: 'Mist slides over the old bridge.', voice: null },
      { speaker: 'Nova', text: 'Stay close.', voice: null },
      { speaker: 'Guard', text: 'Halt!', voice: null },
    ])
    expect(beat.choices).toEqual(['Cross now', 'Find another way', '(Say something)'])
  })

  it('keeps a numbered list in the middle as narration, not choices', () => {
    const beat = readStageText('1. first clue\nThen the door opened.\n\n1. Go in\n2. Wait')
    expect(beat.choices).toEqual(['Go in', 'Wait'])
  })

  it('splits long narration into pages at sentence ends', () => {
    const long = 'One sentence here. '.repeat(30).trim()
    const pages = toPages([{ speaker: null, text: long, voice: null }], 100)
    expect(pages.length).toBeGreaterThan(3)
    expect(pages.every((p) => p.text.length <= 100)).toBe(true)
  })

  it('finds a scene by path or name, never outside the world', () => {
    const scenes = ['D:\\M\\adventures\\sky\\scenes\\01-gate.png', 'D:\\M\\adventures\\sky\\scenes\\03-moon-bridge.png']
    expect(findScene('scenes/03-moon-bridge.png', scenes)).toBe(scenes[1])
    expect(findScene('the moon bridge', scenes)).toBeNull()
    expect(findScene('moon bridge', scenes)).toBe(scenes[1])
    expect(inWorld('D:\\M\\adventures\\sky', 'stage/voice/1.mp3')).toBe('D:\\M\\adventures\\sky\\stage\\voice\\1.mp3')
    expect(inWorld('D:\\M\\adventures\\sky', '../../secret.png')).toBeNull()
  })

  it('gives each speaker a face: cast by name, the companion portrait, else none', () => {
    const cast = [{ name: 'old mira', path: 'mira.png' }, { name: 'hero', path: 'hero.png' }]
    expect(castFor('Old Mira', cast)).toBe('mira.png')
    expect(castFor('Nova', cast, 'Nova', 'portrait.png')).toBe('portrait.png')
    expect(castFor('Guard', cast, 'Nova', 'portrait.png')).toBeNull()
    expect(castFor(null, cast)).toBeNull()
    expect(initialOf('guard')).toBe('G')
    expect(initialOf(null)).toBe('✦')
  })
})
