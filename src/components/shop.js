// STUDIO's Marketplace, the pure parts: when the tab and Publish show, prices and votes in words,
// and which of a member's purchases still need installing.

const NO_SHOP = { live: false, selling: false, connected: false }
export const shopOf = (status) => ({ ...NO_SHOP, ...(status?.marketplace ?? {}) })

/** The Marketplace tab shows only when the Portfolio says the doors are live. */
export const shopVisible = (status) => !!(status?.signedIn && shopOf(status).live)

/**
 * Publish on a workflow card: { show, enabled, label }. Only the member's own working workflows,
 * and only while the Marketplace is live; until selling opens it says so.
 */
export function publishAction(status, entry) {
  if (!shopVisible(status) || entry?.source !== 'yours' || entry?.error) return { show: false, enabled: false, label: '' }
  const shop = shopOf(status)
  if (!shop.selling) return { show: true, enabled: false, label: 'Selling opens soon' }
  return { show: true, enabled: true, label: 'Publish' }
}

export const priceText = (myr) => (Number.isFinite(Number(myr)) ? `RM ${Number(myr).toFixed(2)}` : '')

export function votesText(votes) {
  const up = Number(votes?.up ?? 0)
  const down = Number(votes?.down ?? 0)
  if (!up && !down) return 'No votes yet'
  return `👍 ${up} · 👎 ${down}`
}

/** The listing ids already on the member's rail (bought workflows carry their listing). */
export const installedListings = (entries) => new Set((entries ?? []).filter((e) => e.source === 'bought' && e.listing).map((e) => e.listing))

/** A purchase's button: Install, or Installed once its workflow is on the rail. */
export const installState = (purchase, installed) => (installed.has(String(purchase.listingId)) ? 'installed' : 'install')

export const STATUS_WORDS = {
  pending: 'Waiting for review',
  approved: 'On sale',
  rejected: 'Not accepted',
  paused: 'Paused',
}
