// "What's new": the bundled CHANGELOG.md read by version, using the release tools' own reader so
// the app and the release notes always agree.
import { changelogSections, compareVersions, isVersion } from '../../scripts/release-tools.mjs'

// The sections the member has not seen yet: newer than the last version they saw, up to the one
// running, newest first. Nothing on the very first launch (no last version) or when nothing is new.
export function unseenSections(text, lastSeen, current) {
  if (!isVersion(lastSeen) || !isVersion(current) || compareVersions(current, lastSeen) <= 0) return []
  return changelogSections(text)
    .filter((s) => s.version && s.body && compareVersions(s.version, lastSeen) > 0 && compareVersions(s.version, current) <= 0)
    .map(({ version, date, body }) => ({ version, date, body }))
}

// One version's section (for Settings > About and the dev preview), or null.
export function sectionFor(text, version) {
  const s = changelogSections(text).find((x) => x.version === version && x.body)
  return s ? { version: s.version, date: s.date, body: s.body } : null
}
