// castle-image.mjs - draw one picture for the companion.
//
//   node castle-image.mjs --prompt "<text, or a UTF-8 file holding it>" --out-file <path.png>
//                         [--size 1024x1024|1024x1536|1536x1024]
//                         [--quality low|medium|high]
//                         [--reference <image path>]   # edit / restyle from a picture
//                         [--transparent]              # a see-through background (PNG)
//                         [--what-if-cost]             # print the estimate, no request
//   (the PowerShell names -Prompt, -OutFile, -WhatIfCost ... work too)
//
// Uses OpenAI gpt-image-2 with OPENAI_API_KEY (Castle passes that key to Claude Code only).
// When CASTLE_IMAGE=codex, Codex draws with its own image tool instead and this script exits 4
// without writing anything. No key: the prompt is saved as <name>.prompt.md and it exits 3.

import fs from 'node:fs';
import path from 'node:path';
import * as c from './castle-common.mjs';

export const SPEC = {
  params: {
    Prompt: { type: 'string', mandatory: true },
    OutFile: { type: 'string', mandatory: true },
    Size: { type: 'string', set: ['1024x1024', '1024x1536', '1536x1024'], default: '1024x1024' },
    Quality: { type: 'string', set: ['low', 'medium', 'high'], default: 'medium' },
    Reference: { type: 'string' },
    Transparent: { type: 'switch' },
    WhatIfCost: { type: 'switch' },
  },
  positional: ['Prompt', 'OutFile', 'Size', 'Quality', 'Reference'],
};

const MODEL = 'gpt-image-2';

// Rough USD per image by size and quality. These are ESTIMATES taken from public price lists
// and may drift; a --reference edit also bills the input image tokens, which this table does
// not include. The provider bill wins.
const COST = {
  '1024x1024_low': 0.02,
  '1024x1024_medium': 0.04,
  '1024x1024_high': 0.19,
  '1024x1536_low': 0.03,
  '1024x1536_medium': 0.06,
  '1024x1536_high': 0.25,
  '1536x1024_low': 0.03,
  '1536x1024_medium': 0.06,
  '1536x1024_high': 0.25,
};

const MIME = { '.png': 'image/png', '.webp': 'image/webp' };

/** Draws with already-parsed arguments (castle-look calls this directly). */
export async function draw(a) {
  // the prompt itself, or a UTF-8 file holding it (Castle passes a file, so no quote is lost)
  const prompt = c.resolveText(a.Prompt);
  const cost = COST[`${a.Size}_${a.Quality}`];
  let fullOut = path.resolve(a.OutFile);

  if (process.env.CASTLE_IMAGE === 'codex') {
    c.say(`Codex draws pictures with its own image tool - use it and save to ${fullOut}`);
    c.exit(4);
  }

  if (a.WhatIfCost) {
    const mode = a.Reference ? 'edit with a reference picture (input image tokens extra)' : 'new picture';
    c.writeEstimate(cost, MODEL, `${a.Size}, ${a.Quality} quality, ${mode}`);
    c.exit(0);
  }

  if (a.Reference && !c.isFile(a.Reference)) c.badInput(`reference picture not found: ${a.Reference}`);

  const apiKey = c.getCastleKey('OPENAI_API_KEY');
  if (!apiKey) {
    let content = `# Picture prompt\r\n\r\n${prompt}\r\n\r\n- size: ${a.Size}\r\n- quality: ${a.Quality}`;
    if (a.Reference) content += `\r\n- reference: ${a.Reference}`;
    c.writeFallback('image', fullOut, content, 'no OpenAI key is set for pictures');
  }

  fullOut = c.folderFor(fullOut);
  const start = Date.now();
  let resp;

  if (!a.Reference) {
    // New picture: JSON body. gpt-image models always answer with b64_json, so no
    // response_format is sent (the endpoint rejects it for these models).
    const url = 'https://api.openai.com/v1/images/generations';
    const body = { model: MODEL, prompt, size: a.Size, quality: a.Quality, n: 1 };
    if (a.Transparent) body.background = 'transparent';
    try {
      resp = await c.requestJson(url, {
        method: 'POST',
        headers: { Authorization: `Bearer ${apiKey}`, 'Content-Type': 'application/json; charset=utf-8' },
        body: c.jsonBytes(body),
        timeoutSec: 300,
      });
    } catch (error) {
      c.fail('the picture request', { error });
    }
  } else {
    // Edit from a reference picture: multipart form.
    const url = 'https://api.openai.com/v1/images/edits';
    try {
      const refPath = path.resolve(a.Reference);
      const mime = MIME[path.extname(refPath).toLowerCase()] || 'image/jpeg';
      const form = new FormData();
      c.assertNetworkAllowed(url);
      form.append('image', new Blob([fs.readFileSync(refPath)], { type: mime }), path.basename(refPath));
      form.append('prompt', prompt);
      form.append('model', MODEL);
      form.append('size', a.Size);
      form.append('quality', a.Quality);
      form.append('n', '1');
      if (a.Transparent) form.append('background', 'transparent');
      resp = await c.requestJson(url, {
        method: 'POST',
        headers: { Authorization: `Bearer ${apiKey}` },
        body: form,
        timeoutSec: 300,
      });
    } catch (error) {
      if (error instanceof c.HttpError) {
        let detail = error.body;
        try {
          const j = JSON.parse(error.body);
          if (j && j.error && j.error.message) detail = j.error.message;
        } catch {
          // the body as it came
        }
        c.fail('the picture edit request', { message: `HTTP ${error.status} - ` + String(detail).replace(/\s+/g, ' ').trim() });
      }
      c.fail('the picture edit request', { error });
    }
  }

  const first = Array.isArray(resp.data) ? resp.data[0] : resp.data;
  if (!first || !first.b64_json) {
    c.fail('the picture request', { message: 'the answer held no picture data (b64_json missing)' });
  }
  try {
    fs.writeFileSync(fullOut, Buffer.from(first.b64_json, 'base64'));
  } catch (error) {
    c.fail('saving the picture', { error });
  }

  c.writeResult(fullOut, cost, (Date.now() - start) / 1000, MODEL, 'image');
  c.exit(0);
}

export async function main(argv) {
  await draw(c.parseArgs(argv, SPEC));
}

if (c.isMain(import.meta.url)) c.runMain(main);
