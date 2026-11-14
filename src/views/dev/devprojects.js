// DEV's live servers, pure and tested: the servers a tab owns, the words for a server's owner, the
// link a server opens, and the program's plain name.

/** The servers one tab owns (started inside it, or running in its folder), by port. */
export function serversOf(servers, slot) {
  return (servers ?? []).filter((s) => s.slot === slot).sort((a, b) => a.port - b.port)
}

/** The PROJECT column: the tab's name, '-' for a known service, '(not a DEV project)' otherwise. */
export function ownerWords(server, tabs) {
  if (server.slot !== null && server.slot !== undefined) {
    return (tabs ?? []).find((t) => t.slot === server.slot)?.name ?? `Tab ${server.slot + 1}`
  }
  return /^(mysqld|httpd|nginx|postgres|redis-server|mongod)(\.exe)?$/i.test(server.program ?? '') ? '-' : '(not a DEV project)'
}

/** The address a live server opens in the browser. */
export const linkOf = (server) => `http://localhost:${server.port}`

/** "node.exe" -> "node". */
export const programOf = (server) => String(server.program ?? '').replace(/\.exe$/i, '')
