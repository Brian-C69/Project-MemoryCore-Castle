// castle-world.mjs - prepare a world's model pieces for Castle.
//
//   node castle-world.mjs --prepare --manifest <world.json> --in <folder of exports> --out <pieces folder>
//   (the PowerShell names -Prepare -Manifest -In -Out work too)
//
// The member makes each piece in Tripo (or any 3D tool) and exports a GLB named like its
// picture's number ("01-....glb" or just "01.glb"). This makes every piece light enough for the
// world windows: shapes simplified to a budget (the base 60k triangles, every other piece 20k),
// textures turned to webp at most 1024 px, everything meshopt-compressed - with the free
// gltf-transform tool through npx, which comes with Node.
//
// Exit codes:
//   0  every piece prepared
//   1  the tool failed on a piece (one line says which and why)
//   2  bad input (no manifest, or pieces missing from the exports folder - listed)
//   3  npx (Node's package runner) was not found - a notice was left for Castle
//   9  CASTLE_NO_NETWORK=1 blocked the npx run (tests only)

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import * as c from './castle-common.mjs';

const SPEC = {
  params: {
    Prepare: { type: 'switch' },
    Manifest: { type: 'string', mandatory: true },
    In: { type: 'string', mandatory: true },
    Out: { type: 'string', mandatory: true },
  },
  positional: ['Manifest', 'In', 'Out'],
};

const GLTF_TRANSFORM = '@gltf-transform/cli@4';
const BUDGETS = { base: 60000, default: 20000 };
const MB = 1024 * 1024;

/** Counts the triangles a GLB draws, from its JSON chunk alone. */
export function glbTriangles(file) {
  const bytes = fs.readFileSync(file);
  if (bytes.length < 20 || bytes.readUInt32LE(0) !== 0x46546c67) return 0;
  const jsonLength = bytes.readUInt32LE(12);
  const gltf = JSON.parse(bytes.toString('utf8', 20, 20 + jsonLength));
  const accessors = gltf.accessors || [];
  let total = 0;
  for (const mesh of [].concat(gltf.meshes || [])) {
    for (const p of [].concat(mesh.primitives || [])) {
      if (p.mode !== undefined && p.mode !== null && p.mode !== 4) continue;
      if (p.indices !== undefined && p.indices !== null) total += Number(accessors[p.indices].count) / 3;
      else if (p.attributes && p.attributes.POSITION !== undefined && p.attributes.POSITION !== null) total += Number(accessors[p.attributes.POSITION].count) / 3;
    }
  }
  return c.roundEven(total);
}

/** npx on PATH, else beside the Node running this script (Castle may start Node by its full path). */
function findNpx() {
  const names = process.platform === 'win32' ? ['npx.cmd', 'npx.exe', 'npx'] : ['npx'];
  const dirs = (process.env.PATH || process.env.Path || '').split(path.delimiter).filter(Boolean);
  dirs.push(path.dirname(process.execPath));
  for (const dir of dirs) {
    for (const name of names) {
      const candidate = path.join(dir, name);
      if (c.isFile(candidate)) return candidate;
    }
  }
  return null;
}

