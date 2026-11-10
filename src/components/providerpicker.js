// The provider picker in the companion header: which provider is active, and how each one is described.

// The cli id ('claude' or 'codex') of the provider in use, matched by the label Castle shows for it.
// Empty until the provider list has loaded.
export function currentProvider(providers, cliLabel) {
  return providers.find((p) => p.label === cliLabel)?.cli ?? ''
}

// What the picker shows: the provider being switched to while a switch runs, otherwise the one in use.
export function shownProvider(providers, cliLabel, switchingTarget) {
  return switchingTarget || currentProvider(providers, cliLabel)
}

export function providerStatus(p) {
  if (!p.installed) return 'not installed'
  if (p.signedIn) return 'signed in'
  if (p.signedIn === false) return 'sign in needed'
  return 'status unknown'
}
