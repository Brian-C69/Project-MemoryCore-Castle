import { describe, expect, it } from 'vitest'
import { canDrag, clock, colourVar, emptyForm, formFromQuick, layoutDay, leadText, minutes, movedStart, jobId, jobOptions, jobValue, linkLeadOptions, proposalLine, remindMinutes, remindOptions, remindValue, repeatText, toEvent, toForm, weekOf, weekTitle } from './calendarEvents.js'

describe('weeks', () => {
  it('finds the week of a day, Monday first or Sunday first', () => {
    expect(weekOf('2026-11-22', 1).days[0]).toBe('2026-11-16')
    expect(weekOf('2026-11-22', 1).to).toBe('2026-11-22')
    expect(weekOf('2026-11-22', 0).from).toBe('2026-11-22')
    expect(weekOf('2026-11-22', 0).to).toBe('2026-11-28')
    expect(weekOf('2026-12-31', 1).to).toBe('2027-01-03')
  })
  it('titles a week, across a month end too', () => {
    expect(weekTitle(weekOf('2026-11-18', 1).days)).toBe('16 - 22 Nov 2026')
    expect(weekTitle(weekOf('2026-11-30', 1).days)).toBe('30 Nov - 6 Dec 2026')
  })
  it('reads and writes minutes', () => {
    expect(minutes('09:30')).toBe(570)
    expect(minutes('')).toBe(null)
    expect(minutes('9.30')).toBe(null)
    expect(clock(570)).toBe('09:30')
    expect(clock(-5)).toBe('00:00')
    expect(clock(99999)).toBe('23:59')
  })
})

describe('week blocks', () => {
  const ev = (id, time, endTime, extra = {}) => ({ kind: 'event', id, time, endTime, allDay: false, ...extra })
  it('sets top and height from the times and gives an hour when there is no end', () => {
    const [a, b] = layoutDay([ev(1, '09:00', '10:30'), ev(2, '13:00', null)])
    expect([a.top, a.height]).toEqual([540, 90])
    expect([b.top, b.height]).toEqual([780, 60])
  })
  it('puts events that overlap side by side and lets later ones reuse the lane', () => {
    const blocks = layoutDay([ev(1, '09:00', '10:00'), ev(2, '09:30', '10:30'), ev(3, '10:30', '11:00')])
    expect(blocks.map((b) => [b.item.id, b.lane, b.lanes])).toEqual([[1, 0, 2], [2, 1, 2], [3, 0, 1]])
  })
  it('keeps very short events tall enough to read, and ignores all-day items and other kinds', () => {
    expect(layoutDay([ev(1, '09:00', '09:05')])[0].height).toBe(30)
    expect(layoutDay([ev(1, '', null, { allDay: true }), { kind: 'task', time: '09:00' }, ev(3, null, null)])).toEqual([])
  })
})

