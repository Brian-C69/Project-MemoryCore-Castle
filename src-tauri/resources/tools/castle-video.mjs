// castle-video.mjs - make one short video clip for the companion.
//
//   node castle-video.mjs --prompt "<text, or a UTF-8 file holding it>" --out-file <path.mp4>
//                         [--seconds 5] [--resolution 480p|720p|1080p]
//                         [--what-if-cost]             # print the estimate, no request
//   (the PowerShell names -Prompt, -OutFile, -Seconds, -WhatIfCost ... work too)
//
// Uses BytePlus ModelArk Seedance with ARK_API_KEY: creates a generation task, checks it every
// 10 seconds for up to 10 minutes, then downloads the clip.
// No key: the prompt is saved as <name>.prompt.md and the script exits 3.

import path from 'node:path';
import * as c from './castle-common.mjs';

const SPEC = {
  params: {
    Prompt: { type: 'string', mandatory: true },
    OutFile: { type: 'string', mandatory: true },
    Seconds: { type: 'int', min: 1, max: 15, default: 5 },
    Resolution: { type: 'string', set: ['480p', '720p', '1080p'], default: '720p' },
    WhatIfCost: { type: 'switch' },
  },
  positional: ['Prompt', 'OutFile', 'Seconds', 'Resolution'],
};

const MODEL = 'dreamina-seedance-2-0-260128';
const BASE_URL = 'https://ark.ap-southeast.bytepluses.com/api/v3/contents/generations/tasks';
const POLL_SECONDS = 10;
const TIMEOUT_SECONDS = 600;

// ModelArk bills video by tokens: roughly seconds x width x height x 24 / 1024. Tokens per second
// and USD per million tokens below are ROUGH ESTIMATES and depend on the member's plan or
// resource pack; the BytePlus console is the truth.
const TOKENS_PER_SECOND = { '480p': 10000, '720p': 22000, '1080p': 49000 };
const USD_PER_MILLION = 7.0;

async function main(argv) {
  const a = c.parseArgs(argv, SPEC);
  const tokens = TOKENS_PER_SECOND[a.Resolution] * a.Seconds;
  const cost = (tokens / 1000000.0) * USD_PER_MILLION;
  let fullOut = path.resolve(a.OutFile);

  if (a.WhatIfCost) {
    c.writeEstimate(cost, MODEL, `${a.Seconds} s at ${a.Resolution}, about ${tokens} tokens at USD ${c.num(USD_PER_MILLION)} per million`);
    c.exit(0);
  }

  const apiKey = c.getCastleKey('ARK_API_KEY');
  if (!apiKey) {
    const content = `# Video prompt\r\n\r\n${a.Prompt}\r\n\r\n- seconds: ${a.Seconds}\r\n- resolution: ${a.Resolution}\r\n- model: ${MODEL}`;
    c.writeFallback('video', fullOut, content, 'no Seedance (BytePlus ARK) key is set for video');
  }

  fullOut = c.folderFor(fullOut);
  const headers = { Authorization: `Bearer ${apiKey}` };
  const start = Date.now();

  // 1) Create the task.
  let created;
  try {
    created = await c.requestJson(BASE_URL, {
      method: 'POST',
      headers: { ...headers, 'Content-Type': 'application/json; charset=utf-8' },
      body: c.jsonBytes({ model: MODEL, content: [{ type: 'text', text: a.Prompt }], duration: a.Seconds, resolution: a.Resolution }),
      timeoutSec: 120,
    });
  } catch (error) {
    c.fail('creating the video task', { error });
  }
  const taskId = created.id;
  if (!taskId) c.fail('creating the video task', { message: 'the answer held no task id' });
  c.say(`Video task ${taskId} started; checking every ${POLL_SECONDS} s for up to ${Math.trunc(TIMEOUT_SECONDS / 60)} min.`);

  // 2) Poll until succeeded or failed.
  let videoUrl = null;
  let status = '';
  for (;;) {
    await c.sleep(POLL_SECONDS * 1000);
    if ((Date.now() - start) / 1000 > TIMEOUT_SECONDS) {
      c.fail('the video task', { message: `still '${status}' after ${Math.trunc(TIMEOUT_SECONDS / 60)} minutes (task ${taskId}); it may finish later on BytePlus` });
    }
    let task;
    try {
      task = await c.requestJson(`${BASE_URL}/${taskId}`, { headers, timeoutSec: 60 });
    } catch (error) {
      c.fail('checking the video task', { error });
    }
    status = String(task.status ?? '').toLowerCase();
    if (status === 'succeeded') {
      if (task.content && task.content.video_url) videoUrl = String(task.content.video_url);
      if (!videoUrl) c.fail('the video task', { message: `succeeded but held no content.video_url (task ${taskId})` });
      break;
    }
    if (['failed', 'cancelled', 'canceled', 'expired'].includes(status)) {
      let why = '(no detail given)';
      if (task.error) why = task.error.message ? String(task.error.message) : JSON.stringify(task.error);
      c.fail('the video task', { message: `${status} (task ${taskId}): ${why}` });
    }
    // queued / running: keep waiting
  }

  // 3) Download the clip.
  try {
    await c.download(videoUrl, fullOut, 300);
  } catch (error) {
    c.fail('downloading the video', { error });
  }

  c.writeResult(fullOut, cost, (Date.now() - start) / 1000, MODEL, 'video');
  c.exit(0);
}

if (c.isMain(import.meta.url)) c.runMain(main);
