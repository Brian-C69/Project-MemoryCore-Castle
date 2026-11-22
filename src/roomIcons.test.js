import { describe, expect, it } from 'vitest'
import { GROUPS } from './launcher.js'
import { roomIcon } from './roomIcons.js'

describe('room icon family', () => {
  it('covers every launcher room and the launcher with theme-inheriting local art', () => {
    for (const id of [...GROUPS.flatMap(g => g.rooms), 'launcher']) {
      const svg = roomIcon(id)
      expect(svg, id).toContain('viewBox="0 0 64 64"')
      expect(svg, id).toContain('stroke="currentColor"')
      expect(svg, id).not.toMatch(/#[0-9a-f]{3,8}|<script|\son\w+=|href=/i)
    }
    expect(roomIcon('future-room')).toBe('')
  })
})
