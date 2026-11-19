// test-helpers.mjs - offline checks for the Castle helper scripts.
//
//   node src-tauri/resources/tools/test-helpers.mjs
//
// Every helper runs in a child Node process with the four key variables removed, Castle's app
// data folder pointed at a temp folder (CASTLE_APP_DATA_DIR) and CASTLE_NO_NETWORK=1, so nothing
// here can reach a paid API. Checks (the same as test-helpers.ps1, plus a mock server):
//   - no key            -> exit 3, fallback file written, one notices.jsonl line
//   - CASTLE_IMAGE=codex -> castle-image exits 4 and writes nothing
//   - --what-if-cost with a fake key -> exit 0, no request, nothing written
//   - fake key, no --what-if-cost -> the network guard fires (exit 9), proving the guard sits in
//     front of the request and the --what-if-cost runs are honest
//   - castle-world, castle-beat and castle-look behave; protectSecret and getCastleKey behave
//   - the real request paths, against a local mock server (a preload sends every fetch there,
//     and shortens the poll waits), so the bodies, polling, downloads and result block are proved
// Prints PASS/FAIL lines and exits 1 when anything fails.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import http from 'node:http';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import * as common from './castle-common.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const keyNames = common.KEY_NAMES;
const fakeKey = 'castle-test-fake-0123456789';

const root = path.join(os.tmpdir(), 'castle-helpers-test-' + crypto.randomBytes(4).toString('hex'));
const appData = path.join(root, 'appdata');
const out = path.join(root, 'out folder');
fs.mkdirSync(appData, { recursive: true });
fs.mkdirSync(out, { recursive: true });
const notices = path.join(appData, 'MemoryCore Castle', 'notices.jsonl');

let failures = 0;
function report(ok, name, detail = '') {
  if (ok) console.log(`PASS  ${name}`);
  else {
    failures++;
    console.log(`FAIL  ${name}`);
    if (detail) console.log('      ' + String(detail).replace(/\r?\n/g, '\n      '));
  }
}

