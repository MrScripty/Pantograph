import test from 'node:test';
import assert from 'node:assert/strict';

import {
  formatDiagnosticsBytes,
  formatDiagnosticsDuration,
  formatDiagnosticsPercent,
  formatTimingExpectationDetail,
  formatTimingExpectationSummary,
  formatCheckpointSummary,
  formatNodeMemoryCompatibilityLabel,
  formatNodeMemoryStatusLabel,
  formatSessionResidencyLabel,
  getGraphMemoryImpactCounts,
  getDiagnosticsStatusClasses,
  getNodeMemoryStatusCounts,
  getRuntimeInstallStateClasses,
  getRunNodeStatusCounts,
  getSchedulerStateClasses,
  getTimingExpectationClasses,
} from './presenters.ts';
import type { DiagnosticsRunTrace } from '../../services/diagnostics/types.ts';

function createRunTrace(): DiagnosticsRunTrace {
  return {
    executionId: 'exec-1',
    sessionId: 'session-1',
    workflowId: 'wf-1',
    workflowName: 'Workflow One',
    graphFingerprintAtStart: 'graph-1',
    nodeCountAtStart: 4,
    status: 'running',
    startedAtMs: 1_000,
    endedAtMs: null,
    durationMs: null,
    lastUpdatedAtMs: 1_250,
    error: null,
    waitingForInput: false,
    runtime: {
      runtimeId: 'llama.cpp',
      runtimeInstanceId: 'llama-cpp-1',
      modelTarget: '/models/demo.gguf',
      warmupStartedAtMs: 900,
      warmupCompletedAtMs: 980,
      warmupDurationMs: 80,
      runtimeReused: false,
      lifecycleDecisionReason: 'runtime_ready',
    },
    eventCount: 4,
    streamEventCount: 1,
    lastDirtyTasks: [],
    lastIncrementalTaskIds: [],
    lastGraphMemoryImpact: null,
    nodes: {
      a: {
        nodeId: 'a',
        nodeType: 'loader',
        status: 'completed',
        startedAtMs: 1_000,
        endedAtMs: 1_020,
        durationMs: 20,
        lastProgress: 1,
        lastMessage: null,
        streamEventCount: 0,
        eventCount: 2,
        error: null,
      },
      b: {
        nodeId: 'b',
        nodeType: 'llm',
        status: 'running',
        startedAtMs: 1_010,
        endedAtMs: null,
        durationMs: null,
        lastProgress: 0.6,
        lastMessage: 'Generating',
        streamEventCount: 1,
        eventCount: 3,
        error: null,
      },
      c: {
        nodeId: 'c',
        nodeType: 'approval',
        status: 'waiting',
        startedAtMs: 1_040,
        endedAtMs: null,
        durationMs: null,
        lastProgress: null,
        lastMessage: 'Awaiting input',
        streamEventCount: 0,
        eventCount: 1,
        error: null,
      },
      d: {
        nodeId: 'd',
        nodeType: 'writer',
        status: 'failed',
        startedAtMs: 1_050,
        endedAtMs: 1_070,
        durationMs: 20,
        lastProgress: null,
        lastMessage: null,
        streamEventCount: 0,
        eventCount: 1,
        error: 'disk full',
      },
    },
    events: [],
  };
}

test('formatDiagnosticsDuration reports milliseconds seconds and in-progress states', () => {
  assert.equal(formatDiagnosticsDuration(null), 'In progress');
  assert.equal(formatDiagnosticsDuration(250), '250 ms');
  assert.equal(formatDiagnosticsDuration(2_500), '2.5 s');
  assert.equal(formatDiagnosticsDuration(12_000), '12 s');
  assert.equal(formatDiagnosticsDuration(125_000), '2m 5s');
});

test('formatDiagnosticsPercent and status classes expose stable labels', () => {
  assert.equal(formatDiagnosticsPercent(null), 'No progress');
  assert.equal(formatDiagnosticsPercent(0.523), '52%');
  assert.match(getDiagnosticsStatusClasses('waiting'), /amber/);
});

