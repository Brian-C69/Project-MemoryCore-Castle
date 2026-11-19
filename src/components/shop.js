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

// ---------- My shop ----------

/** A fresh shop form: the member's own words, or what the Portfolio holds. */
export const shopForm = (mine, companion = 'Your companion') => ({
  name: mine?.name ?? '',
  keeperName: mine?.keeperName ?? companion,
  greeting: mine?.greeting ?? '',
  keeperCard: mine?.keeperCard ?? '',
  open: mine?.open ?? true,
})

/** The first thing wrong with a shop before it is sent (the core and the Portfolio check again). */
export function shopFormProblem(f, keeperPicture = '', backdrop = '') {
  const count = (s) => (s ?? '').trim().length
  if (!count(f.name) || count(f.name) > 60) return 'Give the shop a name (at most 60 letters).'
  if (!count(f.keeperName) || count(f.keeperName) > 40) return 'Give the keeper a name (at most 40 letters).'
  if (count(f.greeting) > 160) return 'Keep the greeting under 160 letters.'
  if (count(f.keeperCard) > 1200) return 'Keep the keeper card under 1200 letters.'
  for (const file of [keeperPicture, backdrop]) {
    if (file && !/\.(png|jpe?g|webp)$/i.test(file)) return 'Pictures must be PNG, JPEG or WebP.'
  }
  return ''
}

export const CARD_WORDS = { pending: 'Waiting for review', approved: 'Approved', rejected: 'Not accepted' }