/** Runs one helper in a fresh Node with a controlled environment. */
function run(script, args, setEnv = {}, { cwd, nodeArgs = [], stdin } = {}) {
  const env = { ...process.env };
  for (const k of [...keyNames, 'CASTLE_IMAGE', 'CASTLE_CLI', 'CASTLE_SPEND_DIR', 'CASTLE_STYLES', 'APPDATA']) delete env[k];
  env.CASTLE_APP_DATA_DIR = appData;
  env.CASTLE_TOOLS = here;
  env.CASTLE_NO_NETWORK = '1';
  for (const [k, v] of Object.entries(setEnv)) {
    if (v === null) delete env[k];
    else env[k] = String(v);
  }
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [...nodeArgs, path.join(here, script), ...args], { env, cwd: cwd || root, windowsHide: true });
    let text = '';
    child.stdout.on('data', (d) => (text += d));
    child.stderr.on('data', (d) => (text += d));
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${script} timed out`));
    }, 120000);
    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({ code, out: text.trim() });
    });
    if (stdin !== undefined) child.stdin.end(stdin);
    else child.stdin.end();
  });
}

const exists = (p) => fs.existsSync(p);
const noticeCount = () => (exists(notices) ? fs.readFileSync(notices, 'utf8').split('\n').filter((l) => l.trim()).length : 0);
const lastNotice = () => JSON.parse(fs.readFileSync(notices, 'utf8').split('\n').filter((l) => l.trim()).at(-1));
const readJson = (p) => {
  try {
    return JSON.parse(fs.readFileSync(p, 'utf8'));
  } catch {
    return null;
  }
};

const lyricsFile = path.join(root, 'lyrics.md');
fs.writeFileSync(lyricsFile, 'Verse one\r\nUnder the castle light\r\n', 'utf8');

const cases = [
  { kind: 'image', script: 'castle-image.mjs', key: 'OPENAI_API_KEY', target: path.join(out, 'pic.png'), fallback: path.join(out, 'pic.prompt.md'), needle: 'a lantern in the rain',
    args: ['-Prompt', 'a lantern in the rain', '-OutFile', path.join(out, 'pic.png')], env: { CASTLE_IMAGE: 'openai' } },
  { kind: 'video', script: 'castle-video.mjs', key: 'ARK_API_KEY', target: path.join(out, 'clip.mp4'), fallback: path.join(out, 'clip.prompt.md'), needle: 'waves at dusk',
    args: ['--prompt', 'waves at dusk', '--out-file', path.join(out, 'clip.mp4')], env: {} },
  { kind: 'song', script: 'castle-song.mjs', key: 'SUNOAPI_ORG_KEY', target: path.join(out, 'songs', '01-castle-light.mp3'), fallback: path.join(out, 'songs', 'castle-light.lyrics.md'), needle: 'Under the castle light',
    args: ['-Title', 'Castle Light', '-Style', 'soft piano, warm vocal', '-Lyrics', lyricsFile, '-OutFolder', path.join(out, 'songs')], env: {} },
  { kind: 'voice', script: 'castle-voice.mjs', key: 'ELEVENLABS_API_KEY', target: path.join(out, 'line.mp3'), fallback: path.join(out, 'line.line.txt'), needle: 'Good evening, welcome back.',
    args: ['--text', 'Good evening, welcome back.', '--out-file', path.join(out, 'line.mp3')], env: {} },
];

console.log(`Temp folder: ${root}`);
console.log('');

// 1) No key -> fallback, exit 3, notice line.
for (const c of cases) {
  const before = noticeCount();
  const r = await run(c.script, c.args, c.env);
  report(r.code === 3, `${c.kind}: no key exits 3`, `exit ${r.code}: ${r.out}`);
  const fbText = exists(c.fallback) ? fs.readFileSync(c.fallback, 'utf8') : '';
  report(fbText.includes('Saved instead of rendering:') && fbText.includes("Castle's Settings") && fbText.includes(c.needle), `${c.kind}: fallback file holds header and content`, `file: ${c.fallback}`);
  report(!exists(c.target), `${c.kind}: no rendered file was written`);
  const after = noticeCount();
  let noticeOk = false;
  let noticeDetail = `lines before ${before}, after ${after}`;
  if (after === before + 1) {
    try {
      const n = lastNotice();
      noticeOk = n.kind === c.kind && n.settings === c.key && n.file === c.fallback && !!n.at && !!n.reason;
      noticeDetail = JSON.stringify(n);
    } catch (e) {
      noticeDetail = e.message;
    }
  }
  report(noticeOk, `${c.kind}: one notices.jsonl line (kind, file, settings)`, noticeDetail);
  report(!/castle-test-fake/.test(r.out), `${c.kind}: no key text in output`);
}

// 2) Codex draws its own pictures -> exit 4, nothing written.
{
  const codexOut = path.join(out, 'codex.png');
  const before = noticeCount();
  const r = await run('castle-image.mjs', ['-Prompt', 'a fox', '-OutFile', codexOut], { CASTLE_IMAGE: 'codex', CASTLE_CLI: 'codex', OPENAI_API_KEY: fakeKey });
  report(r.code === 4 && /Codex draws pictures with its own image tool/.test(r.out), 'image: CASTLE_IMAGE=codex exits 4 with the hand-off line', `exit ${r.code}: ${r.out}`);
  report(!exists(codexOut) && !exists(path.join(out, 'codex.prompt.md')) && noticeCount() === before, 'image: codex hand-off writes no file and no notice');
}

// 3) --what-if-cost with a fake key -> exit 0, estimate printed, no request.
for (const c of cases) {
  const env = { ...c.env, [c.key]: fakeKey };
  const before = noticeCount();
  const flag = c.kind === 'video' ? '--what-if-cost' : '-WhatIfCost';
  const r = await run(c.script, [...c.args, flag], env);
  report(r.code === 0 && /cost_estimate_usd: \d/.test(r.out) && !/CASTLE_NO_NETWORK/.test(r.out), `${c.kind}: ${flag} exits 0 before any request`, `exit ${r.code}: ${r.out}`);
  report(!exists(c.target) && noticeCount() === before && !/castle-test-fake/.test(r.out), `${c.kind}: ${flag} writes nothing and shows no key`);
}

// 4) Fake key without --what-if-cost -> the network guard is the first thing hit.
for (const c of cases) {
  const r = await run(c.script, c.args, { ...c.env, [c.key]: fakeKey });
  report(r.code === 9 && /CASTLE_NO_NETWORK: blocked/.test(r.out) && !/castle-test-fake/.test(r.out), `${c.kind}: with a key the request path is guarded (exit 9)`, `exit ${r.code}: ${r.out}`);
}

// 4b) No estimate and no blocked request ever writes a spend line for HOME's credit widgets.
report(!exists(path.join(appData, 'MemoryCore Castle', 'spend')), 'common: estimates and blocked requests write no spend line');

// 5) Image edit path with a missing reference -> exit 2 (bad input), no request.
{
  const r = await run('castle-image.mjs', ['-Prompt', 'restyle', '-OutFile', path.join(out, 'ref.png'), '-Reference', path.join(out, 'missing.png')], { CASTLE_IMAGE: 'openai', OPENAI_API_KEY: fakeKey });
  report(r.code === 2, 'image: missing -Reference file exits 2', `exit ${r.code}: ${r.out}`);
}

// 5b) Arguments: both spellings, case-insensitive, and bad ones refused with exit 2.
{
  const r1 = await run('castle-image.mjs', ['--PROMPT', 'x', '--out-file', path.join(out, 'a.png'), '--size', '1536X1024', '--quality=HIGH', '-whatifcost'], { OPENAI_API_KEY: fakeKey });
  report(r1.code === 0 && /cost_estimate_usd: 0\.25/.test(r1.out) && /1536x1024, high quality/.test(r1.out), 'args: --kebab, -Pascal, =value and any case all work', `exit ${r1.code}: ${r1.out}`);
  const r2 = await run('castle-image.mjs', ['-Prompt', 'x', '-OutFile', path.join(out, 'a.png'), '-Size', '999x999', '-WhatIfCost']);
  report(r2.code === 2 && /^Error: -Size must be one of/m.test(r2.out), 'args: a value outside the set exits 2 with an Error line', `exit ${r2.code}: ${r2.out}`);
  const r3 = await run('castle-image.mjs', ['-Prompt', 'x', '-OutFile', path.join(out, 'a.png'), '-Bogus', 'y']);
  report(r3.code === 2 && /unknown argument -Bogus/.test(r3.out), 'args: an unknown argument exits 2', `exit ${r3.code}: ${r3.out}`);
  const r4 = await run('castle-video.mjs', ['-OutFile', path.join(out, 'a.mp4')]);
  report(r4.code === 2 && /-Prompt is needed/.test(r4.out), 'args: a missing mandatory argument exits 2', `exit ${r4.code}: ${r4.out}`);
  const r5 = await run('castle-video.mjs', ['-Prompt', 'x', '-OutFile', path.join(out, 'a.mp4'), '-Seconds', '40', '-WhatIfCost']);
  report(r5.code === 2 && /-Seconds must be from 1 to 15/.test(r5.out), 'args: -Seconds outside 1..15 exits 2', `exit ${r5.code}: ${r5.out}`);
  const r6 = await run('castle-video.mjs', ['-Prompt', 'x', '-OutFile', path.join(out, 'a.mp4'), '-WhatIfCost:$false']);
  report(r6.code === 3, 'args: -Switch:$false turns a switch off', `exit ${r6.code}: ${r6.out}`);
}

// 6) The world prepare helper: a missing piece is named, and no Node leaves a notice.
const worldDir = path.join(root, 'world');
const hd = path.join(worldDir, 'hd');
fs.mkdirSync(hd, { recursive: true });
const manifest = path.join(worldDir, 'world.json');
fs.writeFileSync(manifest, '{"slug":"t","pieces":[{"file":"01-base.glb","role":"base","size":40},{"file":"02-crystal.glb","role":"companion","size":9}]}', 'utf8');
fs.writeFileSync(path.join(hd, '01 my base export.glb'), Buffer.from([0x67, 0x6c, 0x54, 0x46]));
{
  const r = await run('castle-world.mjs', ['-Prepare', '-Manifest', manifest, '-In', hd, '-Out', path.join(worldDir, 'pieces')]);
  report(r.code === 2 && /02-crystal\.glb/.test(r.out) && !/01-base/.test(r.out), 'world: a missing piece is named (exit 2)', `exit ${r.code}: ${r.out}`);
}
fs.writeFileSync(path.join(hd, '2.glb'), Buffer.from([0x67, 0x6c, 0x54, 0x46]));
// "no Node": an empty PATH, and the Node running the helper pretends to live nowhere
const emptyBin = path.join(root, 'empty-bin');
fs.mkdirSync(emptyBin, { recursive: true });
const hideNode = path.join(root, 'hide-node.mjs');
fs.writeFileSync(hideNode, "process.execPath = '/nowhere/node';\n", 'utf8');
{
  const before = noticeCount();
  const r = await run('castle-world.mjs', ['-Prepare', '-Manifest', manifest, '-In', hd, '-Out', path.join(worldDir, 'pieces')], { PATH: emptyBin, Path: null }, { nodeArgs: ['--import', pathToFileURL(hideNode).href] });
  const noticeOk = noticeCount() === before + 1 && lastNotice().kind === 'world';
  report(r.code === 3 && /Node is not installed/.test(r.out) && noticeOk, 'world: no Node exits 3 with a notice', `exit ${r.code}: ${r.out}`);
  report(!exists(path.join(worldDir, 'pieces')), 'world: nothing written without Node');
}

// 6b) A fake npx that copies its input: the plan, the triangle count and the progress lines.
function glb(json) {
  let text = JSON.stringify(json);
  while (Buffer.byteLength(text) % 4) text += ' ';
  const chunk = Buffer.from(text, 'utf8');
  const head = Buffer.alloc(20);
  head.writeUInt32LE(0x46546c67, 0);
  head.writeUInt32LE(2, 4);
  head.writeUInt32LE(20 + chunk.length, 8);
  head.writeUInt32LE(chunk.length, 12);
  head.writeUInt32LE(0x4e4f534a, 16);
  return Buffer.concat([head, chunk]);
}
{
  const hd2 = path.join(worldDir, 'hd2');
  fs.mkdirSync(hd2, { recursive: true });
  fs.writeFileSync(path.join(hd2, '01-base.glb'), glb({ meshes: [{ primitives: [{ indices: 0 }, { attributes: { POSITION: 1 } }, { mode: 1, indices: 0 }] }], accessors: [{ count: 300000 }, { count: 30 }] }));
  fs.writeFileSync(path.join(hd2, '002_crystal.GLB'), glb({ meshes: [{ primitives: [{ indices: 0 }] }], accessors: [{ count: 900 }] }));
  const fakeBin = path.join(root, 'fake-bin');
  fs.mkdirSync(fakeBin, { recursive: true });
  const argLog = path.join(root, 'npx-args.txt');
  if (process.platform === 'win32') {
    fs.writeFileSync(path.join(fakeBin, 'npx.cmd'), `@echo %* >> "${argLog}"\r\n@copy /y %4 %5 >nul\r\n`, 'utf8');
  } else {
    fs.writeFileSync(path.join(fakeBin, 'npx'), `#!/bin/sh\necho "$@" >> "${argLog}"\n/bin/cp "$4" "$5"\n`, { mode: 0o755 });
  }
  const pieces = path.join(worldDir, 'pieces2');
  const r = await run('castle-world.mjs', ['--prepare', '--manifest', manifest, '--in', hd2, '--out', pieces], { PATH: fakeBin, Path: null, CASTLE_NO_NETWORK: null });
  // cmd's %* keeps the quotes Node put around each argument; the check reads the arguments, not the quoting
  const said = exists(argLog) ? fs.readFileSync(argLog, 'utf8').replace(/"/g, '') : '';
  report(
    r.code === 0 && exists(path.join(pieces, '01-base.glb')) && exists(path.join(pieces, '02-crystal.glb')) &&
      /piece 01-base\.glb: 0\.0 MB -> 0\.0 MB, 100010 -> 100010 triangles \(1\/2\)/.test(r.out) && /result: prepared 2 pieces into /.test(r.out),
    'world: each piece goes through npx with a progress line and a result line',
    `exit ${r.code}: ${r.out}`,
  );
  report(/--simplify-ratio 0\.5999/.test(said) && /@gltf-transform\/cli@4 optimize/.test(said) && /--simplify false/.test(said), 'world: the base is simplified to its budget, a small piece is not', said);
}

