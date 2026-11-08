// What the WORLD hears from the desktop: whether the companion is working, resting, or was
// just written to. ChatView sets these; the 3D crystal reads them (Phase 10).
import { reactive } from 'vue'

export const worldSignals = reactive({ busy: false, asleep: true, sentAt: 0 })
