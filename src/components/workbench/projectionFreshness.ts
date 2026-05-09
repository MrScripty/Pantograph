import type { ProjectionStateRecord } from '../../services/diagnostics/types';

export function formatProjectionFreshnessState(state: ProjectionStateRecord | null): string {
  if (!state) {
    return 'Projection unavailable';
  }
  const cursor = `seq ${state.last_applied_event_seq}`;
  switch (state.status) {
    case 'current':
      return `Current at ${cursor}`;
    case 'rebuilding':
      return `Rebuilding at ${cursor}`;
    case 'needs_rebuild':
      return `Needs rebuild at ${cursor}`;
    case 'failed':
      return state.last_error ? `Failed at ${cursor}: ${state.last_error}` : `Failed at ${cursor}`;
  }
}
