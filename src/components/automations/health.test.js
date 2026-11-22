import { describe, it, expect } from 'vitest'
import { runnerStatus, heartbeatTime } from './health.js'
describe('runner health', () => {
 it('keeps missing, paused and offline states distinct', () => { expect(runnerStatus(null).label).toBe('Checking runner'); expect(runnerStatus({mode:'paused',lastHeartbeat:0},100000).label).toBe('Paused'); expect(runnerStatus({mode:'database_unavailable',lastHeartbeat:90000},100000).tone).toBe('bad'); })
 it('detects a stalled cached heartbeat and backward clock change', () => { expect(runnerStatus({mode:'running',lastHeartbeat:0},50000).label).toBe('Runner not responding'); expect(runnerStatus({mode:'running',lastHeartbeat:100000},50000).tone).toBe('bad'); })
 it('shows pending results without implying an action will repeat', () => { expect(runnerStatus({mode:'saving',lastHeartbeat:90000,pendingResults:2},100000).detail).toContain('Actions are not repeated'); expect(runnerStatus({mode:'running',lastHeartbeat:90000},100000).label).toBe('Runner active'); expect(heartbeatTime(null)).toBe('Not yet'); })
})
