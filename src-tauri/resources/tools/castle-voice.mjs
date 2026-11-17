// castle-voice.mjs - speak one line aloud for the companion.
//
//   node castle-voice.mjs --text "<line, or a path to a UTF-8 text file>" --out-file <path.mp3>
//                         [--voice-id <ElevenLabs voice id>]
//                         [--what-if-cost]             # print the estimate, no request
//   (the PowerShell names -Text, -OutFile, -VoiceId, -WhatIfCost work too)
//
// Uses ElevenLabs text-to-speech (eleven_multilingual_v2) with ELEVENLABS_API_KEY.
// No key: the line is saved as <name>.line.txt and the script exits 3.

import fs from 'node:fs';
import path from 'node:path';
import * as c from './castle-common.mjs';

const SPEC = {
  params: {
    Text: { type: 'string', mandatory: true },
    OutFile: { type: 'string', mandatory: true },
    VoiceId: { type: 'string', default: '21m00Tcm4TlvDq8ikWAM' },
    WhatIfCost: { type: 'switch' },
  },
  positional: ['Text', 'OutFile', 'VoiceId'],
};

const MODEL = 'eleven_multilingual_v2';

// ROUGH ESTIMATE: ElevenLabs bills by characters; on common plans this model works out near
// USD 0.20 per 1000 characters. The member's plan decides.
const USD_PER_1000 = 0.2;

async function main(argv) {
  const a = c.parseArgs(argv, SPEC);
  const line = c.resolveText(a.Text);
  if (!line.trim()) c.badInput('the text to speak is empty');
  if (!a.VoiceId.trim()) c.badInput('the voice id is empty');

  const cost = (line.length / 1000.0) * USD_PER_1000;
  let fullOut = path.resolve(a.OutFile);

  if (a.WhatIfCost) {
    c.writeEstimate(cost, MODEL, `${line.length} characters at about USD ${c.num(USD_PER_1000)} per 1000`);
    c.exit(0);
  }

  const apiKey = c.getCastleKey('ELEVENLABS_API_KEY');
  if (!apiKey) c.writeFallback('voice', fullOut, line, 'no ElevenLabs key is set for voice');

  fullOut = c.folderFor(fullOut);
  const start = Date.now();
  const url = 'https://api.elevenlabs.io/v1/text-to-speech/' + encodeURIComponent(a.VoiceId);
  const existedBefore = fs.existsSync(fullOut);
  try {
    const { bytes } = await c.request(url, {
      method: 'POST',
      headers: { 'xi-api-key': apiKey, Accept: 'audio/mpeg', 'Content-Type': 'application/json; charset=utf-8' },
      body: c.jsonBytes({ text: line, model_id: MODEL }),
      timeoutSec: 180,
      as: 'bytes',
    });
    fs.writeFileSync(fullOut, bytes);
  } catch (error) {
    if (!existedBefore && fs.existsSync(fullOut)) {
      // A failed request must not leave a half-written or error-body .mp3 behind.
      try {
        fs.rmSync(fullOut, { force: true });
      } catch {
        // nothing more to do
      }
    }
    c.fail('the voice request', { error });
  }

  c.writeResult(fullOut, cost, (Date.now() - start) / 1000, MODEL, 'voice');
  c.exit(0);
}

if (c.isMain(import.meta.url)) c.runMain(main);
