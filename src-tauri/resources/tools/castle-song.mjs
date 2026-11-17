// castle-song.mjs - sing one song for the companion.
//
//   node castle-song.mjs --title "<title>" --style "<style tags>"
//                        --lyrics "<lyrics text, or a path to a .md/.txt file>"
//                        --out-folder <folder>
//                        [--instrumental] [--model V5]
//                        [--what-if-cost]             # print the estimate, no request
//   (the PowerShell names -Title, -Style, -Lyrics, -OutFolder, -WhatIfCost ... work too)
//
// Uses sunoapi.org with SUNOAPI_ORG_KEY: starts a generation, checks it every 10 seconds for up
// to 10 minutes, then saves each take as <NN>-<slug>.mp3 (plus <NN>-<slug>.jpg cover art when
// the service returns one).
// No key: the lyrics and style are saved as <slug>.lyrics.md and the script exits 3.

import fs from 'node:fs';
import path from 'node:path';
import * as c from './castle-common.mjs';

const SPEC = {
  params: {
    Title: { type: 'string', mandatory: true },
    Style: { type: 'string', mandatory: true },
    Lyrics: { type: 'string', default: '' },
    OutFolder: { type: 'string', mandatory: true },
    Instrumental: { type: 'switch' },
    Model: { type: 'string', default: 'V5' },
    WhatIfCost: { type: 'switch' },
  },
  positional: ['Title', 'Style', 'Lyrics', 'OutFolder', 'Model'],
};

const API_BASE = 'https://api.sunoapi.org/api/v1';
// The service requires a callback URL; nothing listens there because this script polls.
const CALLBACK_URL = 'https://example.com/castle-callback';
const POLL_SECONDS = 10;
const TIMEOUT_SECONDS = 600;

// ROUGH ESTIMATE: one generation (usually two takes) costs a few credits on sunoapi.org, around
// USD 0.06 at common credit prices. Check the member's sunoapi.org balance for the real figure.
const COST = 0.06;

async function main(argv) {
  const a = c.parseArgs(argv, SPEC);
  const slug = c.slug(a.Title, 'song');
  const folder = path.resolve(a.OutFolder);

  if (!a.Instrumental && !a.Lyrics.trim()) c.badInput('lyrics are needed unless -Instrumental is set');
  const lyricsText = a.Lyrics.trim() ? c.resolveText(a.Lyrics) : '';

  if (a.WhatIfCost) {
    c.writeEstimate(COST, a.Model, `one generation (${a.Instrumental ? 'instrumental' : 'with vocals'}), usually two takes`);
    c.exit(0);
  }

  const apiKey = c.getCastleKey('SUNOAPI_ORG_KEY');
  if (!apiKey) {
    let content = `# ${a.Title}\r\n\r\n## Style\r\n\r\n${a.Style}\r\n\r\n- model: ${a.Model}\r\n- instrumental: ${a.Instrumental ? 'True' : 'False'}\r\n\r\n## Lyrics\r\n\r\n`;
    content += lyricsText || '(instrumental - no lyrics)';
    c.writeFallback('song', path.join(folder, `${slug}.mp3`), content, 'no sunoapi.org key is set for songs');
  }

  fs.mkdirSync(folder, { recursive: true });
  const headers = { Authorization: `Bearer ${apiKey}` };
  const start = Date.now();

  // 1) Start the generation.
  const req = { customMode: true, instrumental: a.Instrumental, model: a.Model, style: a.Style, title: a.Title, callBackUrl: CALLBACK_URL };
  if (!a.Instrumental) req.prompt = lyricsText;
  let created;
  try {
    created = await c.requestJson(`${API_BASE}/generate`, {
      method: 'POST',
      headers: { ...headers, 'Content-Type': 'application/json; charset=utf-8' },
      body: c.jsonBytes(req),
      timeoutSec: 120,
    });
  } catch (error) {
    c.fail('starting the song', { error });
  }
  if (created.code && parseInt(created.code, 10) !== 200) {
    c.fail('starting the song', { message: `the service said code ${created.code}: ${created.msg ?? ''}` });
  }
  const taskId = created.data ? created.data.taskId : null;
  if (!taskId) c.fail('starting the song', { message: 'the answer held no taskId' });
  c.say(`Song task ${taskId} started; checking every ${POLL_SECONDS} s for up to ${Math.trunc(TIMEOUT_SECONDS / 60)} min.`);

  // 2) Poll until SUCCESS or a failed status.
  let tracks = null;
  let status = '';
  for (;;) {
    await c.sleep(POLL_SECONDS * 1000);
    if ((Date.now() - start) / 1000 > TIMEOUT_SECONDS) {
      c.fail('the song task', { message: `still '${status}' after ${Math.trunc(TIMEOUT_SECONDS / 60)} minutes (task ${taskId}); it may finish later on sunoapi.org` });
    }
    let info;
    try {
      info = await c.requestJson(`${API_BASE}/generate/record-info?taskId=${encodeURIComponent(taskId)}`, { headers, timeoutSec: 60 });
    } catch (error) {
      c.fail('checking the song task', { error });
    }
    const data = info.data || {};
    status = String(data.status ?? '').toUpperCase();
    if (status === 'SUCCESS') {
      const got = data.response ? data.response.sunoData : null;
      tracks = got === null || got === undefined ? [] : [].concat(got);
      if (tracks.length < 1) c.fail('the song task', { message: `SUCCESS but no tracks came back (task ${taskId})` });
      break;
    }
    if (/FAIL|ERROR|EXCEPTION/.test(status)) {
      const why = data.errorMessage ? String(data.errorMessage) : '(no detail given)';
      c.fail('the song task', { message: `${status} (task ${taskId}): ${why}` });
    }
    // PENDING / TEXT_SUCCESS / FIRST_SUCCESS: keep waiting
  }

  // 3) Download every take (and its cover when present).
  const saved = [];
  let n = 0;
  for (const t of tracks) {
    n++;
    const nn = String(n).padStart(2, '0');
    if (!t || !t.audioUrl) {
      c.say(`Take ${nn} had no audioUrl; skipped.`);
      continue;
    }
    const mp3 = path.join(folder, `${nn}-${slug}.mp3`);
    try {
      await c.download(t.audioUrl, mp3, 300);
      saved.push(mp3);
    } catch (error) {
      c.fail(`downloading take ${nn}`, { error });
    }
    if (t.imageUrl) {
      const jpg = path.join(folder, `${nn}-${slug}.jpg`);
      try {
        await c.download(t.imageUrl, jpg, 120);
        saved.push(jpg);
      } catch (error) {
        c.say(`Cover for take ${nn} was not saved: ` + c.errorText(error));
      }
    }
  }
  if (saved.length < 1) c.fail('the song task', { message: `no take had audio to download (task ${taskId})` });

  c.writeResult(saved, COST, (Date.now() - start) / 1000, a.Model, 'song');
  c.exit(0);
}

if (c.isMain(import.meta.url)) c.runMain(main);
