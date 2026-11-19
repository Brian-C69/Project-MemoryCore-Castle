// castle-common.mjs - shared pieces for the Castle helper scripts (Node 20+, no packages).
//
// Imported by castle-image, castle-look, castle-video, castle-song, castle-voice, castle-world
// and castle-beat. One copy runs on Windows and macOS.
//
// Castle hands keys to the companion's CLI only as environment variables.
// Nothing here ever prints a key. Files written are UTF-8 without a BOM.
//
// Exit codes used by the helpers:
//   0  done (or --what-if-cost estimate printed)
//   1  the API call failed (one-line reason printed, secrets removed)
//   2  bad input (for example a missing reference file, or an unknown argument)
//   3  no key - a fallback file was saved instead and a notice was left
//   4  the CLI should use its own tool instead (Codex drawing pictures)
//   9  a network call was blocked because CASTLE_NO_NETWORK=1 (tests only)

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const KEY_NAMES = ['OPENAI_API_KEY', 'ARK_API_KEY', 'SUNOAPI_ORG_KEY', 'ELEVENLABS_API_KEY'];

// ---------- leaving: one line out, then the exit code ----------

/** Thrown to end a helper with an exit code; runMain() flushes stdout and exits. */
export class CastleExit extends Error {
  constructor(code) {
    super(`exit ${code}`);
    this.code = code;
  }
}

let sink = (text) => process.stdout.write(text + '\n');
/** Tests may catch the lines instead of printing them. */
export function setOutput(fn) {
  sink = fn;
}
export function say(text) {
  sink(String(text));
}
export function exit(code) {
  throw new CastleExit(code);
}

/** True when this module file is the script Node was started with. */
export function isMain(metaUrl) {
  if (!process.argv[1]) return false;
  try {
    return fs.realpathSync(fileURLToPath(metaUrl)) === fs.realpathSync(path.resolve(process.argv[1]));
  } catch {
    return false;
  }
}

/** Runs a helper's main(argv) and ends the process with its code once stdout is flushed. */
export function runMain(main) {
  Promise.resolve()
    .then(() => main(process.argv.slice(2)))
    .then(
      () => 0,
      (err) => {
        if (err instanceof CastleExit) return err.code;
        say('Error: ' + protectSecret(err && err.message ? err.message : String(err)));
        return 1;
      },
    )
    .then((code) => {
      process.exitCode = code;
      process.stdout.write('', () => process.exit(code));
    });
}

// ---------- arguments: -Name value, --name value, --kebab-name value ----------

const norm = (s) => s.toLowerCase().replace(/[-_]/g, '');

/**
 * Parses PowerShell-style and GNU-style arguments, names case-insensitive:
 *   -OutFile x   --outfile x   --out-file x   -OutFile:x   --out-file=x   -WhatIfCost   --what-if-cost
 * A unique prefix works as in PowerShell (-Out for -OutFile when nothing else starts so).
 * spec = { params: { Name: { type: 'string'|'int'|'switch', mandatory, set: [...], min, max, default, repeat } },
 *          (repeat: N - the value may be given up to N times and arrives as a list, in order)
 *          positional: ['Name', ...] }   (value parameters that may be given without their name)
 * Bad input prints one "Error: ..." line and exits 2.
 */
