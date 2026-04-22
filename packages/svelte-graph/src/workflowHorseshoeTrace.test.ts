import test from 'node:test';
import assert from 'node:assert/strict';

import {
  formatWorkflowHorseshoeOpenRequestTrace,
  formatWorkflowHorseshoeSessionTrace,
  resolveWorkflowHorseshoeBlockedReasonLog,
} from './workflowHorseshoeTrace.ts';

test('formatWorkflowHorseshoeSessionTrace describes session display state and ownership flags', () => {
  assert.equal(
    formatWorkflowHorseshoeSessionTrace({
      dragActive: true,
      openRequested: true,
      displayState: 'blocked',
      blockedReason: 'candidates_pending',
      anchorPosition: { x: 10, y: 20 },
    }),
    'session:blocked:requested:candidates_pending:anchor',
  );
});

test('formatWorkflowHorseshoeSessionTrace reports clear idle sessions without anchors', () => {
  assert.equal(
    formatWorkflowHorseshoeSessionTrace({
      dragActive: false,
      openRequested: false,
      displayState: 'hidden',
      blockedReason: null,
      anchorPosition: null,
    }),
    'session:hidden:idle:clear:no-anchor',
  );
});

test('formatWorkflowHorseshoeOpenRequestTrace describes open request context', () => {
  assert.equal(
    formatWorkflowHorseshoeOpenRequestTrace({
      dragActive: true,
      connectionMode: 'connect',
      hasConnectionIntent: true,
      insertableCount: 3,
      hasAnchorPosition: true,
    }),
    'request-open:drag:connect:intent:3-insertables:anchor',
  );
});

test('formatWorkflowHorseshoeOpenRequestTrace describes idle contexts without intent', () => {
  assert.equal(
    formatWorkflowHorseshoeOpenRequestTrace({
      dragActive: false,
      connectionMode: 'idle',
      hasConnectionIntent: false,
      insertableCount: 0,
      hasAnchorPosition: false,
    }),
    'request-open:idle:idle:no-intent:0-insertables:no-anchor',
  );
});

test('resolveWorkflowHorseshoeBlockedReasonLog suppresses empty and repeated blocked reasons', () => {
  assert.deepEqual(
    resolveWorkflowHorseshoeBlockedReasonLog({
      blockedReason: null,
      lastLoggedBlockedReason: 'no_active_drag',
    }),
    {
      message: null,
      nextLoggedBlockedReason: 'no_active_drag',
      shouldLog: false,
    },
  );
  assert.deepEqual(
    resolveWorkflowHorseshoeBlockedReasonLog({
      blockedReason: 'no_active_drag',
      lastLoggedBlockedReason: 'no_active_drag',
    }),
    {
      message: null,
      nextLoggedBlockedReason: 'no_active_drag',
      shouldLog: false,
    },
  );
});

test('resolveWorkflowHorseshoeBlockedReasonLog formats new blocked reasons', () => {
  assert.deepEqual(
    resolveWorkflowHorseshoeBlockedReasonLog({
      blockedReason: 'missing_anchor_position',
      lastLoggedBlockedReason: 'no_active_drag',
    }),
    {
      message: 'cursor anchor position is unavailable',
      nextLoggedBlockedReason: 'missing_anchor_position',
      shouldLog: true,
    },
  );
});
