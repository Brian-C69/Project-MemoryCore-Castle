// New projects and first-time git for the DEV room, talking to Castle's core. Every error is a
// plain message string.
import { invoke } from '@tauri-apps/api/core'

// -> { installed, signedIn, login, owners: [] } (the GitHub CLI and who is signed in)
export const devGithub = () => invoke('dev_github')
// -> the projects folder, or null when none is chosen yet
export const devRoot = () => invoke('dev_root')
export const devSetRoot = (folder) => invoke('dev_set_root', { folder })
// -> the sub-folders of the projects folder (the kinds; may be empty)
export const devKinds = () => invoke('dev_kinds')
// Clones the link into <root>\<kind>\<name> ('' kind: <root>\<name>). -> the new folder
export const devClone = (link, kind, name) => invoke('dev_clone', { link, kind: kind || '', name })
// Makes a new repo on GitHub and clones it the same way. -> the new folder
export const devCreate = (owner, name, isPrivate, kind) => invoke('dev_create', { owner, name, private: isPrivate, kind: kind || '' })
// Makes a GitHub repo for the tab's folder, adds it as origin and pushes. -> the repo URL
export const devConnect = (slot, owner, name, isPrivate) => invoke('dev_connect', { slot, owner, name, private: isPrivate })
// git init in the tab's folder.
export const devGitInit = (slot) => invoke('dev_git_init', { slot })
