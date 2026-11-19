import { describe, expect, it, vi } from 'vitest'
import { changed, cropOf, drawOp, finish, keptArea, pixelate, pushOp, redo, startHistory, toolForKey, undo } from './markup.js'
import { applyMarked, keepMarks, markupSource, replacePicturePath, saveMarkedCopy } from './markupsave.js'

const arrow = { t: 'arrow', c: '#ff5a5a', a: { x: 0, y: 0 }, b: { x: 50, y: 50 } }
const box = { t: 'box', c: '#ffd24a', r: { x: 10, y: 10, w: 40, h: 30 } }

describe('the marks and their undo', () => {
  it('undoes and redoes in order, and a new mark ends what could be redone', () => {
    let h = startHistory()
    h = pushOp(pushOp(h, arrow), box)
    expect(h.ops).toEqual([arrow, box])
    h = undo(h)
    expect(h.ops).toEqual([arrow])
    expect(h.undone).toEqual([box])
    h = redo(h)
    expect(h.ops).toEqual([arrow, box])
    h = pushOp(undo(h), { ...arrow, c: '#fff' })
    expect(h.undone).toEqual([])
    expect(redo(h)).toBe(h)
    expect(undo(startHistory())).toEqual(startHistory())
  })

  it('knows when the marks differ from the ones it opened with', () => {
    const opened = startHistory([arrow])
    expect(changed(opened)).toBe(false)
    expect(changed(pushOp(opened, box))).toBe(true)
    expect(changed(undo(opened))).toBe(true)
    // marking and undoing it again is no change
    expect(changed(undo(pushOp(opened, box)))).toBe(false)
  })

  it('a crop is a mark too: undo brings the whole picture back', () => {
    const crop = { t: 'crop', r: { x: 5, y: 5, w: 20, h: 20 } }
    const h = pushOp(pushOp(startHistory(), crop), box)
    expect(cropOf(h.ops)).toEqual(crop.r)
    expect(keptArea(h.ops, 100, 80)).toEqual(crop.r)
    const back = undo(undo(h))
    expect(cropOf(back.ops)).toBeNull()
    expect(keptArea(back.ops, 100, 80)).toEqual({ x: 0, y: 0, w: 100, h: 80 })
  })

  it('a click is not a mark, and a drag is kept inside the picture', () => {
    expect(finish({ t: 'box', a: { x: 10, y: 10 }, b: { x: 11, y: 12 } }, 400, 300)).toBeNull()
    expect(finish({ t: 'pen', pts: [{ x: 1, y: 1 }] }, 400, 300)).toBeNull()
    expect(finish({ t: 'crop', a: { x: 350, y: 250 }, b: { x: 500, y: -20 } }, 400, 300)).toEqual({ t: 'crop', r: { x: 350, y: 0, w: 50, h: 250 } })
    expect(finish({ t: 'blur', c: 'x', a: { x: 90, y: 90 }, b: { x: 10, y: 20 } }, 400, 300).r).toEqual({ x: 10, y: 20, w: 80, h: 70 })
  })

  it('the tool keys choose tools', () => {
    expect(['A', 'b', 'p', 't', 'U', 'c', 'x'].map(toolForKey)).toEqual(['arrow', 'box', 'pen', 'text', 'blur', 'crop', null])
  })
})

describe('blur really hides what is under it', () => {
  // a 8x4 picture of random noise: stands in for small writing
  const noise = () => Uint8ClampedArray.from({ length: 8 * 4 * 4 }, (_, i) => (i * 97 + 13) % 251)

  it('every block becomes one colour, and the pixels outside are left alone', () => {
    const before = noise()
    const data = pixelate(noise(), 8, 4, { x: 0, y: 0, w: 4, h: 4 }, 2)
    const px = (x, y) => Array.from(data.slice((y * 8 + x) * 4, (y * 8 + x) * 4 + 4))
    expect(px(0, 0)).toEqual(px(1, 1))
    expect(px(2, 0)).toEqual(px(3, 1))
    expect(px(0, 0)).not.toEqual(Array.from(before.slice(0, 4)))
    expect(Array.from(data.slice(4 * 4, 8 * 4))).toEqual(Array.from(before.slice(4 * 4, 8 * 4)))
  })

  it('the painted blur writes the pixelated pixels into the picture itself', () => {
    const size = 160
    const pixels = Uint8ClampedArray.from({ length: size * size * 4 }, (_, i) => (i * 31) % 256)
    const g = {
      save() {},
      restore() {},
      getImageData: vi.fn((x, y, w, h) => {
        const data = new Uint8ClampedArray(w * h * 4)
        for (let row = 0; row < h; row++) data.set(pixels.slice(((y + row) * size + x) * 4, ((y + row) * size + x + w) * 4), row * w * 4)
        return { data, width: w, height: h }
      }),
      putImageData: vi.fn(),
    }
    drawOp(g, { t: 'blur', r: { x: 20, y: 20, w: 40, h: 40 } }, size, size)
    const [image, x, y, dx, dy, dw, dh] = g.putImageData.mock.calls[0]
    expect([x + dx, y + dy, dw, dh]).toEqual([20, 20, 40, 40])
    // blocks of 6 px at this size, on a grid from the picture's corner (18-23, 24-29, ...):
    // inside one block every pixel is the same, and none is the picture's own any more
    const at = (px, py) => Array.from(image.data.slice(((py - y) * image.width + (px - x)) * 4, ((py - y) * image.width + (px - x)) * 4 + 4))
    const own = (px, py) => Array.from(pixels.slice((py * size + px) * 4, (py * size + px) * 4 + 4))
    expect(at(24, 24)).toEqual(at(29, 29))
    expect(at(20, 20)).toEqual(at(23, 23))
    expect(at(24, 24)).not.toEqual(at(23, 23))
    expect(at(25, 26)).not.toEqual(own(25, 26))
  })
})

