import test from 'node:test';
import assert from 'node:assert/strict';

import {
  createEmptyDiagnosticsProjection,
  normalizeDiagnosticsProjection,
} from './diagnosticsProjection.ts';

test('createEmptyDiagnosticsProjection includes backend-owned projection context', () => {
  const projection = createEmptyDiagnosticsProjection();

  assert.deepEqual(projection.context, {
    requestedSessionId: null,
    requestedWorkflowId: null,
    requestedWorkflowName: null,
    sourceExecutionId: null,
    relevantExecutionId: null,
    relevant: true,
  });
  assert.equal(projection.workflowTimingHistory, null);
});

test('normalizeDiagnosticsProjection backfills legacy optional diagnostics fields', () => {
  const previous = createEmptyDiagnosticsProjection();
  previous.context = {
    requestedSessionId: 'session-1',
    requestedWorkflowId: 'workflow-1',
    requestedWorkflowName: 'Workflow 1',
    sourceExecutionId: null,
    relevantExecutionId: 'run-1',
    relevant: true,
  };
  previous.currentSessionState = {
    contract_version: 1,
    residency: 'active',
    node_memory: [],
    memory_impact: null,
    checkpoint: null,
  };
  previous.workflowTimingHistory = {
    workflowId: 'workflow-1',
    workflowName: 'Workflow 1',
    graphFingerprint: 'graph-1',
    timingExpectation: null,
    nodes: {},
  };
  const incoming = {
    ...createEmptyDiagnosticsProjection(),
  };
  delete (incoming as Partial<typeof incoming>).context;
  delete (incoming as Partial<typeof incoming>).currentSessionState;
  delete (incoming as Partial<typeof incoming>).workflowTimingHistory;

  const normalized = normalizeDiagnosticsProjection(incoming, previous);

  assert.deepEqual(normalized.context, {
    requestedSessionId: 'session-1',
    requestedWorkflowId: 'workflow-1',
    requestedWorkflowName: 'Workflow 1',
    sourceExecutionId: null,
    relevantExecutionId: 'run-1',
    relevant: true,
  });
  assert.equal(normalized.currentSessionState, previous.currentSessionState);
  assert.equal(normalized.workflowTimingHistory, previous.workflowTimingHistory);
});

test('normalizeDiagnosticsProjection preserves backend relevance decisions', () => {
  const previous = createEmptyDiagnosticsProjection();
  const incoming = {
    ...createEmptyDiagnosticsProjection(),
    context: {
      requestedSessionId: 'session-2',
      requestedWorkflowId: 'workflow-2',
      requestedWorkflowName: 'Workflow 2',
      sourceExecutionId: 'run-2',
      relevantExecutionId: 'run-1',
      relevant: false,
    },
  };

  const normalized = normalizeDiagnosticsProjection(incoming, previous);

  assert.equal(normalized.context.relevant, false);
  assert.equal(normalized.context.sourceExecutionId, 'run-2');
  assert.equal(normalized.context.relevantExecutionId, 'run-1');
});