export function parseArgs(argv, spec) {
  const names = Object.keys(spec.params);
  const byNorm = new Map(names.map((n) => [norm(n), n]));
  const out = {};
  const seen = new Set();
  const positional = [...(spec.positional || [])];
  const shown = (n) => '-' + n;

  const lookup = (raw) => {
    const key = norm(raw);
    if (byNorm.has(key)) return byNorm.get(key);
    const hits = names.filter((n) => norm(n).startsWith(key));
    if (hits.length === 1) return hits[0];
    if (hits.length > 1) badInput(`${shown(raw)} could mean ${hits.map(shown).join(' or ')}; say which.`);
    badInput(`unknown argument ${shown(raw)} (known: ${names.map(shown).join(', ')}).`);
  };

  const setValue = (name, value) => {
    const p = spec.params[name];
    if (p.repeat) {
      const list = out[name] || [];
      if (list.length >= p.repeat) badInput(`${shown(name)} can be given at most ${p.repeat} times.`);
      seen.add(name);
      out[name] = [...list, value];
      return;
    }
    if (seen.has(name)) badInput(`${shown(name)} was given more than once.`);
    seen.add(name);
    if (p.type === 'switch') {
      out[name] = value;
      return;
    }
    if (p.set) {
      const hit = p.set.find((s) => s.toLowerCase() === String(value).toLowerCase());
      if (hit === undefined) badInput(`${shown(name)} must be one of ${p.set.join(', ')} (not '${value}').`);
      value = hit;
    }
    if (p.type === 'int') {
      if (!/^[+-]?\d+$/.test(String(value).trim())) badInput(`${shown(name)} must be a whole number (not '${value}').`);
      value = parseInt(value, 10);
      if ((p.min !== undefined && value < p.min) || (p.max !== undefined && value > p.max)) {
        badInput(`${shown(name)} must be from ${p.min} to ${p.max} (not ${value}).`);
      }
    }
    out[name] = value;
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const m = /^--?([A-Za-z][A-Za-z0-9_-]*)(?:[:=](.*))?$/s.exec(arg);
    if (!m) {
      const next = positional.find((n) => !seen.has(n));
      if (!next) badInput(`unexpected argument '${arg}'.`);
      positional.splice(positional.indexOf(next), 1);
      setValue(next, arg);
      continue;
    }
    const name = lookup(m[1]);
    const p = spec.params[name];
    if (p.type === 'switch') {
      let on = true;
      if (m[2] !== undefined) {
        const v = m[2].trim().toLowerCase().replace(/^\$/, '');
        if (['true', '1', 'yes', ''].includes(v)) on = true;
        else if (['false', '0', 'no'].includes(v)) on = false;
        else badInput(`${shown(name)} is a switch; give it alone (or :$false).`);
      }
      setValue(name, on);
    } else if (m[2] !== undefined) {
      setValue(name, m[2]);
    } else {
      if (i + 1 >= argv.length) badInput(`${shown(name)} needs a value.`);
      setValue(name, argv[++i]);
    }
  }

  for (const n of names) {
    const p = spec.params[n];
    if (!(n in out)) {
      if (p.mandatory) badInput(`${shown(n)} is needed.`);
      out[n] = p.type === 'switch' ? false : p.default;
    }
  }
  return out;
}

// ---------- keys and secrets ----------

/**
 * The key from the named environment variable, or null when it is missing, empty, or obviously
 * a placeholder ("your-key-here", "<key>", ...).
 */
