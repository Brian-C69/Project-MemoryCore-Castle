import { describe, expect, it } from 'vitest'
import { currentProvider, providerStatus, shownProvider } from './providerpicker.js'

const providers = [
  { cli: 'claude', label: 'Claude Code', installed: true, signedIn: true },
  { cli: 'codex', label: 'Codex', installed: true, signedIn: true },
]

describe('provider picker', () => {
  it('selects the provider in use instead of a placeholder', () => {
    expect(currentProvider(providers, 'Claude Code')).toBe('claude')
    expect(currentProvider(providers, 'Codex')).toBe('codex')
  })

  it('has no selection until the provider list loads', () => {
    expect(currentProvider([], 'Claude Code')).toBe('')
  })

  it('shows the target while a switch is running', () => {
    expect(shownProvider(providers, 'Claude Code', 'codex')).toBe('codex')
    expect(shownProvider(providers, 'Claude Code', undefined)).toBe('claude')
  })

  it('describes each provider status', () => {
    expect(providerStatus({ installed: false })).toBe('not installed')
    expect(providerStatus({ installed: true, signedIn: true })).toBe('signed in')
    expect(providerStatus({ installed: true, signedIn: false })).toBe('sign in needed')
    expect(providerStatus({ installed: true, signedIn: null })).toBe('status unknown')
  })
})
