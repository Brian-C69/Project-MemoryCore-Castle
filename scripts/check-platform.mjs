// Castle runs on Windows and macOS. This check fails when code reaches for something only Windows
// has (Explorer, cmd.exe, PowerShell, %APPDATA%, a path joined with `\`) without saying it is the
// Windows side: a `cfg(windows)` / `cfg!(windows)` nearby in Rust, `isMac` / `absPath` in the page,
// or a `platform: windows-only` comment on the line or the one above when the use is deliberate.
// Run: node scripts/check-platform.mjs (CI runs it on every pull request).
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'

const ROOT = new URL('..', import.meta.url).pathname
const LOOK_BACK = 14 // lines above a use that may hold its Windows guard

const RUST = [
  [/(Command|CommandBuilder)::new\("(explorer|rundll32|cmd|powershell|taskkill|where)(\.exe)?"/i, 'a Windows program started'],
  [/var(_os)?\("(APPDATA|LOCALAPPDATA|USERPROFILE|ProgramData|SystemRoot|PATHEXT|COMSPEC)"\)/, 'a Windows-only variable'],
]
const RUST_GUARD = /cfg!?\((all\()?(not\()?windows|cfg!?\(target_os|platform: windows-only/

const PAGE = [
  [/\}\\\\\$\{/, 'a path joined with \\'],
  [/\+ *'\\\\' *\+/, 'a path joined with \\'],
  [/%[A-Z_]+%/, 'a cmd-style %VARIABLE%'],
  [/rundll32|explorer\.exe|powershell(\.exe)?\s+-/i, 'a Windows program run'],
]
const PAGE_GUARD = /isMac|onMac|absPath|platform: windows-only/

function files(dir, ext) {
  const out = []
  for (const name of readdirSync(dir)) {
    const path = join(dir, name)
    if (statSync(path).isDirectory()) out.push(...files(path, ext))
    else if (ext.some((e) => name.endsWith(e)) && !/\.test\.(js|ts)$/.test(name)) out.push(path)
  }
  return out
}

// Rust test modules come last in a file: everything after `#[cfg(test)]` is skipped.
function codeLines(path) {
  const lines = readFileSync(path, 'utf8').split('\n')
  const tests = lines.findIndex((l) => l.trim() === '#[cfg(test)]')
  return tests < 0 ? lines : lines.slice(0, tests)
}

const found = []
function scan(list, patterns, guard, isRust) {
  for (const path of list) {
    const lines = isRust ? codeLines(path) : readFileSync(path, 'utf8').split('\n')
    lines.forEach((line, i) => {
      const code = line.trim()
      if (code.startsWith('//') || code.startsWith('*') || code.startsWith('///')) return
      for (const [pattern, what] of patterns) {
        if (!pattern.test(line)) continue
        const near = lines.slice(Math.max(0, i - LOOK_BACK), i + 1).join('\n')
        if (!guard.test(near)) found.push(`${relative(ROOT, path)}:${i + 1}  ${what} with no Windows-only guard`)
      }
    })
  }
}

scan(files(join(ROOT, 'src-tauri/src'), ['.rs']), RUST, RUST_GUARD, true)
scan(files(join(ROOT, 'src'), ['.js', '.vue']), PAGE, PAGE_GUARD, false)

if (found.length) {
  console.error(`Windows-only code without a guard (${found.length}):\n  ${found.join('\n  ')}`)
  console.error('\nGive macOS its own branch, or mark a deliberate Windows-only use with "platform: windows-only".')
  process.exit(1)
}
console.log('Platform check passed: every Windows-only call has its guard.')
