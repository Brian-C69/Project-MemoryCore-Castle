import { describe, expect, it } from 'vitest'
import { DEFAULT_WORLD, allWorlds, findWorld, fromManifest, fromMine, isWorldId } from './registry.js'
import { pieceThere, readiness, statusLine } from './readiness.js'

const answer = (ok, type = 'model/gltf-binary') => async () => ({ ok, headers: { get: () => type } })

describe('world ids', () => {
  it('accepts the three kinds and refuses paths', () => {
    expect(isWorldId('3d/crystal-isle')).toBe(true)
    expect(isWorldId('2d/mine')).toBe(true)
    expect(isWorldId('my/star-garden')).toBe(true)
    expect(isWorldId('3d/../secrets')).toBe(false)
    expect(isWorldId('my/a\\b')).toBe(false)
    expect(isWorldId('4d/x')).toBe(false)
    expect(isWorldId('')).toBe(false)
  })
})

describe('the registry', () => {
  const list = allWorlds()

  it('holds the model worlds, the built-in isles, the slideshows and My pictures', () => {
    const ids = list.map((w) => w.id)
    expect(ids).toContain('3d/crystal-isle')
    expect(ids).toContain('3d/memory-isles')
    expect(ids).toContain(DEFAULT_WORLD)
    expect(ids).toContain('2d/isles-of-memory')
    expect(ids).toContain('2d/mine')
    expect(new Set(ids).size).toBe(ids.length)
    expect(list.every((w) => isWorldId(w.id))).toBe(true)
  })

  it('serves model pieces from public/worlds/<slug>/', () => {
    const crystal = findWorld(list, '3d/crystal-isle')
    expect(crystal.engine).toBe('glb')
    expect(crystal.pieces).toHaveLength(5)
    expect(crystal.pieces[0].url).toBe('/worlds/crystal-isle/01-crystal-isle-base.glb')
    expect(crystal.pieces.filter((p) => p.role === 'companion')).toHaveLength(1)
    expect(crystal.camera.default).toBe('orbit')
  })

  it('makes a member world from its folder, loading pieces from pieces/', () => {
    const item = {
      slug: 'star-garden',
      folder: 'D:\\Memory\\worlds\\star-garden',
      manifest: { title: 'Star Garden', layout: 'centrepiece', pieces: [{ file: '01-base-isle.glb', role: 'base', size: 40 }] },
      have: 1,
      exports: 1,
      preview: 'D:\\Memory\\worlds\\star-garden\\design\\00-concept.png',
      painted: true,
    }
    const w = fromMine(item, (p) => `asset://${p}`)
    expect(w.id).toBe('my/star-garden')
    expect(w.pieces[0].url).toBe('asset://D:\\Memory\\worlds\\star-garden\\pieces\\01-base-isle.glb')
    expect(w.preview).toBe('asset://D:\\Memory\\worlds\\star-garden\\design\\00-concept.png')
    expect(w.source).toBe('yours')
  })

  it('adds the member worlds and drops pieces with an unknown role', () => {
    const mine = fromManifest(
      { slug: 'star-garden', title: 'Star Garden', pieces: [{ file: '01-a.glb', role: 'base' }, { file: 'x.glb', role: 'spaceship' }] },
      { idPrefix: 'my', base: 'asset://w/', source: 'yours' },
    )
    expect(mine.id).toBe('my/star-garden')
    expect(mine.pieces).toHaveLength(1)
    expect(findWorld(allWorlds([mine]), 'my/star-garden')).not.toBeNull()
  })
})

describe('readiness', () => {
  const crystal = findWorld(allWorlds(), '3d/crystal-isle')

  it('counts a piece only when it answers OK and is not an HTML page', async () => {
    expect(await pieceThere('/a.glb', answer(true))).toBe(true)
    expect(await pieceThere('/a.glb', answer(true, 'text/html; charset=utf-8'))).toBe(false)
    expect(await pieceThere('/a.glb', answer(false))).toBe(false)
    expect(await pieceThere('/a.glb', async () => { throw new Error('offline') })).toBe(false)
  })

  it('waits until every piece is there', async () => {
    expect(await readiness(crystal, answer(false))).toEqual({ have: 0, total: 5, ready: false })
    expect(await readiness(crystal, answer(true))).toEqual({ have: 5, total: 5, ready: true })
    let n = 0
    const some = async () => ({ ok: n++ < 2, headers: { get: () => '' } })
    expect(await readiness(crystal, some)).toEqual({ have: 2, total: 5, ready: false })
  })

  it('takes a member world count from the folder and calls built-in worlds ready', async () => {
    expect(await readiness({ engine: 'glb', have: 3, pieces: [1, 2, 3] })).toEqual({ have: 3, total: 3, ready: true })
    expect((await readiness(findWorld(allWorlds(), DEFAULT_WORLD))).ready).toBe(true)
  })

  it('says the status in plain words', () => {
    expect(statusLine(crystal, { have: 0, total: 5, ready: false })).toBe('waiting · 0/5 · make in Tripo')
    expect(statusLine(crystal, { have: 5, total: 5, ready: true })).toBe('ready')
    expect(statusLine(findWorld(allWorlds(), '2d/isles-of-memory'))).toBe('ready · 6 pictures')
  })
})
