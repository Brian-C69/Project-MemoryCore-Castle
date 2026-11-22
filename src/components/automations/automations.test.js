import { describe, it, expect } from 'vitest'
import { triggerSummary, pill, nextRun, newJob, commandFields, actionSummary, castleAction, toggleDestination, runStatus, messageTrigger, fileTrigger, webhookTrigger, triggerDefaults, listeningText, EVENTS, eventTrigger, noticeLine } from './automations.js'
describe('automation presentation', () => {
 it('explains all schedule types', () => { expect(triggerSummary({ cron: '0 8 * * 1-5' })).toContain('0 8'); expect(triggerSummary({ every_minutes: 30 })).toBe('Every 30 minutes'); expect(triggerSummary({ at: '2026-12-01T09:00' })).toBe('Once: 2026-12-01 09:00') })
 it('keeps unfinished and failed runs distinct', () => { expect(pill('running')).toBe('Running'); expect(pill('timed_out')).toBe('Timed out'); expect(pill(null)).toBe('Never run'); expect(nextRun(null)).toBe('Switched off') })
 it('creates independent disabled defaults', () => { const a = newJob(), b = newJob(); a.delivery.pop(); expect(b.delivery).toHaveLength(1); expect(b.enabled).toBe(false); expect(b.action.mode).toBe('think') })
})

describe('command editor', () => {
 it('preserves arguments as text without template expansion', () => { expect(commandFields('["; calc", "{{payload.text}}", "a&calc.pdf"]', '{"MODE":"daily"}')).toEqual({ args: ['; calc', '{{payload.text}}', 'a&calc.pdf'], env: { MODE: 'daily' } }) })
 it('refuses shell strings and nontext JSON', () => { for (const args of ['python -c x', '{}', '[1]']) expect(() => commandFields(args, '{}')).toThrow(); for (const env of ['[]', 'null', '{"A":1}']) expect(() => commandFields('[]', env)).toThrow() })
 it('names the selected program', () => { expect(actionSummary({ kind: 'command', program: 'C:/Python/python.exe' })).toBe('Command: python.exe'); expect(actionSummary({ mode: 'think' })).toBe('Companion briefing') })
})

describe('Castle actions and delivery', () => {
 it('creates unacknowledged session actions', () => { expect(castleAction('save_diary_prompt').allow_tools).toBe(false); expect(actionSummary(castleAction('backup_memory'))).toBe('Back up memory locally') })
 it('updates destinations without duplicates or changing other choices', () => { const a = [{kind:'home'}]; expect(toggleDestination(toggleDestination(a, 'telegram', true), 'telegram', true)).toEqual([{kind:'home'},{kind:'telegram'}]); expect(toggleDestination(a,'home',false)).toEqual([]); expect(a).toEqual([{kind:'home'}]) })
 it('keeps action success distinct from delivery failure and uncertainty', () => { expect(pill(runStatus({status:'ok',delivered:[{status:'failed'}]}))).toBe('Delivery failed'); expect(pill(runStatus({status:'ok',delivered:[{status:'unknown'}]}))).toBe('Delivery unconfirmed'); expect(runStatus({status:'failed',delivered:[]})).toBe('failed') })
})

describe('message triggers', () => {
 it('shows matching mode and approved source', () => { expect(triggerSummary(messageTrigger())).toBe('Any approved chat: contains “invoice”'); expect(triggerSummary({platform:'whatsapp',match:'starts_with',text:'bill'})).toBe('WhatsApp: starts with “bill”'); expect(triggerSummary({platform:'discord',match:'regex',text:'^bill'})).toContain('matches regex'); })
 it('keeps chat opt-in and independent defaults', () => { const a=messageTrigger(), b=messageTrigger(); a.also_chat=true; expect(b.also_chat).toBe(false); expect(b.case).toBe(false); expect(newJob().enabled).toBe(false); })
})