describe('event forms', () => {
  it('writes a timed event with an end', () => {
    const f = { ...emptyForm('2026-11-27', '15:00'), title: 'Dentist', endTime: '16:00' }
    expect(toEvent(f)).toMatchObject({ title: 'Dentist', startsAt: '2026-11-27 15:00', endsAt: '2026-11-27 16:00', allDay: false, repeat: null })
  })
  it('writes an all-day event, with an end day only when it is a different day', () => {
    const f = { ...emptyForm('2026-12-24'), title: 'Trip' }
    expect(toEvent(f)).toMatchObject({ startsAt: '2026-12-24 00:00', endsAt: null, allDay: true })
    expect(toEvent({ ...f, endDate: '2026-12-26' }).endsAt).toBe('2026-12-26 00:00')
    expect(toEvent({ ...f, endDate: '2026-12-24' }).endsAt).toBe(null)
  })
  it('a timed event with no time yet starts at nine', () => {
    expect(toEvent({ ...emptyForm('2026-11-27', ''), allDay: false, title: 'x' }).startsAt).toBe('2026-11-27 09:00')
  })
  it('writes a weekly repeat with weekdays, an end day or a count', () => {
    const f = { ...emptyForm('2026-11-02', '09:00'), title: 'Gym', repeatOn: true, every: 'week', weekdays: [3, 1], endsKind: 'count', count: 8 }
    expect(toEvent(f).repeat).toEqual({ every: 'week', interval: 1, weekdays: [1, 3], until: null, count: 8 })
    expect(toEvent({ ...f, endsKind: 'until', until: '2027-01-01' }).repeat).toMatchObject({ until: '2027-01-01', count: null })
    expect(toEvent({ ...f, every: 'month' }).repeat.weekdays).toEqual([])
    expect(toEvent({ ...f, endsKind: 'never' }).repeat).toMatchObject({ until: null, count: null })
  })
  it('round trips an event through the form', () => {
    const e = { id: 7, title: 'Gym', note: 'bring towel', startsAt: '2026-11-02 18:30', endsAt: '2026-11-02 19:30', allDay: false, colour: 'soft', remindBeforeMin: 15, automationJobId: null, automationLeadMin: null, repeat: { every: 'week', interval: 2, weekdays: [1], until: '2027-03-01', count: null } }
    expect(toEvent(toForm(e))).toEqual(e)
    const day = { id: 8, title: 'Trip', note: '', startsAt: '2026-12-24 00:00', endsAt: '2026-12-26 00:00', allDay: true, colour: 'accent', remindBeforeMin: null, automationJobId: null, automationLeadMin: null, repeat: null }
    expect(toEvent(toForm(day))).toEqual(day)
  })
  it('turns a quick-add result into a form, using the shown day when no date was understood', () => {
    const q = { title: 'Gym', date: null, time: '18:30', repeat: { every: 'week', interval: 1, weekdays: [1] }, remindMin: 30 }
    const f = formFromQuick(q, '2026-11-22')
    expect([f.date, f.time, f.allDay, f.repeatOn, f.weekdays, f.remindBeforeMin]).toEqual(['2026-11-22', '18:30', false, true, [1], 30])
    expect(formFromQuick({ title: 'Milk', date: '2026-11-25', time: null }, '2026-11-22').allDay).toBe(true)
  })
  it('says what a repeat means in words', () => {
    expect(repeatText(null)).toBe('')
    expect(repeatText({ every: 'day', interval: 1 })).toBe('Repeats every day')
    expect(repeatText({ every: 'week', interval: 2, weekdays: [1, 3], count: 6 })).toBe('Repeats every 2 weeks on Mon, Wed, 6 times')
    expect(repeatText({ every: 'month', interval: 1, until: '2027-01-01' })).toBe('Repeats every month, until 2027-01-01')
  })
})

describe('dragging', () => {
  it('keeps the time of day when dropped on another day', () => {
    expect(movedStart({ allDay: false, time: '15:00' }, '2026-12-01')).toBe('2026-12-01 15:00')
    expect(movedStart({ allDay: true, time: null }, '2026-12-01')).toBe('2026-12-01 00:00')
  })
  it('takes the dropped time when dropped on an hour', () => {
    expect(movedStart({ allDay: false, time: '15:00' }, '2026-12-01', '11:15')).toBe('2026-12-01 11:15')
  })
  it('lets only the first piece of an event be dragged, and only events', () => {
    expect(canDrag({ kind: 'event', first: true })).toBe(true)
    expect(canDrag({ kind: 'event', first: false })).toBe(false)
    expect(canDrag({ kind: 'task', first: true })).toBe(false)
  })
  it('names colours by theme variable only', () => {
    expect(colourVar('accent')).toBe('var(--accent)')
    expect(colourVar('whatever')).toBe('var(--accent)')
    expect(colourVar('soft')).toContain('var(--accent)')
  })
})