export function getCastleKey(name) {
  let value = process.env[name];
  if (value === undefined || value === null || !String(value).trim()) return null;
  value = trimChars(trimChars(String(value).trim(), '"'), "'").trim();
  if (value.length < 8) return null;
  if (/\s/.test(value)) return null;
  if (/\.\.\./.test(value)) return null;
  if (/^[<[{(]/.test(value)) return null;
  if (/^(sk-)?(your|insert|enter|paste|put|add|replace|changeme|change-me|placeholder|example|xxx|todo|tbd|none|null|undefined|empty|missing|key-here|api-key-here)/i.test(value)) return null;
  if (/(here|placeholder|changeme)$/i.test(value)) return null;
  if (/^(.)\1+$/is.test(value)) return null;
  return value;
}

function trimChars(s, ch) {
  let a = 0;
  let b = s.length;
  while (a < b && s[a] === ch) a++;
  while (b > a && s[b - 1] === ch) b--;
  return s.slice(a, b);
}

/** Removes anything that looks like a key from text before it is shown. */
export function protectSecret(text) {
  if (text === undefined || text === null || text === '') return '';
  let out = String(text);
  for (const n of KEY_NAMES) {
    const v = process.env[n];
    if (v && v.trim() && v.trim().length >= 6) out = out.split(v.trim()).join('[hidden]');
  }
  out = out.replace(/(bearer\s+)[^\s"',;]+/gi, '$1[hidden]');
  out = out.replace(/(xi-api-key|api[_-]?key|authorization|token|secret)("?\s*[:=]\s*"?)[^\s"',;]+/gi, '$1$2[hidden]');
  out = out.replace(/sk-[A-Za-z0-9_-]{6,}/g, '[hidden]');
  out = out.replace(/[A-Za-z0-9_-]{32,}/g, '[hidden]');
  return out;
}

// ---------- failures ----------

/** A failed HTTP answer, carrying its status and body (as PowerShell's web errors do). */
export class HttpError extends Error {
  constructor(status, body) {
    super(`HTTP ${status}`);
    this.status = status;
    this.body = body;
  }
}

/** The API's own words from an error body (JSON or plain text). */
function apiWords(detail) {
  try {
    const j = JSON.parse(detail);
    if (j && typeof j === 'object') {
      if (j.error && typeof j.error === 'object' && j.error.message) return String(j.error.message);
      if (typeof j.error === 'string') return j.error;
      if (j.detail && typeof j.detail === 'object' && j.detail.message) return String(j.detail.message);
      if (typeof j.detail === 'string') return j.detail;
      if (j.msg) return String(j.msg);
      if (j.message) return String(j.message);
    }
  } catch {
    // not JSON: the text itself
  }
  return detail;
}

/** One line describing a failed request: HTTP status + the API's own words. */
export function errorText(err) {
  const parts = [];
  let detail;
  if (err instanceof HttpError) {
    parts.push(`HTTP ${err.status}`);
    detail = err.body || '';
  } else {
    detail = (err && err.cause && err.cause.message) || (err && err.message) || String(err);
  }
  if (detail) parts.push(apiWords(detail));
  let line = parts.join(' - ').replace(/\s+/g, ' ');
  line = protectSecret(line.trim());
  if (line.length > 400) line = line.slice(0, 400) + '...';
  return line;
}

/** Prints one clear line (secrets removed) and exits 1 (9 when the test guard blocked it). */
export function fail(what, { error, message } = {}) {
  let detail = message;
  if (error) {
    if (error.message && error.message.startsWith('CASTLE_NO_NETWORK')) {
      say(protectSecret(error.message));
      exit(9);
    }
    if (error instanceof CastleExit) throw error;
    detail = errorText(error);
  }
  say(`Error: ${what} failed: ` + protectSecret(detail));
  exit(1);
}

export function badInput(message) {
  say('Error: ' + protectSecret(message));
  exit(2);
}

/**
 * Called right before every request. Tests set CASTLE_NO_NETWORK=1 to prove that
 * --what-if-cost and the no-key fallbacks never reach the network.
 */
export function assertNetworkAllowed(url) {
  if (process.env.CASTLE_NO_NETWORK === '1') {
    throw new Error(`CASTLE_NO_NETWORK: blocked a request to ${url}`);
  }
}

// ---------- the web ----------

/**
 * One request; answers { status, text } for a 2xx, throws HttpError otherwise.
 * `as: 'bytes'` answers { status, bytes } instead.
 */
export async function request(url, { method = 'GET', headers = {}, body, timeoutSec = 120, as = 'text' } = {}) {
  assertNetworkAllowed(url);
  const res = await fetch(url, { method, headers, body, signal: AbortSignal.timeout(timeoutSec * 1000) });
  if (!res.ok) {
    let text = '';
    try {
      text = await res.text();
    } catch {
      // no body
    }
    throw new HttpError(res.status, text);
  }
  if (as === 'bytes') return { status: res.status, bytes: Buffer.from(await res.arrayBuffer()) };
  return { status: res.status, text: await res.text() };
}

/** A JSON answer as an object ({} when it is not JSON, as Invoke-RestMethod hands back text). */
export async function requestJson(url, options = {}) {
  const { text } = await request(url, options);
  try {
    const v = JSON.parse(text);
    return v && typeof v === 'object' ? v : {};
  } catch {
    return {};
  }
}

/** UTF-8 JSON bytes of an object, so any language survives the trip. */
export function jsonBytes(object) {
  return Buffer.from(JSON.stringify(object), 'utf8');
}

/** Saves a download to a file. */
export async function download(url, file, timeoutSec) {
  const { bytes } = await request(url, { timeoutSec, as: 'bytes' });
  fs.writeFileSync(file, bytes);
}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// ---------- files and text ----------

/** Creates the folder that will hold the given file; answers the file's full path. */
export function folderFor(file) {
  const full = path.resolve(file);
  fs.mkdirSync(path.dirname(full), { recursive: true });
  return full;
}

export function readUtf8(file) {
  return fs.readFileSync(file, 'utf8').replace(/^﻿/, '');
}

export function writeUtf8(file, text) {
  fs.writeFileSync(file, text, 'utf8');
}

export function isFile(p) {
  try {
    return fs.statSync(p).isFile();
  } catch {
    return false;
  }
}

export function isDir(p) {
  try {
    return fs.statSync(p).isDirectory();
  } catch {
    return false;
  }
}

/** Accepts either the text itself or a path to a UTF-8 text file holding it. */
export function resolveText(value) {
  if (value.length < 260 && !/[\r\n]/.test(value) && isFile(value)) return readUtf8(value);
  return value;
}

export function slug(text, fallback = 'untitled') {
  let s = '';
  if (text) s = text.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  if (s.length > 60) s = s.slice(0, 60).replace(/^-+|-+$/g, '');
  return s || fallback;
}

const pad = (n, w = 2) => String(Math.trunc(Math.abs(n))).padStart(w, '0');

/** Local time with its offset, like 2026-11-17T14:52:03+08:00. */
export function now() {
  const d = new Date();
  const off = -d.getTimezoneOffset();
  const sign = off >= 0 ? '+' : '-';
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}${sign}${pad(off / 60)}:${pad(off % 60)}`;
}

/** UTC time like 2026-11-17T06:52:03Z. */
export function nowUtc() {
  return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/** Rounds half to even, as .NET's Math.Round does. */
export function roundEven(value, digits = 0) {
  const f = 10 ** digits;
  const x = value * f;
  const r = Math.round(x);
  if (Math.abs(x - Math.trunc(x)) !== 0 && Math.abs(Math.abs(x - Math.trunc(x)) - 0.5) < 1e-9) {
    const down = Math.floor(x);
    return (down % 2 === 0 ? down : down + 1) / f;
  }
  return r / f;
}

/** A number the way PowerShell prints it with the invariant culture (0.04, 1, 12.3). */
export function num(value) {
  return String(value === 0 ? 0 : value);
}

// ---------- where Castle's own files live ----------

/**
 * The per-user application data folder, the same place Rust's crate::env::app_data() reads:
 * Windows %APPDATA%, macOS ~/Library/Application Support, else $XDG_CONFIG_HOME or ~/.config.
 * CASTLE_APP_DATA_DIR moves it (tests only, like CASTLE_SPEND_DIR).
 */
export function appDataFolder() {
  const test = process.env.CASTLE_APP_DATA_DIR;
  if (test && test.trim()) return test;
  const home = os.homedir();
  if (process.platform === 'win32') {
    const set = process.env.APPDATA;
    return set && set.trim() ? set : path.join(home, 'AppData', 'Roaming');
  }
  if (process.platform === 'darwin') return path.join(home, 'Library', 'Application Support');
  return process.env.XDG_CONFIG_HOME && process.env.XDG_CONFIG_HOME.trim() ? process.env.XDG_CONFIG_HOME : path.join(home, '.config');
}

export function castleFolder() {
  return path.join(appDataFolder(), 'MemoryCore Castle');
}

/** One line in notices.jsonl, the toasts Castle shows. */
export function writeNotice(entry) {
  const dir = castleFolder();
  fs.mkdirSync(dir, { recursive: true });
  fs.appendFileSync(path.join(dir, 'notices.jsonl'), JSON.stringify(entry) + '\n', 'utf8');
}

const FALLBACK = {
  image: { suffix: '.prompt.md', setting: 'OPENAI_API_KEY', what: 'the picture prompt' },
  video: { suffix: '.prompt.md', setting: 'ARK_API_KEY', what: 'the video prompt' },
  song: { suffix: '.lyrics.md', setting: 'SUNOAPI_ORG_KEY', what: 'the lyrics and style' },
  voice: { suffix: '.line.txt', setting: 'ELEVENLABS_API_KEY', what: 'the line' },
};

/**
 * No key: save what would have been rendered (prompt, lyrics or line) next to the intended
 * output, leave one notice line for Castle, and exit 3.
 */
export function writeFallback(kind, outFile, content, reason) {
  const { suffix, setting, what } = FALLBACK[kind];
  const full = folderFor(outFile);
  const base = path.basename(full, path.extname(full));
  const saved = path.join(path.dirname(full), base + suffix);
  const header = `Saved instead of rendering: ${reason}. Add the key in Castle's Settings to make the real thing.`;
  const text = kind === 'voice' ? `${header}\r\n\r\n${content}\r\n` : `> ${header}\r\n\r\n${content}\r\n`;
  writeUtf8(saved, text);
  writeNotice({ at: now(), kind, file: saved, reason, settings: setting });
  say(`No ${kind} key is set (${setting}), so ${what} was saved to ${saved} instead of rendering.`);
  exit(3);
}

/** Small result block the companion (and Castle's transcript reader) reads back. */
export function writeResult(savedPaths, costUsd, elapsedSeconds, model, kind = '') {
  say('result:');
  for (const p of [].concat(savedPaths)) say(`  saved_path: ${p}`);
  say('  cost_estimate_usd: ' + num(roundEven(costUsd, 4)));
  say('  elapsed_seconds: ' + num(roundEven(elapsedSeconds, 1)));
  say(`  model: ${model}`);
  if (kind) writeSpend(kind, costUsd);
}

/** Where the spend lines live; CASTLE_SPEND_DIR moves it (tests only). */
export function spendFolder() {
  const set = process.env.CASTLE_SPEND_DIR;
  if (set && set.trim()) return set;
  return path.join(castleFolder(), 'spend');
}

const PROVIDER = { image: 'openai', video: 'seedance', song: 'suno', voice: 'elevenlabs' };

/**
 * One line per real render, so HOME's credit widgets can count a balance down between reads.
 * Only writeResult calls it: an estimate or a blocked request never does. A spend line that
 * cannot be written never fails the render.
 */
export function writeSpend(kind, costUsd) {
  try {
    const dir = spendFolder();
    fs.mkdirSync(dir, { recursive: true });
    const line = JSON.stringify({ at: nowUtc(), usd: roundEven(costUsd, 4), kind }) + '\n';
    fs.appendFileSync(path.join(dir, `${PROVIDER[kind]}-spend.jsonl`), line, 'utf8');
  } catch {
    // the render itself succeeded; the widget simply counts one call fewer
  }
}

/** Printed by --what-if-cost; no request is made. */
export function writeEstimate(costUsd, model, detail) {
  say('estimate (no request made):');
  say('  cost_estimate_usd: ' + num(roundEven(costUsd, 4)));
  say(`  model: ${model}`);
  if (detail) say(`  basis: ${detail}`);
  say('  note: rough estimate only; the provider bill is the truth.');
}

/** The folder this helper lives in. */
export function toolsFolder(metaUrl) {
  return path.dirname(fileURLToPath(metaUrl));
}
