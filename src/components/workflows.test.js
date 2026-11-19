import { describe, expect, it } from 'vitest'
import { ICONS, createAction, describeAction, emptyValues, fieldsOf, iconOf, missing, runValues, usedIds } from './workflows.js'

const twoCharacters = {
  name: 'Two characters',
  icon: 'people',
  blocks: [
    { id: 'a', type: 'character', label: 'Character A' },
    { id: 'b', type: 'character', label: 'Character B' },
    { id: 'scene', type: 'text', label: 'Scene' },
    { id: 'style', type: 'chips', label: 'Style', options: ['Anime', 'Painterly', 'Photo'] },
    { id: 'make', type: 'create', prompt: '{{a.name}} and {{ b.name }}: {{scene}}. Style: {{style}}.', references: ['a', 'b'] },
  ],
}
const fromAPicture = {
  name: 'From a picture',
  blocks: [
    { id: 'photo', type: 'picture', label: 'Upload a picture' },
    { id: 'read', type: 'describe', label: 'Write prompt from picture', from: 'photo', into: 'prompt' },
    { id: 'prompt', type: 'text', label: '' },
    { id: 'make', type: 'create', prompt: '{{prompt}}', references: [] },
  ],
}

describe('workflows', () => {
  it('draws every input and Write-prompt button, never the picture step', () => {
    expect(fieldsOf(fromAPicture).map((f) => f.type)).toEqual(['picture', 'describe', 'text'])
    expect(fieldsOf(fromAPicture)[2].label).toBe('prompt') // an empty label falls back to the id
    expect(fieldsOf({})).toEqual([])
  })

  it('starts each choice on its first option and everything else empty', () => {
    const v = emptyValues(twoCharacters)
    expect(Object.keys(v)).toEqual(['a', 'b', 'scene', 'style'])
    expect(v.style.text).toBe('Anime')
    expect(v.a).toEqual({ text: '', name: '', picture: '' })
  })

  it('reads the names the prompt uses', () => {
    expect(usedIds(twoCharacters)).toEqual(['a', 'b', 'scene', 'style'])
  })

  it('says the first thing still to fill', () => {
    const v = emptyValues(twoCharacters)
    expect(missing(twoCharacters, v)).toBe('Fill in Scene.')
    v.scene.text = 'a picnic'
    expect(missing(twoCharacters, v)).toBe('Add a picture to Character A.')
    v.a.picture = 'C:/m/a.png'
    v.b.picture = 'C:/m/b.png'
    expect(missing(twoCharacters, v)).toBe('')
    expect(missing({ blocks: [] }, {})).toBe('This workflow has no picture step.')
  })

  it('the picture button follows the way to draw, then what is missing', () => {
    const v = emptyValues(twoCharacters)
    expect(createAction(twoCharacters, v, 'ready', false).kind).toBe('checking')
    expect(createAction(twoCharacters, v, 'prompt')).toEqual({ kind: 'key', label: 'Add the OpenAI key' })
    expect(createAction(twoCharacters, v, 'builtin', true, true).kind).toBe('busy')
    expect(createAction(twoCharacters, v, 'ready')).toMatchObject({ kind: 'missing', why: 'Fill in Scene.' })
    Object.assign(v.scene, { text: 'tea' })
    Object.assign(v.a, { picture: 'a.png' })
    Object.assign(v.b, { picture: 'b.png' })
    expect(createAction(twoCharacters, v, 'builtin')).toEqual({ kind: 'ready', label: 'Create picture' })
  })

  it('Write prompt from picture waits for its picture', () => {
    const read = fromAPicture.blocks[1]
    const v = emptyValues(fromAPicture)
    expect(describeAction(read, v, 'prompt').kind).toBe('key')
    expect(describeAction(read, v, 'ready')).toMatchObject({ kind: 'missing', why: 'Upload the picture first.' })
    v.photo.picture = 'C:/m/photo.png'
    expect(describeAction(read, v, 'ready').kind).toBe('ready')
    expect(describeAction(read, v, 'ready', true).label).toBe('Reading the picture…')
  })

  it('a run sends trimmed words and leaves empty parts out', () => {
    expect(runValues({ a: { text: '', name: ' Mochi ', picture: 'a.png' }, scene: { text: '  tea  ', name: '', picture: '' } })).toEqual({
      a: { name: 'Mochi', picture: 'a.png' },
      scene: { text: 'tea' },
    })
  })

  it('every workflow has an icon, an unknown one the spark', () => {
    expect(iconOf(twoCharacters)).toBe(ICONS.people)
    expect(iconOf({ icon: 'dragon' })).toBe(ICONS.spark)
    expect(iconOf(null)).toBe(ICONS.spark)
  })
})
