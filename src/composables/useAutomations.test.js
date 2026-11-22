import { describe, it, expect, vi } from 'vitest'
vi.mock('../api/castle.js', () => ({ automationList: vi.fn(), automationHealth: vi.fn() }))
import { automationList, automationHealth } from '../api/castle.js'
const { prepareAutomations, automations, runnerHealth } = await import('./useAutomations.js')
describe('backend health refresh', () => {
 it('retains jobs and updates health when the database list fails', async () => { automations.value = { jobs: [{id:1}], notices: [] }; automationList.mockRejectedValueOnce(new Error('Database unavailable')); automationHealth.mockResolvedValueOnce({mode:'database_unavailable',lastHeartbeat:Date.now()}); await expect(prepareAutomations()).rejects.toThrow('Database unavailable'); expect(automations.value.jobs).toEqual([{id:1}]); expect(runnerHealth.value.mode).toBe('database_unavailable'); })
 it('coalesces overlapping refreshes instead of accumulating database calls', async () => { let finish; automationList.mockImplementationOnce(() => new Promise(resolve => {finish=resolve})); automationHealth.mockResolvedValueOnce({mode:'running'}); const before=automationList.mock.calls.length; const a=prepareAutomations(),b=prepareAutomations(); expect(a).toBe(b); finish({jobs:[],notices:[]}); await a; expect(automationList.mock.calls.length).toBe(before+1); })
})