// 7) The story beat helper: a good card is numbered and written whole, a bad one is refused.
const adventures = path.join(root, 'adventures');
fs.mkdirSync(path.join(adventures, 'sky-kingdom'), { recursive: true });
const beat = (args) => run('castle-beat.mjs', args, {}, { cwd: adventures });
{
  const goodCard = path.join(root, 'good-card.json');
  fs.writeFileSync(goodCard, '{"chapter":"Chapter 1","scene":"scenes/01-gate.png","lines":[{"speaker":"","text":"Mist over the bridge."},{"speaker":"Nova","text":"Stay close."}],"choices":["Cross now","Wait","(Say something)"]}', 'utf8');
  let r = await beat(['card', '-World', 'sky-kingdom', '-Json', goodCard]);
  const beatFile = path.join(adventures, 'sky-kingdom', 'stage', 'beat-0001.json');
  report(r.code === 0 && exists(beatFile) && !exists(beatFile + '.partial') && r.out === 'beat 1 written: stage/beat-0001.json', 'beat: a good card is written whole as beat-0001', `exit ${r.code}: ${r.out}`);
  const written = readJson(beatFile);
  report(written && written.beat === 1 && written.lines.length === 2, 'beat: the card carries its number and lines');
  r = await beat(['next', '--world', 'sky-kingdom']);
  report(r.code === 0 && r.out === '0002', 'beat: next gives the following number', `exit ${r.code}: ${r.out}`);
  const badCard = path.join(root, 'bad-card.json');
  fs.writeFileSync(badCard, '{"lines":[{"speaker":"Nova","text":"hi","mood":"happy"}],"choices":["only one"]}', 'utf8');
  r = await beat(['card', '-World', 'sky-kingdom', '-Json', badCard]);
  report(r.code === 2 && /mood/.test(r.out) && !exists(path.join(adventures, 'sky-kingdom', 'stage', 'beat-0002.json')), 'beat: an unknown key is refused and nothing written', `exit ${r.code}: ${r.out}`);
  const escape = path.join(root, 'escape-card.json');
  fs.writeFileSync(escape, '{"scene":"../../secret.png","lines":[{"speaker":"","text":"x"}],"choices":[]}', 'utf8');
  r = await beat(['card', '-World', 'sky-kingdom', '-Json', escape]);
  report(r.code === 2, 'beat: a scene outside the world is refused', `exit ${r.code}: ${r.out}`);
  r = await beat(['card', '-World', 'sky-kingdom', '-Json', '{"lines":[{"speaker":"","text":"inline"}],"choices":["a","b"]}']);
  report(r.code === 0 && /beat 2 written/.test(r.out), 'beat: a card given inline as JSON text is written too', `exit ${r.code}: ${r.out}`);
}

