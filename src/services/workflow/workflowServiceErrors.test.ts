import test from 'node:test';
import assert from 'node:assert/strict';
import {
  normalizeWorkflowServiceError,
  parseWorkflowBackendErrorEnvelope,
  WorkflowServiceError,
} from './workflowServiceErrors.ts';

test('parseWorkflowBackendErrorEnvelope parses Tauri workflow error JSON', () => {
  const envelope = parseWorkflowBackendErrorEnvelope(
    JSON.stringify({
      code: 'queue_item_not_found',
      message: 'queue item missing',
      details: { queue: 'session-a' },
      diagnostics: {
        workflow_run_id: 'run-1',
        diagnostic_event_id: 'event-error-1',
      },
    }),
  );

  assert.deepEqual(envelope, {
    code: 'queue_item_not_found',
    message: 'queue item missing',
    details: { queue: 'session-a' },
    diagnostics: {
      workflow_run_id: 'run-1',
      diagnostic_event_id: 'event-error-1',
      diagnostics_unavailable: null,
    },
  });
});

test('normalizeWorkflowServiceError preserves backend code and details', () => {
  const error = normalizeWorkflowServiceError(
    JSON.stringify({
      code: 'scheduler_busy',
      message: 'runtime capacity exhausted',
      details: {
        scheduler: {
          reason: 'runtime_capacity_exhausted',
        },
      },
      diagnostics: {
        workflow_run_id: 'run-a',
        diagnostic_event_id: 'event-a',
      },
    }),
  );

  assert.ok(error instanceof WorkflowServiceError);
  assert.equal(error.code, 'scheduler_busy');
  assert.equal(error.message, 'runtime capacity exhausted');
  assert.deepEqual(error.details, {
    scheduler: {
      reason: 'runtime_capacity_exhausted',
    },
  });
  assert.deepEqual(error.diagnostics, {
    workflow_run_id: 'run-a',
    diagnostic_event_id: 'event-a',
    diagnostics_unavailable: null,
  });
});

test('normalizeWorkflowServiceError classifies non-envelope failures as transport errors', () => {
  const error = normalizeWorkflowServiceError(new Error('IPC disconnected'));

  assert.equal(error.code, 'transport_error');
  assert.equal(error.message, 'IPC disconnected');
  assert.equal(error.backendEnvelope, null);
  assert.equal(error.diagnostics, null);
});
