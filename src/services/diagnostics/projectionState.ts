import type { ProjectionStateRecord } from './types.ts';

export function mockProjectionState(
  overrides: Partial<ProjectionStateRecord> & Pick<ProjectionStateRecord, 'projection_name'>,
): ProjectionStateRecord {
  return {
    projection_version: 1,
    last_applied_event_seq: 0,
    status: 'current',
    rebuilt_at_ms: null,
    updated_at_ms: Date.now(),
    last_error: null,
    last_error_at_ms: null,
    last_failed_event_seq: null,
    ...overrides,
  };
}
