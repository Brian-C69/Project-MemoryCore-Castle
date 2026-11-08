# MemoryCore Castle

A full-screen Windows desktop for your **MemoryCore** companion. Castle opens your own MemoryCore folder, runs **Claude Code** or **Codex** inside it, and shows your companion's Chat and Log beside rooms for everything your memory holds.

> Version 0.1.0 - early build, under active development.

## What it gives you

- **HOME** - clock, CPU and RAM, today's reminders, your projects, the newest diary entry, and how your memory has grown
- **Chat and Log** - talk to your companion in a chat view, or switch to the real terminal underneath
- **Rooms** - Apps, Diary, Projects, Library, Appearance, Studio, Story, World and Chat; a room appears when its MemoryCore feature is installed
- **Your keys, encrypted** - ChatGPT (OpenAI), Seedance, Suno and ElevenLabs keys are entered in Settings and kept by Windows Credential Manager, never in your memory folder. Without a key, Castle saves the prompt or lyrics instead and tells you what to add
- **Your login** - Claude Code or Codex signs in with your own account; Castle has no account of its own

## Requirements

- Windows 10 or 11
- [Claude Code](https://docs.claude.com/en/docs/claude-code) or [Codex CLI](https://github.com/openai/codex), installed and signed in
- A MemoryCore folder ([AI MemoryCore](https://github.com/Kiyoraka/Poject-AI-Memory-Architecture)) - Castle can create one for you on first run

To build from source you also need:

- [Node.js](https://nodejs.org/) 20 or newer
- [Rust](https://www.rust-lang.org/tools/install) (stable) with the MSVC toolchain
- Microsoft Edge WebView2 (already on Windows 11)

## Build and run

```bash
npm install
npm run dev          # Castle in development, with live reload
npm run tauri build  # the installer, in src-tauri/target/release/bundle/nsis/
```

## Make it yours

Castle is MIT licensed - fork it and change anything: the theme, the rooms, the name. A guide (`MAKE-IT-YOURS.md`) arrives with the first release.

## License

[MIT](LICENSE)