// 7b) castle-look: the style's slots are filled from the look's words (from the database), never by an AI.
const writeWords = (name, object) => {
  const p = path.join(root, name);
  fs.writeFileSync(p, JSON.stringify(object, null, 2), 'utf8');
  return p;
};
const full = writeWords('look.json', { words: 'Violet eyes, pale skin, slender build.', artStyle: 'soft watercolor illustration', outfit: { name: 'Winter coat', words: 'A long cream wool coat with brass buttons.' }, hair: { name: 'Long bob', words: 'Shoulder length, curled inward.' } });
const plain = writeWords('plain.json', { words: 'Violet eyes.', artStyle: '', outfit: null, hair: null });
const empty = writeWords('empty.json', { words: '', artStyle: '', outfit: null, hair: null });
const parse = (t) => {
  try {
    return JSON.parse(t);
  } catch {
    return null;
  }
};
{
  let r = await run('castle-look.mjs', ['-Kind', 'tryon', '-Words', full, '-Changes', 'warmer light', '-PromptOnly']);
  let look = parse(r.out);
  report(
    r.code === 0 && look && !/\{\{/.test(look.prompt) && /soft watercolor illustration character portrait/.test(look.prompt) && /Violet eyes/.test(look.prompt) &&
      /Winter coat - A long cream wool coat/.test(look.prompt) && /Long bob - Shoulder length/.test(look.prompt) && /Requested changes.*warmer light/.test(look.prompt) &&
      look.size === '1024x1536' && look.quality === 'medium',
    "look: -PromptOnly fills every slot from the words, with the style's size and quality",
    `exit ${r.code}: ${r.out}`,
  );
  r = await run('castle-look.mjs', ['--kind', 'standard', '--words', plain, '--quality', 'low', '--prompt-only']);
  look = parse(r.out);
  report(r.code === 0 && look && look.quality === 'low' && /the outfit described in the character notes/.test(look.prompt), 'look: the chosen quality wins over the style, and no outfit points at the notes', `exit ${r.code}: ${r.out}`);
  r = await run('castle-look.mjs', ['-Kind', 'standard', '-Words', empty, '-PromptOnly']);
  report(r.code === 2 && /no words yet/.test(r.out), 'look: no words and no picture are refused (they describe nobody)', `exit ${r.code}: ${r.out}`);
  const ref = path.join(root, 'portrait.png');
  fs.writeFileSync(ref, Buffer.from([0x89, 0x50, 0x4e, 0x47]));
  r = await run('castle-look.mjs', ['-Kind', 'standard', '-Words', empty, '-Reference', ref, '-PromptOnly']);
  look = parse(r.out);
  report(r.code === 0 && look && /same person as in the reference picture/.test(look.prompt), 'look: with a picture and no words, the picture is followed', `exit ${r.code}: ${r.out}`);
  r = await run('castle-look.mjs', ['-Kind', 'standard', '-Words', path.join(root, 'missing.json'), '-PromptOnly']);
  report(r.code === 2, 'look: a missing words file is refused', `exit ${r.code}: ${r.out}`);
  const lookOut = path.join(out, 'portrait.png');
  const before = noticeCount();
  r = await run('castle-look.mjs', ['-Kind', 'standard', '-Words', plain, '-OutFile', lookOut]);
  report(r.code === 3 && !exists(lookOut) && noticeCount() === before + 1, 'look: no key -> castle-image saves the prompt (exit 3), nothing drawn', `exit ${r.code}: ${r.out}`);
  r = await run('castle-look.mjs', ['-Kind', 'standard', '-Words', plain, '-OutFile', lookOut], { OPENAI_API_KEY: fakeKey });
  report(r.code === 9 || /CASTLE_NO_NETWORK/.test(r.out), 'look: with a key the request goes out through castle-image (the network guard fires)', `exit ${r.code}: ${r.out}`);
}

// 7c) The status line: the same answers the Rust test (session.rs) expects from the .ps1.
{
  const transcript = path.join(root, 't.jsonl');
  fs.writeFileSync(
    transcript,
    [
      '{"type":"user","message":{"content":"hi"}}',
      '{"type":"assistant","message":{"usage":{"input_tokens":10,"cache_creation_input_tokens":2000,"cache_read_input_tokens":40000,"output_tokens":50}}}',
      '{"type":"assistant","isSidechain":true,"message":{"usage":{"input_tokens":999999}}}',
      '{"type":"user","message":{"content":"next"}}',
    ].join('\n'),
  );
  const line = async (json) => (await run('castle-statusline.mjs', [], {}, { stdin: typeof json === 'string' ? json : JSON.stringify(json) })).out;
  const got = [
    await line({ model: { id: 'claude-x', display_name: 'Opus' }, transcript_path: transcript }),
    await line({ model: { id: 'claude-x[1m]', display_name: 'Opus' }, transcript_path: transcript }),
    await line({ model: { display_name: 'Sonnet' }, context_window: { used_percentage: 42.4, context_window_size: 200000 } }),
    await line({ model: { display_name: 'Sonnet' } }),
    await line('not json'),
  ];
  const want = ['Opus | 21% | 42k tokens', 'Opus | 4% | 42k tokens', 'Sonnet | 42% | 84.8k tokens', 'Sonnet | 0% | 0 tokens', 'Claude | --% | -- tokens'];
  report(JSON.stringify(got) === JSON.stringify(want), 'statusline: model, percent and tokens as session.rs expects', JSON.stringify(got));
}

// 8) Unit checks on the shared functions.
{
  const leak = common.protectSecret('HTTP 401 - Incorrect API key provided: sk-proj-AbCdEf1234567890XyZ. Authorization: Bearer abc.def.ghi xi-api-key: 0123456789abcdef');
  report(!/sk-proj-AbCd/.test(leak) && !/abc\.def\.ghi/.test(leak) && !/0123456789abcdef/.test(leak) && /HTTP 401/.test(leak), 'common: protectSecret strips key-like text', leak);
  const long = common.protectSecret('token ' + 'a1B2'.repeat(12));
  report(!/a1B2a1B2/.test(long), 'common: protectSecret strips long tokens', long);

  const body = common.jsonBytes({ prompt: `Mira's "look" - violet ✿ ミラ`, n: 1 });
  const back = parse(body.toString('utf8'));
  report(Buffer.isBuffer(body) && back && back.prompt === `Mira's "look" - violet ✿ ミラ` && back.n === 1, 'common: jsonBytes gives UTF-8 bytes of valid JSON');

  const saved = process.env.CASTLE_TEST_KEY;
  let okKey = true;
  for (const v of ['', '   ', 'your-key-here', '<paste key>', 'sk-...', 'changeme', 'xxxxxxxxxxxx', 'PLACEHOLDER']) {
    process.env.CASTLE_TEST_KEY = v;
    if (common.getCastleKey('CASTLE_TEST_KEY') !== null) {
      okKey = false;
      console.log(`      treated as a key: '${v}'`);
    }
  }
  process.env.CASTLE_TEST_KEY = `"${fakeKey}"`;
  if (common.getCastleKey('CASTLE_TEST_KEY') !== fakeKey) {
    okKey = false;
    console.log('      a real-looking key was refused');
  }
  if (saved === undefined) delete process.env.CASTLE_TEST_KEY;
  else process.env.CASTLE_TEST_KEY = saved;
  report(okKey, 'common: getCastleKey treats empty and placeholder values as missing');

  // A real render's result block writes one spend line for its provider, and stdout stays the same.
  const spendDir = path.join(root, 'spend');
  process.env.CASTLE_SPEND_DIR = spendDir;
  const printed = [];
  common.setOutput((t) => printed.push(t));
  common.writeResult('x.mp3', 0.06, 1, 'm', 'song');
  common.setOutput((t) => process.stdout.write(t + '\n'));
  delete process.env.CASTLE_SPEND_DIR;
  const spendFile = path.join(spendDir, 'suno-spend.jsonl');
  const lines = exists(spendFile) ? fs.readFileSync(spendFile, 'utf8').split('\n').filter((l) => l.trim()) : [];
  const entry = lines.length === 1 ? parse(lines[0]) : null;
  report(entry && entry.usd === 0.06 && entry.kind === 'song' && /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/.test(entry.at), 'common: a real result writes one spend line for its provider', lines.join('\n'));
  report(printed.join('\n') === 'result:\n  saved_path: x.mp3\n  cost_estimate_usd: 0.06\n  elapsed_seconds: 1\n  model: m', 'common: the result block reads as before', printed.join('\n'));
  report(common.roundEven(2.5) === 2 && common.roundEven(3.5) === 4 && common.roundEven(0.04, 4) === 0.04 && common.slug('Castle Light!') === 'castle-light', 'common: rounding and slugs match PowerShell');
}

// 9) The real request paths against a local mock server (no paid API is ever reached).
const png = Buffer.from('89504e470d0a1a0a0000000d49484452', 'hex');
const seen = [];
let videoPolls = 0;
const server = http.createServer(async (req, res) => {
  const chunks = [];
  for await (const ch of req) chunks.push(ch);
  const body = Buffer.concat(chunks);
  const url = new URL(req.url, 'http://x');
  seen.push({ method: req.method, path: url.pathname, query: url.search, headers: req.headers, body });
  const json = (code, obj) => {
    res.writeHead(code, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(obj));
  };
  const p = url.pathname;
  if (p === '/api.openai.com/v1/images/generations') {
    const j = parse(body.toString('utf8')) || {};
    if (/please fail/.test(j.prompt)) return json(401, { error: { message: `Incorrect API key provided: ${fakeKey}. Bearer ${fakeKey}` } });
    return json(200, { data: [{ b64_json: png.toString('base64') }] });
  }
  if (p === '/api.openai.com/v1/images/edits') return json(200, { data: [{ b64_json: png.toString('base64') }] });
  if (p === '/ark.ap-southeast.bytepluses.com/api/v3/contents/generations/tasks') return json(200, { id: 'cgt-1' });
  if (p === '/ark.ap-southeast.bytepluses.com/api/v3/contents/generations/tasks/cgt-1') {
    videoPolls++;
    return json(200, videoPolls < 2 ? { status: 'running' } : { status: 'succeeded', content: { video_url: 'https://cdn.example.test/clip.mp4' } });
  }
  if (p === '/api.sunoapi.org/api/v1/generate') return json(200, { code: 200, msg: 'success', data: { taskId: 's-1' } });
  if (p === '/api.sunoapi.org/api/v1/generate/record-info') {
    return json(200, { data: { status: 'SUCCESS', response: { sunoData: [{ audioUrl: 'https://cdn.example.test/a.mp3', imageUrl: 'https://cdn.example.test/a.jpg' }, { audioUrl: 'https://cdn.example.test/b.mp3', imageUrl: 'https://cdn.example.test/gone.jpg' }] } } });
  }
  if (p.startsWith('/api.elevenlabs.io/v1/text-to-speech/')) {
    if (p.endsWith('/bad-voice')) return json(422, { detail: { message: 'voice not found' } });
    res.writeHead(200, { 'Content-Type': 'audio/mpeg' });
    return res.end(Buffer.from('ID3-fake-audio'));
  }
  if (p === '/cdn.example.test/gone.jpg') return json(404, { message: 'no such cover' });
  if (p.startsWith('/cdn.example.test/')) {
    res.writeHead(200);
    return res.end(Buffer.from('bytes of ' + p));
  }
  json(404, { message: 'unknown mock route ' + p });
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const mockBase = `http://127.0.0.1:${server.address().port}`;
const preload = path.join(root, 'mock-net.mjs');
fs.writeFileSync(
  preload,
  `const base = ${JSON.stringify(mockBase)};
const realFetch = globalThis.fetch;
globalThis.fetch = (input, init) => {
  const u = new URL(typeof input === 'string' || input instanceof URL ? input : input.url);
  if (u.protocol !== 'https:') throw new Error('mock: only https is expected, got ' + u.href);
  return realFetch(base + '/' + u.host + u.pathname + u.search, init);
};
const realSetTimeout = globalThis.setTimeout;
globalThis.setTimeout = (fn, ms, ...rest) => realSetTimeout(fn, Math.min(ms || 0, 20), ...rest);
`,
  'utf8',
);
const mockSpend = path.join(root, 'mock-spend');
const mock = (script, args, env = {}) => run(script, args, { CASTLE_NO_NETWORK: null, CASTLE_SPEND_DIR: mockSpend, ...env }, { nodeArgs: ['--import', pathToFileURL(preload).href] });
const resultOk = (r, file) => r.code === 0 && r.out.includes(`  saved_path: ${file}`) && /cost_estimate_usd: \d/.test(r.out) && /elapsed_seconds: \d/.test(r.out);
const spendLines = (provider) => {
  const f = path.join(mockSpend, `${provider}-spend.jsonl`);
  return exists(f) ? fs.readFileSync(f, 'utf8').split('\n').filter((l) => l.trim()).length : 0;
};
try {
  // image: new picture
  const briefFile = path.join(root, 'brief.txt');
  fs.writeFileSync(briefFile, 'a "quoted" lantern ✿ in the rain', 'utf8');
  const pic = path.join(out, 'mock', 'pic.png');
  let r = await mock('castle-image.mjs', ['-Prompt', briefFile, '-OutFile', pic, '-Transparent'], { OPENAI_API_KEY: fakeKey });
  let call = seen.findLast((s) => s.path.endsWith('/images/generations'));
  let sent = call ? parse(call.body.toString('utf8')) : null;
  report(
    resultOk(r, pic) && exists(pic) && fs.readFileSync(pic).equals(png) && call.headers.authorization === `Bearer ${fakeKey}` &&
      sent.prompt === 'a "quoted" lantern ✿ in the rain' && sent.model === 'gpt-image-2' && sent.background === 'transparent' && sent.n === 1,
    'mock image: the prompt file goes out as UTF-8 JSON, the picture is saved and the result block printed',
    `exit ${r.code}: ${r.out}`,
  );
  report(spendLines('openai') === 1, 'mock image: one spend line for openai');

  // image: an API error is one Error line with the key hidden
  r = await mock('castle-image.mjs', ['-Prompt', 'please fail', '-OutFile', path.join(out, 'mock', 'fail.png')], { OPENAI_API_KEY: fakeKey });
  report(r.code === 1 && /^Error: the picture request failed: HTTP 401 - Incorrect API key provided: \[hidden\]/m.test(r.out) && !r.out.includes(fakeKey), 'mock image: an API refusal exits 1 with one Error line, key hidden', `exit ${r.code}: ${r.out}`);

  // image: edit with a reference (multipart)
  const ref = path.join(root, 'portrait.png');
  const edited = path.join(out, 'mock', 'edited.png');
  r = await mock('castle-image.mjs', ['--prompt', 'restyle it', '--out-file', edited, '--reference', ref, '--quality', 'low'], { OPENAI_API_KEY: fakeKey });
  call = seen.findLast((s) => s.path.endsWith('/images/edits'));
  const form = call ? call.body.toString('latin1') : '';
  report(
    resultOk(r, edited) && /multipart\/form-data; boundary=/.test(call.headers['content-type']) && /name="image"; filename="portrait.png"/.test(form) &&
      /Content-Type: image\/png/.test(form) && /name="prompt"\r\n\r\nrestyle it/.test(form) && /name="quality"\r\n\r\nlow/.test(form),
    'mock image: a reference goes out as a multipart edit',
    `exit ${r.code}: ${r.out}`,
  );

  // image: two references (a STUDIO workflow with two characters) go out as image[], in order
  const ref2 = path.join(out, 'mock', 'second.png');
  fs.writeFileSync(ref2, png);
  const both = path.join(out, 'mock', 'both.png');
  r = await mock('castle-image.mjs', ['--prompt', 'the two of them', '--out-file', both, '--reference', ref, '--reference', ref2, '--size', '1536x1024'], { OPENAI_API_KEY: fakeKey });
  call = seen.findLast((s) => s.path.endsWith('/images/edits'));
  const form2 = call ? call.body.toString('latin1') : '';
  const first = form2.indexOf('name="image[]"; filename="portrait.png"');
  const second = form2.indexOf('name="image[]"; filename="second.png"');
  report(
    resultOk(r, both) && first !== -1 && second > first && !/name="image"; /.test(form2) && /name="size"\r\n\r\n1536x1024/.test(form2),
    'mock image: two references go out as image[] in the order given',
    `exit ${r.code}: ${r.out}`,
  );
  r = await mock('castle-image.mjs', ['--prompt', 'crowd', '--out-file', both, ...Array(6).fill(['--reference', ref]).flat()], { OPENAI_API_KEY: fakeKey });
  report(r.code === 2 && /at most 5 times/.test(r.out), 'mock image: a sixth reference is refused', `exit ${r.code}: ${r.out}`);

  // look -> image in one process, with the key
  const lookPic = path.join(out, 'mock', 'look.png');
  r = await mock('castle-look.mjs', ['-Kind', 'tryon', '-Words', full, '-OutFile', lookPic], { OPENAI_API_KEY: fakeKey });
  call = seen.findLast((s) => s.path.endsWith('/images/generations'));
  sent = call ? parse(call.body.toString('utf8')) : null;
  report(resultOk(r, lookPic) && sent && /Winter coat/.test(sent.prompt) && sent.size === '1024x1536', 'mock look: the filled prompt is drawn at the style size', `exit ${r.code}: ${r.out}`);

  // video: create, poll twice, download
  const clip = path.join(out, 'mock', 'clip.mp4');
  r = await mock('castle-video.mjs', ['-Prompt', 'waves at dusk', '-OutFile', clip, '-Seconds', '3', '-Resolution', '480p'], { ARK_API_KEY: fakeKey });
  call = seen.find((s) => s.method === 'POST' && s.path.endsWith('/generations/tasks'));
  sent = call ? parse(call.body.toString('utf8')) : null;
  report(
    resultOk(r, clip) && /Video task cgt-1 started; checking every 10 s for up to 10 min\./.test(r.out) && videoPolls === 2 && /cost_estimate_usd: 0\.21/.test(r.out) &&
      fs.readFileSync(clip, 'utf8') === 'bytes of /cdn.example.test/clip.mp4' && sent.duration === 3 && sent.content[0].text === 'waves at dusk',
    'mock video: the task is created, polled until it succeeds and the clip downloaded',
    `exit ${r.code}: ${r.out}`,
  );

  // song: generate, poll, two takes, one cover missing
  const songs = path.join(out, 'mock', 'songs');
  r = await mock('castle-song.mjs', ['-Title', 'Castle Light', '-Style', 'soft piano', '-Lyrics', lyricsFile, '-OutFolder', songs], { SUNOAPI_ORG_KEY: fakeKey });
  call = seen.find((s) => s.path.endsWith('/api/v1/generate'));
  sent = call ? parse(call.body.toString('utf8')) : null;
  const info = seen.find((s) => s.path.endsWith('/record-info'));
  report(
    resultOk(r, path.join(songs, '01-castle-light.mp3')) && r.out.includes(`saved_path: ${path.join(songs, '01-castle-light.jpg')}`) && r.out.includes(`saved_path: ${path.join(songs, '02-castle-light.mp3')}`) &&
      /Cover for take 02 was not saved: HTTP 404 - no such cover/.test(r.out) && !exists(path.join(songs, '02-castle-light.jpg')) &&
      sent.prompt.includes('Under the castle light') && sent.customMode === true && sent.instrumental === false && info.query === '?taskId=s-1',
    'mock song: every take and cover is saved; a missing cover is one line, not a failure',
    `exit ${r.code}: ${r.out}`,
  );

  // voice: success, and a refusal leaves no .mp3 behind
  const voice = path.join(out, 'mock', 'line.mp3');
  r = await mock('castle-voice.mjs', ['-Text', 'Good evening.', '-OutFile', voice], { ELEVENLABS_API_KEY: fakeKey });
  call = seen.findLast((s) => s.path.includes('/text-to-speech/'));
  report(resultOk(r, voice) && fs.readFileSync(voice, 'utf8') === 'ID3-fake-audio' && call.headers['xi-api-key'] === fakeKey && call.path.endsWith('/21m00Tcm4TlvDq8ikWAM'), 'mock voice: the line is spoken and saved', `exit ${r.code}: ${r.out}`);
  const badVoice = path.join(out, 'mock', 'bad.mp3');
  r = await mock('castle-voice.mjs', ['-Text', 'Hello there.', '-OutFile', badVoice, '-VoiceId', 'bad-voice'], { ELEVENLABS_API_KEY: fakeKey });
  report(r.code === 1 && /Error: the voice request failed: HTTP 422 - voice not found/.test(r.out) && !exists(badVoice), 'mock voice: a refusal exits 1 and leaves no file', `exit ${r.code}: ${r.out}`);
  report(spendLines('seedance') === 1 && spendLines('suno') === 1 && spendLines('elevenlabs') === 1 && spendLines('openai') === 4, 'mock: one spend line per real render, none for failures');
} finally {
  server.close();
}

console.log('');
try {
  fs.rmSync(root, { recursive: true, force: true });
} catch {
  // a temp folder left behind is harmless
}
if (failures > 0) {
  console.log(`${failures} check(s) FAILED`);
  process.exit(1);
}
console.log('All checks passed.');
process.exit(0);
