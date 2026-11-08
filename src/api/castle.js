// The only file that talks to Castle's Rust core.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export const hideCastle = () => invoke('hide_castle')

// Settings. -> { settings, companionLabel, cliLabel, warning, startupNote }
export const getSettings = () => invoke('get_settings')
export const saveSettings = (settings) => invoke('save_settings', { settings })
export const chooseFolder = (title) => invoke('choose_folder', { title })

// First-run setup. -> FolderState { exists, isMemorycore, needsNames, entryFile, companionName }
export const setupInspect = (folder) => invoke('setup_inspect', { folder })
export const setupCreate = (folder) => invoke('setup_create', { folder })
// -> { cli, label, installed, version, signedIn, installCommand, signInHint }
export const setupCheckCli = (cli) => invoke('setup_check_cli', { cli })
export const setupApply = (folder, companion, member, style) => invoke('setup_apply', { folder, companion, member, style })
// Keys the member already keeps in the memory folder's .env (masked), and keeping them in Castle.
export const setupFoundKeys = (folder) => invoke('setup_found_keys', { folder })
export const setupImportKeys = (folder, names) => invoke('setup_import_keys', { folder, names })
export const choosePicture = (title) => invoke('choose_picture', { title })
// The companion's picture (a data URL), or null for the default crystal.
export const avatarSet = (path) => invoke('avatar_set', { path })
export const avatarGet = () => invoke('avatar_get')
export const avatarClear = () => invoke('avatar_clear')

// Keys, kept encrypted by Windows. -> [{ name, label, set, masked }]
export const keyStatus = () => invoke('key_status')
export const keySave = (name, value) => invoke('key_save', { name, value })
export const keyForget = (name) => invoke('key_forget', { name })
// The whole key, only when the member presses Show.
export const keyReveal = (name) => invoke('key_reveal', { name })

// The companion's sessions. room: undefined for the main session, 'story' for the STORY room's.
// -> { running, status: { ready, message, fix }, snapshot }
export const terminalState = (room) => invoke('terminal_state', { room })
export const terminalStart = (cols, rows, room) => invoke('terminal_start', { cols, rows, room })
export const terminalWrite = (data, room) => invoke('terminal_write', { data, room })
export const terminalResize = (cols, rows, room) => invoke('terminal_resize', { cols, rows, room })
export const storyClose = () => invoke('story_close')
// True once, when Windows started Castle and the start-session switch is on.
export const takeAutoWake = () => invoke('take_auto_wake')

// Chat: new entries from the session's own record. -> { entries: [{ id, role, text, at, path, pictures, state }], reset, status }
export const chatPoll = (room) => invoke('chat_poll', { room })
// A picture or voice Chat showed this session, as a data URL.
export const chatMedia = (path, room) => invoke('chat_media', { path, room })
// Pictures the member dropped or pasted; returns the ones Chat may show.
export const chatAttach = (paths) => invoke('chat_attach', { paths })

// Which MemoryCore features the member's folder holds. -> { diary, projects, library, saveMemory, ... }
export const memoryFeatures = () => invoke('memory_features')

// The desktop.
export const systemMeters = () => invoke('system_meters')
export const appsList = () => invoke('apps_list')
export const appsDrawIcons = () => invoke('apps_draw_icons')
export const appOpen = (path) => invoke('app_open', { path })
export const clipboardPaste = () => invoke('clipboard_paste')
export const clipboardCopy = (text) => invoke('clipboard_copy', { text })
export const userThemes = () => invoke('user_themes')
// Opens a file from the memory folder in its own app, or its folder with the file selected.
export const chatMediaOpen = (path, folder = false) => invoke('media_open', { path, folder })

// Events from the core.
const on = (name) => (handler) => listen(name, (event) => handler(event.payload))
export const onTerminalOutput = on('terminal-output')
export const onTerminalExit = on('terminal-exit')
export const onStoryOutput = on('story-output')
export const onStoryExit = on('story-exit')
// The tray's Wake companion.
export const onWakeRequest = (handler) => listen('castle-wake', () => handler())