test('timing expectation presenters expose duration comparison labels', () => {
  assert.equal(formatTimingExpectationSummary(null), 'No timing history');
  assert.equal(formatTimingExpectationDetail(null), 'No comparable completed runs yet');
  assert.match(getTimingExpectationClasses('slower_than_expected'), /amber/);

  assert.equal(
    formatTimingExpectationSummary({
      comparison: 'slower_than_expected',
      sampleCount: 5,
      currentDurationMs: 450,
      medianDurationMs: 220,
      typicalMinDurationMs: 200,
      typicalMaxDurationMs: 300,
    }),
    'Slower than usual',
  );
  assert.equal(
    formatTimingExpectationDetail({
      comparison: 'within_expected_range',
      sampleCount: 5,
      currentDurationMs: 220,
      medianDurationMs: 220,
      typicalMinDurationMs: 200,
      typicalMaxDurationMs: 300,
    }),
    'Typical 200 ms-300 ms | median 220 ms | n=5',
  );
  assert.equal(
    formatTimingExpectationDetail({
      comparison: 'insufficient_history',
    } as Parameters<typeof formatTimingExpectationDetail>[0]),
    'No comparable completed runs yet',
  );
});

test('byte and runtime or scheduler label helpers expose readable labels', () => {
  assert.equal(formatDiagnosticsBytes(512), '512 B');
  assert.equal(formatDiagnosticsBytes(2048), '2.0 KiB');
  assert.match(getSchedulerStateClasses('idle_loaded'), /emerald/);
  assert.match(getRuntimeInstallStateClasses('missing', false), /amber/);
  assert.match(getRuntimeInstallStateClasses('installed', true), /emerald/);
});

test('getRunNodeStatusCounts groups node states for overview summaries', () => {
  const counts = getRunNodeStatusCounts(createRunTrace());
  assert.deepEqual(counts, {
    running: 1,
    waiting: 1,
    completed: 1,
    cancelled: 0,
    failed: 1,
  });
});

test('graph memory impact helpers summarize compatibility decisions for the UI', () => {
  const counts = getGraphMemoryImpactCounts({
    fallback_to_full_invalidation: true,
    node_decisions: [
      {
        node_id: 'input',
        compatibility: 'preserve_as_is',
        reason: null,
      },
      {
        node_id: 'prompt',
        compatibility: 'preserve_with_input_refresh',
        reason: 'input_changed',
      },
      {
        node_id: 'llm',
        compatibility: 'drop_on_identity_change',
        reason: 'node_removed',
      },
      {
        node_id: 'output',
        compatibility: 'fallback_full_invalidation',
        reason: 'graph_changed',
      },
    ],
  });

  assert.deepEqual(counts, {
    preserved: 1,
    refreshed: 1,
    dropped: 1,
    fallback: 1,
  });
  assert.equal(formatNodeMemoryCompatibilityLabel('preserve_as_is'), 'Preserved');
  assert.equal(
    formatNodeMemoryCompatibilityLabel('drop_on_schema_incompatibility'),
    'Dropped Schema',
  );
});

test('session inspection helpers summarize residency, checkpoint, and node-memory status', () => {
  assert.equal(formatSessionResidencyLabel('checkpointed_but_unloaded'), 'Checkpointed');
  assert.equal(formatSessionResidencyLabel(null), 'Unavailable');
  assert.equal(formatNodeMemoryStatusLabel('invalidated'), 'Invalidated');
  assert.equal(
    formatCheckpointSummary({
      session_id: 'session-1',
      graph_revision: 'graph-1',
      residency: 'warm',
      checkpoint_available: true,
      preserved_node_count: 3,
      checkpointed_at_ms: 2_000,
    }),
    '3 preserved nodes',
  );
  assert.deepEqual(
    getNodeMemoryStatusCounts([
      {
        identity: {
          session_id: 'session-1',
          node_id: 'input',
          node_type: 'input',
          schema_version: null,
        },
        status: 'ready',
      },
      {
        identity: {
          session_id: 'session-1',
          node_id: 'llm',
          node_type: 'llm',
          schema_version: null,
        },
        status: 'invalidated',
      },
      {
        identity: {
          session_id: 'session-1',
          node_id: 'output',
          node_type: 'output',
          schema_version: null,
        },
        status: 'empty',
      },
    ]),
    {
      ready: 1,
      empty: 1,
      invalidated: 1,
    },
  );
});