describe('the marked copy takes the original’s place in the message', () => {
  it('names the copy wherever the message names the picture, quoted or not', () => {
    expect(replacePicturePath('look C:\\a\\shot.png here', 'C:\\a\\shot.png', 'C:\\a\\shot-marked.png')).toBe('look C:\\a\\shot-marked.png here')
    expect(replacePicturePath('see "/Users/me/my shot.png".', '/Users/me/my shot.png', '/Users/me/my shot-marked.png')).toBe('see "/Users/me/my shot-marked.png".')
    expect(replacePicturePath('/a/shot.png', '/a/shot.png', '/tmp/x y.png')).toBe('"/tmp/x y.png"')
    // a longer path that only ends the same is not the picture
    expect(replacePicturePath('/b/a/shot.png and /a/shot.png', '/a/shot.png', '/a/m.png')).toBe('/b/a/shot.png and /a/m.png')
  })

  it('remembers the marks by the copy, opens a copy on its original, and goes back when marks are cleared', () => {
    let state = { draft: 'fix /p/shot.png please', marks: {} }
    expect(markupSource(state.marks, '/p/shot.png')).toEqual({ original: '/p/shot.png', ops: [], previous: null })

    state = applyMarked(state, { shown: '/p/shot.png', original: '/p/shot.png', ops: [arrow], saved: '/p/shot-marked.png' })
    expect(state.draft).toBe('fix /p/shot-marked.png please')
    expect(state.marks).toEqual({ '/p/shot-marked.png': { original: '/p/shot.png', ops: [arrow] } })
    expect(markupSource(state.marks, '/p/shot-marked.png')).toEqual({ original: '/p/shot.png', ops: [arrow], previous: '/p/shot-marked.png' })

    // every mark taken away: the original goes back in and nothing is remembered
    state = applyMarked(state, { shown: '/p/shot-marked.png', original: '/p/shot.png', ops: [], saved: '/p/shot.png' })
    expect(state).toEqual({ draft: 'fix /p/shot.png please', marks: {} })
  })

  it('forgets the marks of a picture once it is sent or removed', () => {
    const marks = { '/a-marked.png': { original: '/a.png', ops: [arrow] }, '/b-marked.png': { original: '/b.png', ops: [box] } }
    expect(keepMarks(marks, ['/b-marked.png'])).toEqual({ '/b-marked.png': marks['/b-marked.png'] })
    expect(keepMarks(marks, [])).toEqual({})
  })

  it('saves through the core with the original and the copy shown now, and saves nothing without marks', async () => {
    const save = vi.fn(async () => '/p/shot-marked.png')
    const picture = 'data:image/png;base64,iVBORw0KGgo='
    expect(await saveMarkedCopy({ original: '/p/shot.png', previous: null, ops: [arrow], picture, save })).toBe('/p/shot-marked.png')
    expect(save).toHaveBeenCalledWith('/p/shot.png', picture, null)
    save.mockClear()
    expect(await saveMarkedCopy({ original: '/p/shot.png', previous: '/p/shot-marked.png', ops: [], picture: null, save })).toBe('/p/shot.png')
    expect(save).not.toHaveBeenCalled()
    await expect(saveMarkedCopy({ original: '/p/shot.png', previous: null, ops: [arrow], picture: 'data:image/jpeg;base64,x', save })).rejects.toThrow()
  })
})