describe('file and web request triggers', () => {
 it('describes what a file trigger watches and a web request trigger waits for', () => { expect(triggerSummary({ folder: 'C:/drop', glob: '*.pdf', event: 'created', settle_s: 5 }, 'file')).toBe('File appears in C:/drop (*.pdf)'); expect(triggerSummary({ folder: 'C:/drop', glob: '*', event: 'changed' })).toContain('appears or changes'); expect(triggerSummary({}, 'webhook')).toBe('Web request to its private link') })
 it('starts every new trigger kind with safe independent defaults', () => { const a = triggerDefaults('file'), b = fileTrigger(); a.glob = '*.exe'; expect(b).toEqual({ folder: '', glob: '*', event: 'created', settle_s: 5 }); expect(webhookTrigger()).toEqual({}); expect(triggerDefaults('webhook')).toEqual({}); expect(triggerDefaults('message').platform).toBe('any'); expect(triggerDefaults('schedule')).toEqual({ cron: '0 8 * * 1-5' }) })
 it('says what a switched-on job is doing, and that a switched-off one is off', () => { expect(listeningText({ triggerKind: 'file', enabled: 1 })).toBe('Watching the folder'); expect(listeningText({ triggerKind: 'webhook', enabled: 1 })).toBe('Waiting for requests'); expect(listeningText({ triggerKind: 'webhook', enabled: 0 })).toBe('Switched off'); expect(listeningText({ triggerKind: 'message', enabled: 1 })).toBe('Listening for messages'); expect(listeningText({ triggerKind: 'schedule', nextRun: '2026-12-01 08:00' })).toBe('Next: 2026-12-01 08:00') })
})

describe('event triggers', () => {
 it('lists exactly the seven events Castle can raise, by the names it uses', () => { expect(EVENTS.map(e => e.value)).toEqual(['backup_done', 'backup_failed', 'provider_switched', 'diary_saved', 'task_done', 'reminder_due', 'calendar_event']); expect(new Set(EVENTS.map(e => e.label)).size).toBe(7) })
 it('describes an event job and starts from a safe default', () => { expect(triggerSummary({ event: 'backup_failed' }, 'event')).toBe('When: Memory backup failed'); expect(triggerSummary({ event: 'task_done' })).toBe('When: Project task ticked'); expect(triggerSummary({ event: 'x' }, 'event')).toBe('When: a Castle event'); expect(triggerDefaults('event')).toEqual({ event: 'backup_failed' }); const a = eventTrigger(); a.event = 'diary_saved'; expect(eventTrigger().event).toBe('backup_failed'); expect(listeningText({ triggerKind: 'event', enabled: 1 })).toBe('Waiting for the event'); expect(listeningText({ triggerKind: 'event', enabled: 0 })).toBe('Switched off') })
})

describe('HOME card lines', () => {
 it('shows a job, how it went and its words without markdown marks', () => { expect(noticeLine({ name: 'web test', status: 'ok', summary: '**Overdue:** The Beatles   cutoff\nwas 12 Nov `x`' })).toEqual({ name: 'web test', state: 'OK', tone: 'ok', text: 'Overdue: The Beatles cutoff was 12 Nov x' }) })
 it('marks failures, timeouts and lost deliveries as bad and copes with missing fields', () => { for (const status of ['failed', 'timed_out', 'delivery_failed']) expect(noticeLine({ name: 'a', status, summary: 's' }).tone).toBe('bad'); expect(noticeLine({ status: 'delivery_unknown' }).tone).toBe('ok'); expect(noticeLine({})).toEqual({ name: 'A job', state: 'Never run', tone: 'ok', text: '' }) })
})

describe('phase7 HTTP and follow-up editor', () => {
 it('serializes JSON bodies and preserves event triggers', async () => { const {requestBody,savedTrigger}=await import('./automations.js');expect(requestBody('GET','invalid')).toBeNull();expect(requestBody('POST','{"text":"{{payload.text}}"}')).toEqual({text:'{{payload.text}}'});expect(()=>requestBody('POST','{')).toThrow('valid JSON');expect(savedTrigger('event',{event:'task_done'},'cron','')).toEqual({event:'task_done'});expect(savedTrigger('webhook',{},'cron','')).toEqual({});expect(savedTrigger('schedule',{},'every_minutes','30')).toEqual({every_minutes:30}); })
 it('names a fixed HTTP request', () => { expect(actionSummary({kind:'http',method:'POST',url:'https://example.com'})).toBe('HTTP: POST https://example.com'); })
})
