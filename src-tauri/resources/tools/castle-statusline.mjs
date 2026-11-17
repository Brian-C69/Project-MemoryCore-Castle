// castle-statusline.mjs - the status line of a Castle session.
//
//   node castle-statusline.mjs        (Claude Code pipes the session's JSON on stdin)
//
// Claude Code runs this after each turn and pipes its session as JSON on stdin (model,
// transcript_path and, in newer versions, context_window). It prints one line:
//   <model> | NN% | 123.4k tokens
// - how much of the context window the conversation now fills. Castle's Chat reads that line
// from the Log screen for its context meter.
//
// Nothing personal is read or written: only the JSON and the session's own transcript. Never
// fails loudly - a status line that cannot be worked out prints "--". Imports nothing of Castle's
// own, so it starts fast.

import fs from 'node:fs';

/** Rounds half to even, as .NET's Math.Round does. */
function roundEven(value, digits = 0) {
  const f = 10 ** digits;
  const x = value * f;
  const frac = Math.abs(x - Math.trunc(x));
  if (frac !== 0 && Math.abs(frac - 0.5) < 1e-9) {
    const down = Math.floor(x);
    return (down % 2 === 0 ? down : down + 1) / f;
  }
  return Math.round(x) / f;
}

function tokenText(tokens) {
  if (tokens >= 1000000) return String(roundEven(tokens / 1000000, 1)) + 'M';
  if (tokens >= 1000) return String(roundEven(tokens / 1000, 1)) + 'k';
  return String(roundEven(tokens));
}

function number(value) {
  if (value === undefined || value === null) return 0;
  const n = Number(value);
  return Number.isFinite(n) ? n : 0;
}

// What one API answer put in the context: the input it was sent, read from the cache or written
// to it. (Its own output is part of the next answer's input.)
function usageTokens(usage) {
  if (usage === undefined || usage === null) return 0;
  return number(usage.input_tokens) + number(usage.cache_creation_input_tokens) + number(usage.cache_read_input_tokens);
}

/** The last `count` lines of a file, read from its end. */
function tailLines(file, count) {
  const fd = fs.openSync(file, 'r');
  try {
    const size = fs.fstatSync(fd).size;
    const chunk = 1 << 20;
    let pos = size;
    let buf = Buffer.alloc(0);
    let newlines = 0;
    while (pos > 0 && newlines <= count) {
      const len = Math.min(chunk, pos);
      pos -= len;
      const part = Buffer.alloc(len);
      fs.readSync(fd, part, 0, len, pos);
      for (const b of part) if (b === 10) newlines++;
      buf = Buffer.concat([part, buf]);
    }
    const lines = buf.toString('utf8').replace(/^﻿/, '').split(/\r?\n/);
    if (lines.length && lines[lines.length - 1] === '') lines.pop();
    return lines.slice(-count);
  } finally {
    fs.closeSync(fd);
  }
}

// The context used, from the newest main-thread answer in the transcript that carries a usage
// block (a side agent's answers are its own context, not this one).
function transcriptTokens(file) {
  if (!file || !file.trim()) return null;
  try {
    if (!fs.statSync(file).isFile()) return null;
  } catch {
    return null;
  }
  const lines = tailLines(file, 400);
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    if (!/"usage"/i.test(line) || !/"assistant"/i.test(line)) continue;
    let entry;
    try {
      entry = JSON.parse(line);
    } catch {
      continue;
    }
    if (!entry || entry.type !== 'assistant' || entry.isSidechain === true || entry.isApiErrorMessage === true) continue;
    const tokens = usageTokens(entry.message ? entry.message.usage : null);
    if (tokens > 0) return tokens;
  }
  return null;
}

let model = 'Claude';
let percent = '--';
let tokens_ = '--';
try {
  let raw = '';
  try {
    raw = fs.readFileSync(0, 'utf8');
  } catch {
    raw = '';
  }
  const j = JSON.parse(raw);
  const m = (j && j.model) || {};
  if (m.display_name) model = String(m.display_name);
  else if (m.id) model = String(m.id);

  const window = j.context_window;
  // the window's size: as Claude Code says it, else 1M for a [1m] model, else 200k
  let size = number(window ? window.context_window_size : null);
  if (size <= 0) {
    size = 200000;
    if (/\[1m\]|1m context|\(1M/i.test(`${m.id ?? ''} ${m.display_name ?? ''}`)) size = 1000000;
  }

  let tokens = null;
  if (window && window.current_usage !== undefined && window.current_usage !== null) tokens = usageTokens(window.current_usage);
  if (!tokens) tokens = transcriptTokens(j.transcript_path === undefined || j.transcript_path === null ? '' : String(j.transcript_path));

  if (window && window.used_percentage !== undefined && window.used_percentage !== null) {
    percent = String(Math.min(100, roundEven(number(window.used_percentage))));
    if (!tokens) tokens = roundEven((number(window.used_percentage) / 100) * size);
  } else if (tokens) {
    percent = String(Math.min(100, roundEven((tokens / size) * 100)));
  } else {
    // a fresh session: nothing has been sent yet
    percent = '0';
    tokens = 0;
  }
  tokens_ = tokenText(tokens);
} catch {
  percent = '--';
  tokens_ = '--';
}

process.stdout.write(`${model} | ${percent}% | ${tokens_} tokens\n`);
