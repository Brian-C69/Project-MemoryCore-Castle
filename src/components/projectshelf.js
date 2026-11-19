// The PROJECTS room's shelf moves: from each shelf, what a project can do next. `ask` asks first
// (a pause's reason, a delete's yes); `to` is the shelf it goes to.
export const MOVES = {
  active: [
    { label: 'Pause', ask: 'pause', to: 'paused' },
    { label: 'Archive', to: 'archived' },
    { label: 'Delete…', ask: 'delete', to: 'deleted' },
  ],
  paused: [
    { label: 'Resume', to: 'active' },
    { label: 'Archive', to: 'archived' },
    { label: 'Delete…', ask: 'delete', to: 'deleted' },
  ],
  archived: [
    { label: 'Back to active', to: 'active' },
    { label: 'Delete…', ask: 'delete', to: 'deleted' },
  ],
  deleted: [{ label: 'Restore', to: 'active' }],
}

// The words on the project's chip.
export function stateLine(state) {
  switch (state?.shelf) {
    case 'paused':
      return 'Paused'
    case 'archived':
      return 'Archived'
    case 'deleted': {
      const left = state.daysLeft ?? 0
      return left > 1 ? `Deleted · gone for good in ${left} days` : left === 1 ? 'Deleted · gone for good tomorrow' : 'Deleted · goes today'
    }
    default:
      return 'Active'
  }
}
