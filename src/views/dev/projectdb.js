// The DEV tab's Database panel: the pure side - which kind of statement
// the box holds (the core checks again), the UPDATE a grid cell makes, the rows/time chip and the
// saved queries.

/** The statement's first word, past comments: 'read', 'write', 'shape' or '' (the core decides for real). */
export function kindOf(sql) {
  const text = String(sql ?? '')
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/(--|#)[^\n]*/g, ' ')
    .trim()
  const word = (/^[A-Za-z]+/.exec(text)?.[0] ?? '').toUpperCase()
  if (!word) return ''
  if (['SELECT', 'SHOW', 'DESCRIBE', 'DESC', 'EXPLAIN'].includes(word)) return 'read'
  if (word === 'WITH') return /\b(INSERT|UPDATE|DELETE|REPLACE)\b/i.test(text) ? 'write' : 'read'
  if (['INSERT', 'UPDATE', 'DELETE', 'REPLACE'].includes(word)) return 'write'
  return 'shape'
}

/** A value as a quoted SQL literal (the core's own escaping). */
export function quote(value) {
  if (value === null || value === undefined) return 'NULL'
  const map = { '\\': '\\\\', "'": "\\'", '\n': '\\n', '\r': '\\r', '\0': '\\0', '\x1a': '\\Z' }
  return `'${String(value).replace(/[\\'\n\r\0\x1a]/g, (c) => map[c])}'`
}

export const ident = (name) => `\`${String(name).replace(/`/g, '``')}\``

/**
 * The UPDATE one edited cell makes, or null when the table has no primary key among the grid's
 * columns (the cell cannot be told apart safely). `columns` are the table's ({ name, key }), `grid`
 * the result's column names, `row` the row's values.
 */
export function updateFor(table, columns, grid, row, colIndex, value) {
  const keys = (columns ?? []).filter((c) => c.key).map((c) => c.name)
  if (!table || !keys.length || !keys.every((k) => grid.includes(k))) return null
  const where = keys.map((k) => `${ident(k)} = ${quote(row[grid.indexOf(k)])}`).join(' AND ')
  return `UPDATE ${ident(table)} SET ${ident(grid[colIndex])} = ${quote(value)} WHERE ${where} LIMIT 1;`
}

/** The look of a table opened from the list. */
export const tableLook = (table) => `SELECT * FROM ${ident(table)} LIMIT 100;`

/** "50 rows · 0.08 s", "first 500 rows · 1.20 s", "3 rows changed". */
export function chipOf(result) {
  if (!result) return ''
  if (!result.columns?.length) return `${result.affected ?? 0} ${result.affected === 1 ? 'row' : 'rows'} changed`
  const n = result.rows?.length ?? 0
  const rows = result.truncated ? `first ${n} rows` : `${n} ${n === 1 ? 'row' : 'rows'}`
  return `${rows} · ${((result.ms ?? 0) / 1000).toFixed(2)} s`
}

/** Saved queries with one added or replaced by name (newest first, at most 100). */
export function savedWith(list, name, sql) {
  const n = String(name ?? '').trim()
  const s = String(sql ?? '').trim()
  if (!n || !s) return list ?? []
  return [{ name: n, sql: s }, ...(list ?? []).filter((q) => q.name.toLowerCase() !== n.toLowerCase())].slice(0, 100)
}

export const savedWithout = (list, name) => (list ?? []).filter((q) => q.name !== name)

/** Seconds left of a held change's two minutes. */
export const secondsLeft = (since, now = Date.now(), hold = 120) => Math.max(0, hold - Math.floor((now - since) / 1000))

/** The table list narrowed by the member's words: any part of the name, any case, in order. */
export function filterTables(tables, words) {
  const w = String(words ?? '').trim().toLowerCase()
  return w ? (tables ?? []).filter((t) => t.toLowerCase().includes(w)) : tables ?? []
}

/** A table's row estimate as the list shows it: 86, 1,248, 12.4k, 3.1M; '' when unknown. */
export function rowsLabel(n) {
  if (n == null || !Number.isFinite(Number(n))) return ''
  const v = Number(n)
  if (v < 10_000) return v.toLocaleString('en-US')
  if (v < 1_000_000) return `${(v / 1000).toFixed(1).replace(/\.0$/, '')}k`
  return `${(v / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
}

/** A host as the connection bar shows it: a long cloud host by its last three labels (the full one
 * rides in the hover); an address or a short name as it is. */
export function shortHost(host) {
  const h = String(host ?? '')
  if (h.length <= 28 || /^[\d.:]+$/.test(h)) return h
  const labels = h.split('.')
  return labels.length > 3 ? labels.slice(-3).join('.') : h
}

/**
 * The setup form for one side, pre-filled from what Castle knows ({ host, port, database, user, from,
 * hasPassword, ca }). The password field always starts empty: an empty one keeps the password
 * already kept.
 */
export function formOf(view, side) {
  const fallback = side === 'production' ? { host: '', port: 25060, user: '' } : { host: '127.0.0.1', port: 3306, user: 'root' }
  return {
    host: view?.host || fallback.host,
    port: Number(view?.port) || fallback.port,
    database: view?.database ?? '',
    user: view?.user || fallback.user,
    password: '',
    caPath: '',
  }
}

/**
 * What Save sends: the fields trimmed, the port a number, the password only when one was typed
 * (undefined keeps the kept one), a chosen CA only for Production. Null with a reason when it cannot
 * be saved yet.
 */
export function formToSave(form, side, hasPassword) {
  const host = String(form?.host ?? '').trim()
  const user = String(form?.user ?? '').trim()
  const port = Number(form?.port)
  if (!host || /\s/.test(host)) return { problem: 'Fill in the host (a name or an address, no spaces).' }
  if (!user) return { problem: 'Fill in the user.' }
  if (!Number.isInteger(port) || port < 1 || port > 65535) return { problem: 'The port is a number such as 3306.' }
  if (side === 'production' && !hasPassword && !form.password) return { problem: 'Production needs its password.' }
  const out = { host, port, database: String(form.database ?? '').trim(), user }
  if (form.password) out.password = form.password
  if (side === 'production' && form.caPath) out.caPath = form.caPath
  return { form: out }
}

/** Where a side's login came from, in words for the connection line. */
export function fromWords(from) {
  return { saved: 'saved in Castle', '.env': "the project's .env", 'application.properties': "the project's application.properties" }[from] ?? 'not set yet'
}
