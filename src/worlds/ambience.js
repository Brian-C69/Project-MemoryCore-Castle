// Castle's own music, played before the songs the companion made: Castle Theme for the 2D worlds,
// Floating Isles for the 3D ones. Replace the files in public/ambience/ (keep the names) or change
// this list to use other music.
export const AMBIENCE = {
  '2d': [
    { title: 'Castle Theme', url: '/ambience/castle-theme.mp3' },
    { title: 'Castle Theme (second take)', url: '/ambience/castle-theme-v2.mp3' },
  ],
  '3d': [
    { title: 'Floating Isles', url: '/ambience/floating-isles.mp3' },
    { title: 'Floating Isles (second take)', url: '/ambience/floating-isles-v2.mp3' },
  ],
}

export const ambienceFor = (kind) => AMBIENCE[kind === '2d' ? '2d' : '3d']

// "03-morning-light.mp3" -> "morning light"
export const songTitle = (name) => String(name).replace(/\.[^.]+$/, '').replace(/^\d+-/, '').replace(/-/g, ' ')
