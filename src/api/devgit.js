// The DEV tab's Source panel talking to Castle's core (src-tauri/src/dev_git.rs). Every call
// names the tab's slot; the core finds that tab's project folder itself.
import { invoke } from '@tauri-apps/api/core'

// -> { branch, upstream, ahead, behind, files: [{ path, code, from }] }
export const devGitStatus = (slot) => invoke('dev_git_status', { slot })
// One file's diff text: the working tree, or inside `commit` (a hash) when given.
export const devGitDiff = (slot, path, commit = null) => invoke('dev_git_diff', { slot, path, commit })
// -> 50 commits after `skip`: [{ hash, short, parents, author, date, subject, refs }]
export const devGitLog = (slot, skip = 0) => invoke('dev_git_log', { slot, skip })
// -> { commit, message, files: [{ path, added, removed }], summary }
export const devGitShow = (slot, hash) => invoke('dev_git_show', { slot, hash })
// Commits only the given paths with the message. -> git's own words
export const devGitCommit = (slot, paths, message) => invoke('dev_git_commit', { slot, paths, message })
// Pushes the current branch (never forced). -> git's own words
export const devGitPush = (slot) => invoke('dev_git_push', { slot })
// -> 'https://github.com/<owner>/<repo>/commit/' or null
export const devGitRemote = (slot) => invoke('dev_git_remote', { slot })
// Opens a GitHub page or a server on this PC in the browser (the core refuses anything else).
export const devOpenLink = (url) => invoke('dev_open_link', { url })