/** Runs npx with its arguments; a .cmd goes through cmd.exe (Node will not start one directly). */
function runNpx(npx, args) {
  let result;
  if (process.platform === 'win32' && /\.cmd$/i.test(npx)) {
    const line = [npx, ...args].map((x) => `"${x}"`).join(' ');
    result = spawnSync(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', `"${line}"`], { windowsVerbatimArguments: true, windowsHide: true, encoding: 'utf8' });
  } else {
    result = spawnSync(npx, args, { windowsHide: true, encoding: 'utf8' });
  }
  const output = (result.stdout || '') + (result.stderr || '') + (result.error ? result.error.message : '');
  return { code: result.status === null ? 1 : result.status, output };
}

const mb = (n) => (n / MB).toLocaleString('en-US', { minimumFractionDigits: 1, maximumFractionDigits: 1 });

async function main(argv) {
  const a = c.parseArgs(argv, SPEC);
  if (!a.Prepare) c.badInput('Say -Prepare to prepare the pieces.');
  if (!c.isFile(a.Manifest)) c.badInput(`No world.json at ${a.Manifest}.`);
  if (!c.isDir(a.In)) c.badInput(`No exports folder at ${a.In}.`);

  let world;
  try {
    world = JSON.parse(c.readUtf8(path.resolve(a.Manifest)));
  } catch {
    c.badInput(`The world.json at ${a.Manifest} is not valid JSON.`);
  }
  const pieces = [].concat((world && world.pieces) || []).filter((p) => p && /^\d+/.test(String(p.file ?? '')));
  if (!pieces.length) c.badInput('The world.json lists no numbered pieces.');

  // every piece needs its export: matched by its number
  const exports = fs
    .readdirSync(a.In, { withFileTypes: true })
    .filter((e) => e.isFile() && /\.glb$/i.test(e.name))
    .map((e) => e.name)
    .sort((x, y) => x.toLowerCase().localeCompare(y.toLowerCase()));
  const plan = [];
  const missing = [];
  for (const piece of pieces) {
    const number = parseInt(/^\d+/.exec(piece.file)[0], 10);
    const found = exports.find((name) => {
      const m = /^(\d+)(\D|$)/.exec(path.basename(name, path.extname(name)));
      return m && parseInt(m[1], 10) === number;
    });
    if (found) plan.push({ piece, source: path.resolve(a.In, found) });
    else missing.push(piece.file);
  }
  if (missing.length) c.badInput('These pieces are not in the exports folder yet: ' + missing.join(', '));

  // the tool comes with Node
  const npx = findNpx();
  if (!npx) {
    c.writeNotice({
      at: c.now(),
      kind: 'world',
      file: a.In,
      reason: 'Node is not installed, so the world pieces could not be prepared. Install Node from nodejs.org, then press Prepare pieces again.',
      settings: '',
    });
    c.say('Node is not installed (npx was not found), so the pieces could not be prepared. Install Node from nodejs.org and try again.');
    c.exit(3);
  }
  try {
    c.assertNetworkAllowed('npx (the gltf-transform tool may be downloaded once)');
  } catch (error) {
    c.fail('Preparing the pieces', { error });
  }

  fs.mkdirSync(a.Out, { recursive: true });
  let done = 0;
  for (const step of plan) {
    const { piece } = step;
    const target = path.join(a.Out, piece.file);
    const budget = piece.role === 'base' ? BUDGETS.base : BUDGETS.default;
    const before = glbTrianglesSafe(step.source);
    const ratio = before > budget ? c.roundEven(budget / before, 4) : 1;
    const args = ['--yes', GLTF_TRANSFORM, 'optimize', step.source, target, '--compress', 'meshopt', '--texture-compress', 'webp', '--texture-size', '1024'];
    if (ratio < 1) args.push('--simplify', 'true', '--simplify-ratio', c.num(ratio), '--simplify-error', '0.01');
    else args.push('--simplify', 'false');
    const sizeBefore = fs.statSync(step.source).size;
    const { code, output } = runNpx(npx, args);
    if (code !== 0 || !fs.existsSync(target)) {
      const why = output.split(/\r?\n/).filter((l) => /\S/.test(l)).slice(-2).join(' ');
      c.fail(`Preparing ${piece.file}`, { message: why });
    }
    const after = glbTrianglesSafe(target);
    const sizeAfter = fs.statSync(target).size;
    done++;
    c.say(`piece ${piece.file}: ${mb(sizeBefore)} MB -> ${mb(sizeAfter)} MB, ${before} -> ${after} triangles (${done}/${plan.length})`);
  }
  c.say(`result: prepared ${done} pieces into ${a.Out}`);
  c.exit(0);
}

/** A GLB whose JSON cannot be read counts as 0 triangles (it is still handed to the tool). */
function glbTrianglesSafe(file) {
  try {
    return glbTriangles(file);
  } catch {
    return 0;
  }
}

if (c.isMain(import.meta.url)) c.runMain(main);
