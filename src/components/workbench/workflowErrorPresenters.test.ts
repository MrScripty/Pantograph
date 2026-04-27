import test from 'node:test';
import assert from 'node:assert/strict';
import { WorkflowServiceError } from '../../services/workflow/workflowServiceErrors.ts';
import { formatWorkflowCommandError } from './workflowErrorPresenters.ts';

test('formatWorkflowCommandError preserves backend error category from envelope strings', () => {
  const message = formatWorkflowCommandError(
    JSON.stringify({
      code: 'invalid_request',
      message: 'workflow_run_id must be non-empty',
    }),
  );

  assert.equal(message, 'Invalid request: workflow_run_id must be non-empty');
});

test('formatWorkflowCommandError preserves normalized scheduler details category', () => {
  const error = new WorkflowServiceError('scheduler_busy', 'runtime capacity exhausted', {
    details: {
      scheduler: {
        reason: 'runtime_capacity_exhausted',
      },
    },
  });

  assert.equal(formatWorkflowCommandError(error), 'Scheduler busy: runtime capacity exhausted');
});

test('formatWorkflowCommandError keeps non-envelope failures distinct from backend categories', () => {
  assert.equal(formatWorkflowCommandError(new Error('IPC closed')), 'Transport error: IPC closed');
});
