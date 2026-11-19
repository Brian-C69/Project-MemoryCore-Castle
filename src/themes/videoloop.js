// A theme's living wallpaper as a short looping video (theme.json "motionVideo"), kept pure so it is
// tested. It plays only while Moving wallpapers is on, HOME is showing, the window is visible and the
// system does not ask to reduce motion; otherwise the still wallpaper shows and nothing is loaded.

export function shouldPlay({ enabled = false, visible = false, awake = false, calm = false } = {}) {
  return Boolean(enabled && visible && awake && !calm)
}

// Whether the video file is asked for at all: only once motion is on and allowed, so a member who
// keeps it off (to avoid any lag) never loads it.
export function shouldLoad({ enabled = false, calm = false } = {}) {
  return Boolean(enabled && !calm)
}
