// The markup sheet's marks: what the member drew on a picture (arrows, boxes, pen lines, words,
// blurred areas and a crop), undo and redo, and how the marks are painted onto a canvas. Kept apart
// from the sheet so it is tested without a page. Points are in the picture's own pixels.

export const TOOLS = [
  { id: 'arrow', label: 'Arrow', icon: '↗', key: 'a' },
  { id: 'box', label: 'Box', icon: '▢', key: 'b' },
  { id: 'pen', label: 'Pen', icon: '✎', key: 'p' },
  { id: 'text', label: 'Text', icon: 'T', key: 't' },
  { id: 'blur', label: 'Blur', icon: '▦', key: 'u' },
  { id: 'crop', label: 'Crop', icon: '⌗', key: 'c' },
]

export const COLORS = [
  { value: '#ff5a5a', name: 'Red' },
  { value: '#ffd24a', name: 'Yellow' },
  { value: '#8fb4ff', name: 'Blue' },
  { value: '#ffffff', name: 'White' },
]

/** The tool a plain key chooses (A, B, P, T, U, C), or null. */
export const toolForKey = (key) => TOOLS.find((t) => t.key === String(key).toLowerCase())?.id ?? null

// ---- history: the marks in order, and the ones undone (Redo brings them back) ----

/** A fresh history starting from `ops` (the marks a marked copy was saved with, or none). */
export const startHistory = (ops = []) => ({ ops: [...ops], undone: [], start: [...ops] })

/** A new mark: it goes on top, and whatever was undone can no longer be redone. */
export const pushOp = (h, op) => ({ ...h, ops: [...h.ops, op], undone: [] })

export const undo = (h) => (h.ops.length ? { ...h, ops: h.ops.slice(0, -1), undone: [...h.undone, h.ops[h.ops.length - 1]] } : h)

export const redo = (h) => (h.undone.length ? { ...h, ops: [...h.ops, h.undone[h.undone.length - 1]], undone: h.undone.slice(0, -1) } : h)

/** Whether the marks differ from the ones the sheet opened with (Cancel then asks first). */
export const changed = (h) => h.ops.length !== h.start.length || h.ops.some((op, i) => op !== h.start[i])

/** The crop in force: the newest crop mark, or null for the whole picture. */
export function cropOf(ops) {
  for (let i = ops.length - 1; i >= 0; i--) if (ops[i].t === 'crop') return ops[i].r
  return null
}

// ---- geometry ----

export const rectOf = (a, b) => ({ x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(b.x - a.x), h: Math.abs(b.y - a.y) })

/** `r` kept inside a width x height picture, in whole pixels. */
export function clampRect(r, width, height) {
  const x = Math.max(0, Math.min(width, Math.round(r.x)))
  const y = Math.max(0, Math.min(height, Math.round(r.y)))
  const right = Math.max(x, Math.min(width, Math.round(r.x + r.w)))
  const bottom = Math.max(y, Math.min(height, Math.round(r.y + r.h)))
  return { x, y, w: right - x, h: bottom - y }
}

/** Sizes that follow the picture, so a mark looks the same on a small or a 4K screenshot. */
export function sizes(width, height) {
  const long = Math.max(width, height)
  return {
    line: Math.max(2, Math.round(long / 240)),
    font: Math.max(14, Math.round(long / 45)),
    block: Math.max(6, Math.round(long / 80)),
    // a drag shorter than this was a click, not a mark
    least: Math.max(4, Math.round(long / 160)),
  }
}

/** A finished drag (`draft`) to keep as a mark, or null when it was too small to mean one. */
export function finish(draft, width, height) {
  if (!draft) return null
  const { least } = sizes(width, height)
  if (draft.t === 'pen') return draft.pts.length > 1 ? draft : null
  if (draft.t === 'arrow') return Math.hypot(draft.b.x - draft.a.x, draft.b.y - draft.a.y) >= least ? draft : null
  const r = clampRect(rectOf(draft.a, draft.b), width, height)
  if (r.w < least || r.h < least) return null
  if (draft.t === 'crop') return { t: 'crop', r }
  return { ...draft, r }
}

// ---- blur: real pixelation, so nothing underneath can be read back from the saved file ----