describe('suggestions', () => {
  it('writes one line for a suggestion', () => {
    expect(proposalLine({ title: 'Dentist', startsAt: '2026-11-27 15:00', allDay: false, repeat: null })).toBe('Dentist - Friday 27 Nov 15:00')
    expect(proposalLine({ title: 'Trip', startsAt: '2026-12-24 00:00', allDay: true, repeat: null })).toBe('Trip - Thursday 24 Dec')
    expect(proposalLine({ title: 'Gym', startsAt: '2026-11-23 18:30', allDay: false, repeat: { every: 'week', interval: 1, weekdays: [1] } })).toBe('Gym - Monday 23 Nov 18:30 - repeats every week on mon')
  })
  it('a suggestion cannot be dragged and the form never sends its state back', () => {
    expect(canDrag({ kind: 'event', first: true, state: 'proposed' })).toBe(false)
    expect(canDrag({ kind: 'event', first: true, state: 'active' })).toBe(true)
    const form = toForm({ id: 5, title: 'x', startsAt: '2026-11-27 15:00', allDay: false, state: 'proposed', createdBy: 'companion', note: 'Why: ok' })
    expect([form.state, form.createdBy]).toEqual(['proposed', 'companion'])
    const sent = toEvent(form)
    expect('state' in sent || 'createdBy' in sent).toBe(false)
  })
})

describe('starting an automation', () => {
  const jobs = [{ id: 4, name: 'Briefing', enabled: true }, { id: 9, name: 'Digest', enabled: false }]
  it('lists the jobs an event may start, and says which are switched off', () => {
    expect(jobOptions(jobs, null)).toEqual([{ value: 'none', label: 'No automation' }, { value: '4', label: 'Briefing' }, { value: '9', label: 'Digest (switched off)' }])
  })
  it('keeps a linked job that can no longer be chosen visible', () => {
    const options = jobOptions(jobs, 77)
    expect(options.at(-1).value).toBe('77')
    expect(options.at(-1).label).toContain('no longer')
  })
  it('maps between the select and the job id', () => {
    expect([jobValue(null), jobValue(4)]).toEqual(['none', '4'])
    expect([jobId('none'), jobId('4'), jobId(null)]).toEqual([null, 4, null])
  })
  it('offers the same leads as a reminder but always one of them', () => {
    expect(linkLeadOptions(null)[0]).toEqual({ value: '0', label: 'At the time' })
    expect(linkLeadOptions(null).some((o) => o.value === 'none')).toBe(false)
    expect(linkLeadOptions(45).some((o) => o.value === '45')).toBe(true)
  })
  it('writes the link only when a job is chosen', () => {
    const f = { ...emptyForm('2026-11-27', '09:00'), title: 'Planning', automationJobId: 4, automationLeadMin: 30 }
    expect(toEvent(f)).toMatchObject({ automationJobId: 4, automationLeadMin: 30 })
    expect(toEvent({ ...f, automationJobId: null })).toMatchObject({ automationJobId: null, automationLeadMin: null })
    const round = toForm({ id: 3, title: 'Planning', startsAt: '2026-11-27 09:00', allDay: false, automationJobId: 4, automationLeadMin: 30 })
    expect([round.automationJobId, round.automationLeadMin]).toEqual([4, 30])
  })
})

describe('reminders', () => {
  it('says a lead in words', () => {
    expect([1, 30, 60, 90, 120, 1440, 2880, 10080].map(leadText)).toEqual(['1 minute', '30 minutes', '1 hour', '90 minutes', '2 hours', '1 day', '2 days', '1 week'])
  })
  it('lists the choices and keeps an odd one from quick add', () => {
    const plain = remindOptions(null)
    expect(plain[0]).toEqual({ value: 'none', label: 'No reminder' })
    expect(plain[1]).toEqual({ value: '0', label: 'At the time' })
    expect(plain.find((o) => o.value === '30').label).toBe('30 minutes before')
    expect(plain.length).toBe(11)
    const odd = remindOptions(45)
    expect(odd.length).toBe(12)
    expect(odd.find((o) => o.value === '45').label).toBe('45 minutes before')
    expect(remindOptions(30).length).toBe(11)
  })
  it('maps between the select and the minutes', () => {
    expect(remindValue(null)).toBe('none')
    expect(remindValue(0)).toBe('0')
    expect(remindMinutes('none')).toBe(null)
    expect(remindMinutes('0')).toBe(0)
    expect(remindMinutes('1440')).toBe(1440)
  })
})
