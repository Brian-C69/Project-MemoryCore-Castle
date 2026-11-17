// castle-beat.mjs - write one beat card for Castle's STORY stage.
//
//   node castle-beat.mjs next --world <world-folder>
//   node castle-beat.mjs card --world <world-folder> --json '<card json>' (or a path to a .json file)
//   (the PowerShell names -World -Json work too; the verb defaults to card)
//
// The stage (the visual novel the member sees) reads the newest card in <world>/stage/. A card:
//   {"chapter": "...", "scene": "scenes/03-x.png",
//    "lines": [{"speaker": "", "text": "..."}, {"speaker": "Nova", "text": "...", "voice": "stage/voice/0007-1.mp3"}],
//    "choices": ["...", "...", "(Say something)"]}
// The helper checks the card, numbers it and writes it whole, so the stage never reads half a card.
//
// Exit codes: 0 written (or the number printed), 2 the card was refused (one line says why).

import fs from 'node:fs';
import path from 'node:path';
import * as c from './castle-common.mjs';

const SPEC = {
  params: {
    Verb: { type: 'string', set: ['next', 'card'], default: 'card' },
    World: { type: 'string', mandatory: true },
    Json: { type: 'string' },
  },
  positional: ['Verb'],
};

const TOP_KEYS = ['beat', 'chapter', 'scene', 'lines', 'choices'];
const LINE_KEYS = ['speaker', 'text', 'voice'];

const blank = (v) => v === undefined || v === null || !String(v).trim();
const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);

function worldFolder(name) {
  if (blank(name) || name.includes('..')) c.badInput('Name the world folder.');
  const full = path.join(process.cwd(), name);
  if (!c.isDir(full)) c.badInput(`There is no world folder '${name}' here.`);
  return full;
}

function nextBeat(folder) {
  const stage = path.join(folder, 'stage');
  let max = 0;
  if (c.isDir(stage)) {
    for (const name of fs.readdirSync(stage)) {
      const m = /^beat-(\d+)\.json$/i.exec(name);
      if (m && c.isFile(path.join(stage, name)) && parseInt(m[1], 10) > max) max = parseInt(m[1], 10);
    }
  }
  return max + 1;
}

/** A path inside the world folder, with forward or back slashes, never leaving it. */
function insideWorld(p) {
  return !(blank(p) || p.includes('..') || /^[A-Za-z]:/.test(p) || p.startsWith('/') || p.startsWith('\\'));
}

const truthy = (v) => !(v === undefined || v === null || v === false || v === '' || v === 0 || (Array.isArray(v) && v.length === 0));

async function main(argv) {
  const a = c.parseArgs(argv, SPEC);
  const folder = worldFolder(a.World);
  if (a.Verb === 'next') {
    c.say(String(nextBeat(folder)).padStart(4, '0'));
    c.exit(0);
  }

  if (blank(a.Json)) c.badInput('Give the card as -Json.');
  const text = c.resolveText(a.Json);
  let card;
  try {
    card = JSON.parse(text);
  } catch {
    c.badInput('The card is not valid JSON.');
  }
  if (!isObject(card)) c.badInput('The card must be one JSON object.');

  for (const key of Object.keys(card)) {
    if (!TOP_KEYS.includes(key)) c.badInput(`The card has an unknown key '${key}' (allowed: ${TOP_KEYS.join(', ')}).`);
  }
  const lines = card.lines === undefined || card.lines === null ? [] : [].concat(card.lines);
  if (!truthy(card.lines) || lines.length === 0) c.badInput('The card needs at least one line.');
  for (const line of lines) {
    if (!isObject(line)) c.badInput('Every line needs its text.');
    for (const key of Object.keys(line)) {
      if (!LINE_KEYS.includes(key)) c.badInput(`A line has an unknown key '${key}' (allowed: ${LINE_KEYS.join(', ')}).`);
    }
    if (blank(line.text)) c.badInput('Every line needs its text.');
    if (truthy(line.voice) && !insideWorld(String(line.voice))) c.badInput('A voice must be a file inside the world folder.');
  }
  const choices = card.choices === undefined || card.choices === null ? [] : [].concat(card.choices);
  if (choices.length === 1 || choices.length > 5) c.badInput('Give 2 to 5 choices (or none, only on the very last beat).');
  for (const ch of choices) if (blank(ch)) c.badInput('A choice is empty.');
  if (truthy(card.scene) && !insideWorld(String(card.scene))) c.badInput('The scene must be a picture inside the world folder.');

  const number = nextBeat(folder);
  card.beat = number;
  const stage = path.join(folder, 'stage');
  fs.mkdirSync(stage, { recursive: true });
  const name = `beat-${String(number).padStart(4, '0')}.json`;
  const target = path.join(stage, name);
  const partial = `${target}.partial`;
  c.writeUtf8(partial, JSON.stringify(card, null, 2));
  fs.renameSync(partial, target);
  c.say(`beat ${number} written: stage/${name}`);
  c.exit(0);
}

if (c.isMain(import.meta.url)) c.runMain(main);