/**
 * Pixelates `rect` of RGBA `data` (width x height) in place: every block of `block` x `block`
 * pixels becomes that block's average colour. Returns `data`.
 */
export function pixelate(data, width, height, rect, block) {
  const r = clampRect(rect, width, height)
  for (let by = r.y; by < r.y + r.h; by += block) {
    for (let bx = r.x; bx < r.x + r.w; bx += block) {
      const bw = Math.min(block, r.x + r.w - bx)
      const bh = Math.min(block, r.y + r.h - by)
      const sum = [0, 0, 0, 0]
      for (let y = by; y < by + bh; y++) {
        for (let x = bx; x < bx + bw; x++) {
          const i = (y * width + x) * 4
          for (let c = 0; c < 4; c++) sum[c] += data[i + c]
        }
      }
      const n = bw * bh
      const avg = sum.map((s) => Math.round(s / n))
      for (let y = by; y < by + bh; y++) {
        for (let x = bx; x < bx + bw; x++) {
          const i = (y * width + x) * 4
          for (let c = 0; c < 4; c++) data[i + c] = avg[c]
        }
      }
    }
  }
  return data
}

// ---- painting ----

function arrow(g, a, b, line) {
  const angle = Math.atan2(b.y - a.y, b.x - a.x)
  const head = line * 4.5
  g.beginPath()
  g.moveTo(a.x, a.y)
  g.lineTo(b.x - Math.cos(angle) * head * 0.6, b.y - Math.sin(angle) * head * 0.6)
  g.stroke()
  g.beginPath()
  g.moveTo(b.x, b.y)
  g.lineTo(b.x - head * Math.cos(angle - 0.45), b.y - head * Math.sin(angle - 0.45))
  g.lineTo(b.x - head * Math.cos(angle + 0.45), b.y - head * Math.sin(angle + 0.45))
  g.closePath()
  g.fill()
}

/** Paints one mark on `g` (a 2D context the size of the picture). A crop paints nothing here. */
export function drawOp(g, op, width, height) {
  const { line, font, block } = sizes(width, height)
  g.save()
  g.strokeStyle = g.fillStyle = op.c ?? '#ff5a5a'
  g.lineWidth = line
  g.lineCap = g.lineJoin = 'round'
  if (op.t === 'arrow') arrow(g, op.a, op.b, line)
  else if (op.t === 'box') g.strokeRect(op.r.x, op.r.y, op.r.w, op.r.h)
  else if (op.t === 'pen') {
    g.beginPath()
    op.pts.forEach((p, i) => (i ? g.lineTo(p.x, p.y) : g.moveTo(p.x, p.y)))
    g.stroke()
  } else if (op.t === 'text') {
    g.font = `600 ${font}px Inter, "Segoe UI", -apple-system, sans-serif`
    g.textBaseline = 'top'
    g.lineWidth = Math.max(2, font / 6)
    g.strokeStyle = 'rgba(0, 0, 0, 0.7)'
    g.strokeText(op.s, op.p.x, op.p.y)
    g.fillText(op.s, op.p.x, op.p.y)
  } else if (op.t === 'blur') {
    const r = clampRect(op.r, width, height)
    if (r.w && r.h) {
      // the block grid starts at the picture's corner, so two blurs side by side share one grid
      const grid = { x: Math.floor(r.x / block) * block, y: Math.floor(r.y / block) * block }
      const area = clampRect({ x: grid.x, y: grid.y, w: r.x + r.w - grid.x, h: r.y + r.h - grid.y }, width, height)
      const image = g.getImageData(area.x, area.y, area.w, area.h)
      pixelate(image.data, area.w, area.h, { x: 0, y: 0, w: area.w, h: area.h }, block)
      g.putImageData(image, area.x, area.y, r.x - area.x, r.y - area.y, r.w, r.h)
    }
  }
  g.restore()
}

/** The picture with every mark, in the order they were made (a blur hides marks under it too). */
export function paint(g, picture, ops, width, height) {
  g.clearRect(0, 0, width, height)
  g.drawImage(picture, 0, 0, width, height)
  for (const op of ops) drawOp(g, op, width, height)
}

/** The area the saved copy keeps: the crop, or the whole picture. */
export const keptArea = (ops, width, height) => cropOf(ops) ?? { x: 0, y: 0, w: width, h: height }
