// The pure parts of GlassSelect: its options in one shape, the label of a value, and where the
// keyboard goes next (disabled options are stepped over).

/** Options as `[{ value, label, disabled }]`, from objects or plain strings. */
export function normalise(options = []) {
  return options.map((o) => (o !== null && typeof o === 'object' ? { value: o.value, label: String(o.label ?? o.value), disabled: Boolean(o.disabled) } : { value: o, label: String(o), disabled: false }))
}

/** The label shown for `value`, or the placeholder when no option has it. */
export function labelOf(options, value, placeholder = '') {
  return options.find((o) => o.value === value)?.label ?? placeholder
}

/** The next enabled index from `from` in `step` (+1 / -1), wrapping; -1 when none is enabled. */
export function nextEnabled(options, from, step) {
  const n = options.length
  if (!n) return -1
  for (let i = 1; i <= n; i++) {
    const at = (((from + step * i) % n) + n) % n
    if (!options[at].disabled) return at
  }
  return -1
}

/** The first (or last) enabled index; -1 when none. */
export function edgeEnabled(options, last = false) {
  return last ? nextEnabled(options, 0, -1) : nextEnabled(options, -1, 1)
}
