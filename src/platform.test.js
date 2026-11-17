import { describe, expect, it } from 'vitest'
import { detectMac, isMac, modKey, onMac, shortcutLabel } from './platform.js'

describe('platform', () => {
  it('is not a Mac without a page (Node, the tests)', () => {
    expect(detectMac(undefined)).toBe(false)
    expect(isMac).toBe(false)
  })

  it('reads the user agent and platform', () => {
    expect(detectMac({ userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', platform: 'MacIntel' })).toBe(true)
    expect(detectMac({ userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)', platform: 'Win32' })).toBe(false)
    expect(detectMac({ userAgentData: { platform: 'macOS' } })).toBe(true)
  })

  it('uses ⌘ on a Mac and Ctrl elsewhere', () => {
    expect(modKey({ metaKey: true, ctrlKey: false }, true)).toBe(true)
    expect(modKey({ metaKey: false, ctrlKey: true }, true)).toBe(false)
    expect(modKey({ metaKey: false, ctrlKey: true }, false)).toBe(true)
    expect(modKey({ metaKey: true, ctrlKey: false }, false)).toBe(false)
  })

  it('labels shortcuts the Mac way only on a Mac', () => {
    expect(shortcutLabel('Ctrl+Enter', true)).toBe('⌘↩')
    expect(shortcutLabel('Alt+Left', true)).toBe('⌥←')
    expect(shortcutLabel('Ctrl+Enter', false)).toBe('Ctrl+Enter')
  })

  it('picks the words for the computer', () => {
    expect(onMac('Finder', 'Explorer', true)).toBe('Finder')
    expect(onMac('Finder', 'Explorer', false)).toBe('Explorer')
  })
})
