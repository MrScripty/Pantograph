import test from 'node:test';
import assert from 'node:assert/strict';

import {
  createDiagnosticsSnapshot,
  createEmptyDiagnosticsProjection,
  normalizeDiagnosticsProjection,
} from './diagnosticsProjection.ts';

test('createEmptyDiagnosticsProjection includes backend-owned projection context', () => {
  const projection = createEmptyDiagnosticsProjection();

  assert.deepEqual(projection.context, {
    requestedWorkflowRunId: null,
    requestedSessionId: null,
    requestedWorkflowId: null,
    sourceWorkflowRunId: null,
    relevantWorkflowRunId: null,
    relevant: true,
  });
  assert.equal(projection.workflowTimingHistory, null);
});

test('normalizeDiagnosticsProjection backfills legacy optional diagnostics fields', () => {
  const previous = createEmptyDiagnosticsProjection();
  previous.context = {
    requestedWorkflowRunId: null,
    requestedSessionId: 'session-1',
    requestedWorkflowId: 'workflow-1',
    sourceWorkflowRunId: null,
    relevantWorkflowRunId: 'run-1',
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
    requestedWorkflowRunId: null,
    requestedSessionId: 'session-1',
    requestedWorkflowId: 'workflow-1',
    sourceWorkflowRunId: null,
    relevantWorkflowRunId: 'run-1',
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
      requestedWorkflowRunId: null,
      requestedSessionId: 'session-2',
      requestedWorkflowId: 'workflow-2',
      sourceWorkflowRunId: 'run-2',
      relevantWorkflowRunId: 'run-1',
      relevant: false,
    },
  };

  const normalized = normalizeDiagnosticsProjection(incoming, previous);

  assert.equal(normalized.context.relevant, false);
  assert.equal(normalized.context.sourceWorkflowRunId, 'run-2');
  assert.equal(normalized.context.relevantWorkflowRunId, 'run-1');
});

test('createDiagnosticsSnapshot keeps switched workflow run labels on workflow ids', () => {
  const projection = {
    ...createEmptyDiagnosticsProjection(),
    runsById: {
      'run-b': {
        workflowRunId: 'run-b',
        sessionId: 'session-b',
        workflowId: 'workflow-b',
        graphFingerprintAtStart: 'graph-b',
        nodeCountAtStart: 1,
        status: 'completed' as const,
        startedAtMs: 1_000,
        endedAtMs: 1_500,
        durationMs: 500,
        lastUpdatedAtMs: 1_500,
        error: null,
        waitingForInput: false,
        runtime: {
          runtimeId: null,
          runtimeInstanceId: null,
          modelTarget: null,
          warmupStartedAtMs: null,
          warmupCompletedAtMs: null,
          warmupDurationMs: null,
          runtimeReused: null,
          lifecycleDecisionReason: null,
        },
        eventCount: 2,
        streamEventCount: 0,
        lastDirtyTasks: [],
        lastIncrementalTaskIds: [],
        lastGraphMemoryImpact: null,
        nodes: {},
        events: [],
      },
    },
    runOrder: ['run-b'],
  };

  const snapshot = createDiagnosticsSnapshot({
    projection,
    uiState: {
      panelOpen: true,
      activeTab: 'overview',
      selectedRunId: 'run-b',
      selectedNodeId: null,
    },
    workflowId: 'workflow-b',
    workflowGraph: null,
    sessionId: 'session-b',
  });

  assert.equal(snapshot.state.currentWorkflowId, 'workflow-b');
  assert.equal(snapshot.selectedRun?.workflowId, 'workflow-b');
  assert.equal('workflowName' in (snapshot.selectedRun ?? {}), false);
});
