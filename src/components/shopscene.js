// A visit to a shop in the Marketplace town (designs marketplace-shop-flow step 1 to 4), the pure
// parts: which step comes next, what the keeper says when no AI is asked, what an item asks for, and
// the public facts handed to the keeper (keeper.rs drops anything else again).

export const STEPS = ['greet', 'items', 'explain', 'question']

/** Where a visit starts: at the greeting, or at the explanation when an item was picked in the town. */
export const startStep = (item) => (item ? 'explain' : 'greet')

/** The shop a visit stands in. With no shops door yet, a shop is just its seller. */
export function shopOf(detail, item) {
  if (detail?.slug) {
    return {
      slug: detail.slug,
      name: detail.name ?? '',
      keeperName: detail.keeper?.name || detail.seller?.name || 'The keeper',
      keeperPicture: detail.keeper?.picture ?? '',
      backdrop: detail.backdrop ?? '',
      greeting: detail.greeting ?? '',
      keeperCard: detail.keeperCard ?? '',
      seller: detail.seller?.name ?? '',
      items: detail.items ?? [],
    }
  }
  const seller = item?.seller?.name ?? 'The seller'
  return { slug: '', name: `${seller}'s stall`, keeperName: seller, keeperPicture: '', backdrop: '', greeting: '', keeperCard: '', seller, items: item ? [item] : [] }
}

/** The greeting the owner wrote, or a plain welcome. No AI is asked for it. */
export const greetingOf = (shop) => shop.greeting?.trim() || `Welcome to ${shop.name}. Would you like to see what is on the counter?`

const ASKS = { picture: 'A picture', character: 'A character', text: 'Some words', chips: 'A choice' }
/** What an item asks for, from its blocks (the steps are how it works, not what it asks for). */
export function asksFor(blocks) {
  return (blocks ?? [])
    .filter((b) => ASKS[b.type])
    .map((b) => (b.label?.trim() ? b.label.trim() : ASKS[b.type]))
}

/** The shop's public facts for the keeper. */
export const shopFacts = (shop) => ({ name: shop.name, keeperName: shop.keeperName, keeperCard: shop.keeperCard, seller: shop.seller })

/** An item's public facts for the keeper: never its prompt, never a file. */
export function itemFacts(listing) {
  return {
    id: String(listing?.id ?? ''),
    name: listing?.name ?? '',
    kind: listing?.kind ?? 'workflow',
    description: listing?.description ?? '',
    priceMyr: Number.isFinite(Number(listing?.priceMyr)) ? Number(listing.priceMyr) : null,
    votesUp: Number(listing?.votes?.up ?? 0),
    votesDown: Number(listing?.votes?.down ?? 0),
    blocks: (listing?.blocks ?? []).map((b) => ({ type: String(b.type ?? ''), label: String(b.label ?? '') })),
    updatedAt: String(listing?.updatedAt ?? ''),
  }
}

/** The choices under the dialogue box at each step. */
export function choicesFor(step, item) {
  const buy = item ? { id: 'buy', label: `Buy for RM ${Number(item.priceMyr ?? 0).toFixed(2)}` } : null
  switch (step) {
    case 'greet':
      return [{ id: 'items', label: 'Show me your items' }, { id: 'question', label: 'Ask a question' }, { id: 'leave', label: 'Leave' }]
    case 'items':
      return [{ id: 'question', label: 'Ask a question' }, { id: 'leave', label: 'Leave' }]
    case 'explain':
      return [{ id: 'question', label: 'Ask a question' }, buy, { id: 'items', label: 'See other items' }].filter(Boolean)
    case 'question':
      return [buy, { id: 'items', label: 'See other items' }].filter(Boolean)
    default:
      return []
  }
}
